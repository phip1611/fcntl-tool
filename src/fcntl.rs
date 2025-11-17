/* SPDX-License-Identifier: MIT OR Apache-2.0 */
use crate::cli;
use crate::cli::LockScope;
use anyhow::anyhow;
use nix::errno::Errno;
use nix::fcntl::{fcntl, FcntlArg};
use nix::libc;
use nix::libc::off_t;
use std::fmt::{Debug, Display, Formatter};
use std::fs::File;
use std::io;

#[derive(Clone, Copy, Debug)]
pub enum LockType {
    Write,
    Read,
}

impl LockType {
    pub const fn to_libc_val(self) -> libc::c_int {
        match self {
            Self::Write => libc::F_WRLCK as libc::c_int,
            Self::Read => libc::F_RDLCK as libc::c_int,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum LockState {
    ExclusiveWrite,
    SharedRead,
    Unlocked,
}

impl TryFrom<libc::c_int> for LockState {
    type Error = anyhow::Error;

    fn try_from(value: libc::c_int) -> Result<Self, Self::Error> {
        const F_UNLCK: libc::c_int = libc::F_UNLCK as libc::c_int;
        const F_WRLCK: libc::c_int = libc::F_WRLCK as libc::c_int;
        const F_RDLCK: libc::c_int = libc::F_RDLCK as libc::c_int;
        match value {
            F_UNLCK => Ok(Self::Unlocked),
            F_WRLCK => Ok(Self::ExclusiveWrite),
            F_RDLCK => Ok(Self::SharedRead),
            _ => Err(anyhow!("invalid lock type {value}")),
        }
    }
}

impl Display for LockState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ExclusiveWrite => f.write_str("Exclusive Write Lock"),
            Self::SharedRead => f.write_str("Shared Read Lock"),
            Self::Unlocked => f.write_str("Unlocked"),
        }
    }
}

fn get_flock_len(scope: &LockScope, file: &File) -> anyhow::Result<off_t> {
    match scope {
        LockScope::WholeFile => Ok(0 /* EOF */),
        LockScope::WholeByteRange => {
            let len = file
                .metadata()
                .map(|m| m.len())
                .map_err(|e| anyhow::Error::new(e))?;
            off_t::try_from(len).map_err(|e| anyhow::Error::new(e))
        }
    }
}

/// Returns a [`struct@libc::flock`] structure for the whole file.
const fn get_flock(lock_type: LockType, len: off_t) -> libc::flock {
    libc::flock {
        l_type: lock_type.to_libc_val() as libc::c_short,
        l_whence: libc::SEEK_SET as libc::c_short,
        l_start: 0,
        l_len: len,
        l_pid: 0, /* filled by callee */
    }
}

/// Describes the lock operation/strategy.
#[derive(Copy, Clone, Debug)]
pub enum LockOperation {
    /// Traditional POSIX fcntl locks.
    Traditional,
    /// Open File Description (OFD) locks available, which are available since
    /// Linux 3.15.
    #[cfg(any(target_os = "android", target_os = "linux"))]
    OpenFileDescription,
}

impl TryFrom<&cli::Command> for LockOperation {
    type Error = anyhow::Error;

    #[cfg(not(any(target_os = "android", target_os = "linux")))]
    fn try_from(value: &cli::Command) -> Result<Self, Self::Error> {
        #[allow(unreachable_patterns)]
        match value {
            cli::Command::WriteLock { .. }
            | cli::Command::ReadLock { .. }
            | cli::Command::TestLock { .. } => Ok(Self::Traditional),
            _ => Err(anyhow!("Can't create a `LockOperation` from {value:?}")),
        }
    }

    #[cfg(any(target_os = "android", target_os = "linux"))]
    fn try_from(value: &cli::Command) -> Result<Self, Self::Error> {
        #[allow(unreachable_patterns)]
        match value {
            cli::Command::WriteLock { dont_use_ofd, .. } => {
                if *dont_use_ofd {
                    Ok(Self::Traditional)
                } else {
                    Ok(Self::OpenFileDescription)
                }
            }
            cli::Command::ReadLock { dont_use_ofd, .. } => {
                if *dont_use_ofd {
                    Ok(Self::Traditional)
                } else {
                    Ok(Self::OpenFileDescription)
                }
            }
            cli::Command::TestLock { dont_use_ofd, .. } => {
                if *dont_use_ofd {
                    Ok(Self::Traditional)
                } else {
                    Ok(Self::OpenFileDescription)
                }
            }
            _ => Err(anyhow!("can't create a `LockOperation` from {value:?}")),
        }
    }
}

