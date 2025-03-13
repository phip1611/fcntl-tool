/* SPDX-License-Identifier: MIT OR Apache-2.0 */

use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// Your small yet useful swiss army knife for the `fcntl` system call,
/// specifically for acquiring and testing file locks, but not limited to those.
#[derive(Parser)]
#[command(version, about, long_about)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
#[allow(clippy::enum_variant_names)]
#[non_exhaustive]
pub enum Command {
    /// Acquire a write (exclusive) lock on the given file.
    #[command(name = "write-lock")]
    WriteLock {
        /// Path to file.
        #[arg(short = 'f', long = "file")]
        file: PathBuf,
        /// Whether no Open File Description (OFD) locks should be used, but
        /// the legacy POSIX ones.
        #[cfg(any(target_os = "android", target_os = "linux"))]
        #[arg(long = "legacy")]
        dont_use_ofd: bool,
    },
    /// Acquire a read (non-exclusive) lock on the given file.
    #[command(name = "read-lock")]
    ReadLock {
        /// Path to file.
        #[arg(short = 'f', long = "file")]
        file: PathBuf,
        /// Whether no Open File Description (OFD) locks should be used, but
        /// the legacy POSIX ones.
        #[cfg(any(target_os = "android", target_os = "linux"))]
        #[arg(long = "legacy")]
        dont_use_ofd: bool,
    },
    /// Test if there is currently a lock on the given file.
    #[command(name = "test-lock")]
    TestLock {
        /// Path to file.
        #[arg(short = 'f', long = "file")]
        file: PathBuf,
        /// Whether no Open File Description (OFD) locks should be used, but
        /// the legacy POSIX ones.
        #[cfg(any(target_os = "android", target_os = "linux"))]
        #[arg(long = "legacy")]
        dont_use_ofd: bool,
    },
}
