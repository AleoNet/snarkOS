// Copyright (C) 2019-2022 Aleo Systems Inc.
// This file is part of the snarkOS library.

// The snarkOS library is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// The snarkOS library is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with the snarkOS library. If not, see <https://www.gnu.org/licenses/>.

mod bech32m;
pub use bech32m::*;

mod log_writer;
use log_writer::*;

pub mod logger;
pub use logger::*;

pub mod updater;
pub use updater::*;

#[cfg(target_family = "unix")]
use colored::*;
#[cfg(target_family = "unix")]
use nix::sys::resource::{getrlimit, Resource};

/// Check if process's open files limit is above minimum and warn if not.
#[cfg(target_family = "unix")]
pub fn check_open_files_limit(minimum: u64) {
    // Acquire current limits.
    match getrlimit(Resource::RLIMIT_NOFILE) {
        Ok((soft_limit, _)) => {
            // Check if requirements are met.
            if soft_limit < minimum {
                // Warn about too low limit.
                let warning = [
                    format!("⚠️  Current open files limit ({soft_limit}) for this process is lower than recommended."),
                    format!("⚠️  Please raise it to at least {minimum} to ensure correct behavior of the node."),
                    "⚠️  See `ulimit` command and `/etc/security/limits.conf` for more details.".to_owned(),
                ]
                .join("\n")
                .yellow()
                .bold();
                eprintln!("\n{warning}\n");
            }
        }
        Err(err) => {
            // Warn about unknown limit.
            let warning = [
                format!("⚠️  Couldn't check process's open files limit due to {err}."),
                format!("⚠️  Please make sure it's at least {minimum} to ensure correct behavior of the node."),
                "⚠️  See `ulimit` command and `/etc/security/limits.conf` for more details.".to_owned(),
            ]
            .join("\n")
            .yellow()
            .bold();
            eprintln!("\n{warning}\n");
        }
    };
}