#[derive(Copy, Clone, Debug)]
struct SetLockOperation(LockOperation);

impl SetLockOperation {
    // allow: To keep 1.74.1 as MSRV
    #[allow(clippy::missing_const_for_fn)]
    fn to_fcntl_arg<'a>(self, flock: &'a libc::flock) -> FcntlArg<'a> {
        match self.0 {
            LockOperation::Traditional => FcntlArg::F_SETLK(flock),
            #[cfg(any(target_os = "android", target_os = "linux"))]
            LockOperation::OpenFileDescription => FcntlArg::F_OFD_SETLK(flock),
        }
    }
}

impl From<LockOperation> for SetLockOperation {
    fn from(value: LockOperation) -> Self {
        Self(value)
    }
}

#[derive(Copy, Clone, Debug)]
struct GetLockOperation(LockOperation);

impl GetLockOperation {
    fn to_fcntl_arg<'a>(self, flock: &'a mut libc::flock) -> FcntlArg<'a> {
        match self.0 {
            LockOperation::Traditional => FcntlArg::F_GETLK(flock),
            #[cfg(any(target_os = "android", target_os = "linux"))]
            LockOperation::OpenFileDescription => FcntlArg::F_OFD_GETLK(flock),
        }
    }
}

impl From<LockOperation> for GetLockOperation {
    fn from(value: LockOperation) -> Self {
        Self(value)
    }
}

#[derive(Copy, Clone, Debug)]
pub struct FileAlreadyLockedError;

impl Display for FileAlreadyLockedError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("The file is already locked")
    }
}

impl std::error::Error for FileAlreadyLockedError {}

/// Tries to acquire a lock using [`fcntl`] with respect to the given
/// parameters.
///
/// Please note that `fcntl()` locks are **advisory locks**, which do not
/// prevent to `open()` a file if a lock is already placed.
///
/// # Parameters
/// - `file`: The file to acquire a lock for [`LockType`]
/// - `lock_type`: The [`LockType`]
/// - `operation`: The [`LockOperation`]
/// - `scope`: The [`LockScope`]
pub fn try_acquire_lock(
    file: &mut File,
    lock_type: LockType,
    operation: LockOperation,
    scope: &LockScope,
) -> anyhow::Result<()> {
    // Ensure that clippy understands we want a mutable binding.
    // We mark the binding as mutable as meta state for that file will be
    // altered in the callee (the kernel).
    let file: &mut File = file;
    let operation = SetLockOperation::from(operation);
    let flock_len = get_flock_len(scope, file)?;
    let flock = get_flock(lock_type, flock_len);
    let arg = operation.to_fcntl_arg(&flock);

    let res = fcntl(file, arg);
    match res {
        Ok(_) => Ok(()),
        // See man page for error code:
        // <https://man7.org/linux/man-pages/man2/fcntl.2.html>
        Err(Errno::EAGAIN | Errno::EACCES) => Err(FileAlreadyLockedError.into()),
        Err(e) => Err(anyhow!("error trying to get {lock_type:?} lock {e:?}")),
    }
}

/// Returns the current lock state using [`fcntl`] with respect to the given
/// parameters.
///
/// # Parameters
/// - `file`: The file to acquire a lock for [`LockType`]
/// - `operation`: The [`LockOperation`]
/// - `scope`: The [`LockScope`]
pub fn get_lock_state(
    file: &File,
    operation: LockOperation,
    scope: &LockScope,
) -> anyhow::Result<LockState> {
    let operation = GetLockOperation::from(operation);
    let flock_len = get_flock_len(scope, file)?;
    let mut flock = get_flock(LockType::Write, flock_len);
    let arg = operation.to_fcntl_arg(&mut flock);
    let ret = fcntl(file, arg)?;
    if ret != 0 {
        Err(io::Error::last_os_error().into())
    } else {
        let state = flock.l_type as libc::c_int;
        let state = LockState::try_from(state)?;
        Ok(state)
    }
}
