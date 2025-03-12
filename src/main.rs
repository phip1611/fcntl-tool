/* SPDX-License-Identifier: MIT OR Apache-2.0 */

//! Your small yet useful swiss army knife for the `fcntl` system call,
//! specifically for acquiring and testing file locks, but not limited to those.
//!
//! This tool only works on UNIX or POSIX-like systems. Please find more
//! information in the following resources:
//!
//! - <https://man7.org/linux/man-pages/man2/fcntl.2.html>
//! - <https://apenwarr.ca/log/20101213>
//!
//! ## Supported Platforms
//!
//! This crate works on all platforms that Rust code can be compiled to. It,
//! however, only makes sense on UNIX or POSIX-like systems.
//!
//! ## Supported `fcntl` Operations
//!
//! | operation     | supported |
//! |---------------|-----------|
//! | `F_GETLK`     | ✅         |
//! | `F_SETLK`     | ✅         |
//! | `F_OFD_GETLK` | ✅         |
//! | `F_OFD_SETLK` | ✅         |
//! | ...           | Not yet   |
//!
//!
//! ## CLI Usage
//!
//! Type `fcntl-tool --help` to get guidance.


#![deny(
    clippy::all,
    clippy::cargo,
    clippy::nursery,
    clippy::must_use_candidate,
    // clippy::restriction,
    // clippy::pedantic
)]
// now allow a few rules which are denied by the above statement
// --> they are ridiculous and not necessary
#![allow(
    clippy::suboptimal_flops,
    clippy::redundant_pub_crate,
    clippy::fallible_impl_from
)]
#![allow(clippy::multiple_crate_versions)]
#![deny(missing_docs)]
#![deny(missing_debug_implementations)]
#![deny(rustdoc::all)]

use clap::Parser;
use std::fs::{File, OpenOptions};
use std::io::BufRead;
use std::path::Path;

mod cli;
mod fcntl;

fn wait_for_enter() {
    println!("Waiting for enter to release lock ...");
    let mut buf = String::new();
    // blocking waits for enter (newline)
    let _ = std::io::stdin().lock().read_line(&mut buf);
}

fn open_file(path: &Path, write: bool) -> anyhow::Result<File> {
    OpenOptions::new()
        .create(false)
        .read(true)
        .write(write)
        .create(false)
        .open(path)
        .map_err(|e| e.into())
}

fn main() -> anyhow::Result<()> {
    let cli: cli::Cli = cli::Cli::parse();

    match &cli.command {
        cmd @ cli::Command::WriteLock { file, .. } => {
            let mut file = open_file(file, true)?;
            let operation = fcntl::LockOperation::try_from(cmd)?;
            fcntl::try_acquire_lock(&mut file, fcntl::LockType::Write, operation)?;
            wait_for_enter();
        }
        cmd @ cli::Command::ReadLock { file, .. } => {
            let mut file = open_file(file, false)?;
            let operation = fcntl::LockOperation::try_from(cmd)?;
            fcntl::try_acquire_lock(&mut file, fcntl::LockType::Read, operation)?;
            wait_for_enter();
        }
        cmd @ cli::Command::TestLock { file, .. } => {
            let mut file = open_file(file, false)?;
            let operation = fcntl::LockOperation::try_from(cmd)?;
            let state = fcntl::get_lock_state(&mut file, operation)?;
            println!("state: {:?}", state);
        }
    }
    Ok(())
}
