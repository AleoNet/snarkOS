// Copyright (C) 2019-2023 Aleo Systems Inc.
// This file is part of the snarkOS library.

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at:
// http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

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
