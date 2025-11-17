/* SPDX-License-Identifier: MIT OR Apache-2.0 */
use clap::{Parser, Subcommand, ValueEnum};
use std::fmt::{Display, Formatter};
use std::path::PathBuf;

/// Your small yet useful swiss army knife for the `fcntl` system call,
/// specifically for acquiring and testing file locks, but not limited to those.
#[derive(Parser)]
#[command(version, about, long_about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

/// The scope of a file lock.
#[derive(Clone, Debug, Default, ValueEnum)]
pub enum LockScope {
    /// Lock the whole file.
    #[default]
    WholeFile,
    /// Lock the whole byte range the file occupies.
    ///
    /// This lock means that if the file's size is for example increased, the
    /// lock will not be valid for the new byte range.
    WholeByteRange,
}

impl Display for LockScope {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            // must match the way clap accepts arguments
            LockScope::WholeFile => write!(f, "whole-file"),
            LockScope::WholeByteRange => write!(f, "whole-byte-range"),
        }
    }
}

#[derive(Debug, Subcommand)]
#[allow(clippy::enum_variant_names)]
#[non_exhaustive]
pub enum Command {
    /// Acquire a write (exclusive) lock on the given file.
    #[command(name = "write-lock")]
    WriteLock {
        /// Path to file.
        #[arg()] // positional arg
        file: PathBuf,
        /// Whether legacy POSIX locks should be used instead of Open File
        /// Description (OFD) locks.
        #[cfg(any(target_os = "android", target_os = "linux"))]
        #[arg(long = "legacy")]
        dont_use_ofd: bool,
        /// The scope of the file lock.
        #[arg(long = "scope", default_value_t = LockScope::default())]
        scope: LockScope,
    },
    /// Acquire a read (non-exclusive) lock on the given file.
    #[command(name = "read-lock")]
    ReadLock {
        /// Path to file.
        #[arg()] // positional arg
        file: PathBuf,
        /// Whether legacy POSIX locks should be used instead of Open File
        /// Description (OFD) locks.
        #[cfg(any(target_os = "android", target_os = "linux"))]
        #[arg(long = "legacy")]
        dont_use_ofd: bool,
        /// The scope of the file lock.
        #[arg(long = "scope", default_value_t = LockScope::default())]
        scope: LockScope,
    },
    /// Test if there is currently a lock on the given file.
    #[command(name = "test-lock")]
    TestLock {
        /// Path to file.
        #[arg()] // positional arg
        file: PathBuf,
        /// Whether legacy POSIX locks should be used instead of Open File
        /// Description (OFD) locks.
        #[cfg(any(target_os = "android", target_os = "linux"))]
        #[arg(long = "legacy")]
        dont_use_ofd: bool,
        /// The scope of the file lock.
        #[arg(long = "scope", default_value_t = LockScope::default())]
        scope: LockScope,
    },
}
