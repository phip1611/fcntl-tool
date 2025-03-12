/* SPDX-License-Identifier: MIT OR Apache-2.0 */
use crate::cli;
use anyhow::anyhow;
use nix::errno::Errno;
use nix::fcntl::{FcntlArg, fcntl};
use nix::libc;
use std::fmt::{Debug, Display, Formatter};
use std::fs::File;
use std::io;
use std::os::fd::AsRawFd;

#[derive(Clone, Copy, Debug)]
pub enum LockType {
    Write,
    Read,
}

impl LockType {
    pub const fn to_libc_val(self) -> libc::c_int {
        match self {
            Self::Write => libc::F_WRLCK,
            Self::Read => libc::F_RDLCK,
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
        match value {
            libc::F_UNLCK => Ok(Self::Unlocked),
            libc::F_WRLCK => Ok(Self::ExclusiveWrite),
            libc::F_RDLCK => Ok(Self::SharedRead),
            _ => Err(anyhow!("Invalid lock type {value}")),
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

/// Returns a [`struct@libc::flock`] structure for the whole file.
const fn get_flock(lock_type: LockType) -> libc::flock {
    libc::flock {
        l_type: lock_type.to_libc_val() as libc::c_short,
        l_whence: libc::SEEK_SET as libc::c_short,
        l_start: 0,
        l_len: 0, /* EOF */
        l_pid: 0, /* filled by callee */
    }
}

/// Describes the lock operation/strategy.
#[derive(Copy, Clone, Debug, Default)]
pub enum LockOperation {
    /// Traditional POSIX fcntl locks.
    Traditional,
    /// Open File Description (OFD) locks available, which are available since
    /// Linux 3.15.
    #[default]
    OpenFileDescription,
}

impl TryFrom<&cli::Command> for LockOperation {
    type Error = anyhow::Error;

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
            _ => Err(anyhow!("Can't create a `LockOperation` from {value:?}")),
        }
    }
}

#[derive(Copy, Clone, Debug)]
struct SetLockOperation(LockOperation);

impl SetLockOperation {
    fn to_fcntl_arg(self, flock: &libc::flock) -> FcntlArg {
        match self.0 {
            LockOperation::Traditional => FcntlArg::F_SETLK(flock),
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
    fn to_fcntl_arg(self, flock: &mut libc::flock) -> FcntlArg {
        match self.0 {
            LockOperation::Traditional => FcntlArg::F_GETLK(flock),
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
// /// parameters.
///
/// # Parameters
/// - `file`: The file to acquire a lock for [`LockType`]
/// - `lock_type`: The [`LockType`]
/// - `operation`: The [`LockOperation`]
pub fn try_acquire_lock(
    file: &mut File,
    lock_type: LockType,
    operation: LockOperation,
) -> anyhow::Result<()> {
    // Ensure that clippy understands we want a mutable binding.
    // We mark the binding as mutable as meta state for that file will be
    // altered in the callee (the kernel).
    let file: &mut File = file;
    let operation = SetLockOperation::from(operation);
    let flock = get_flock(lock_type);
    let arg = operation.to_fcntl_arg(&flock);

    let res = fcntl(file.as_raw_fd(), arg);
    match res {
        Ok(_) => Ok(()),
        // See man page for error code:
        // <https://man7.org/linux/man-pages/man2/fcntl.2.html>
        Err(Errno::EAGAIN | Errno::EACCES) => Err(FileAlreadyLockedError.into()),
        Err(_) => Err(anyhow!("Trying to get {lock_type:?} lock")),
    }
}

/// Returns the current lock state using [`fcntl`] with respect to the given
/// parameters.
///
/// # Parameters
/// - `file`: The file to acquire a lock for [`LockType`]
/// - `operation`: The [`LockOperation`]
pub fn get_lock_state(file: &File, operation: LockOperation) -> anyhow::Result<LockState> {
    let operation = GetLockOperation::from(operation);
    let mut flock = get_flock(LockType::Write);
    let arg = operation.to_fcntl_arg(&mut flock);
    let ret = fcntl(file.as_raw_fd(), arg)?;
    if ret != 0 {
        Err(io::Error::last_os_error().into())
    } else {
        let state = flock.l_type as libc::c_int;
        let state = LockState::try_from(state)?;
        Ok(state)
    }
}
