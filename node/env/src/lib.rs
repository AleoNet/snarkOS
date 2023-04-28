// Copyright (C) 2019-2023 Aleo Systems Inc.
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

use std::{env, process::Command};

use once_cell::sync::OnceCell;
use serde::Serialize;

// Contains the environment information.
pub static ENV_INFO: OnceCell<EnvInfo> = OnceCell::new();

// Environment information.
#[allow(dead_code)]
#[derive(Debug, Serialize)]
pub struct EnvInfo {
    package: String,
    host: String,
    rustc: String,
    args: Vec<String>,
    repo: String,
    branch: String,
    commit: String,
}

impl EnvInfo {
    pub fn register() {
        // A helper function to extract command output.
        fn command(args: &[&str]) -> String {
            let mut output = String::from_utf8(
                Command::new(args[0]).args(&args[1..]).output().map(|out| out.stdout).unwrap_or_default(),
            )
            .unwrap_or_default();
            output.pop(); // Strip the trailing newline.

            output
        }

        // Process the rustc version information.
        let rustc_info = command(&["rustc", "--version", "--verbose"]);
        let rustc_info = rustc_info.split('\n').map(|line| line.split(": "));
        let mut rustc = String::new();
        let mut host = String::new();
        for mut pair in rustc_info {
            let key = pair.next();
            let value = pair.next();

            match (key, value) {
                (Some(key), None) => {
                    if key.starts_with("rustc ") {
                        rustc = key.trim_start_matches("rustc ").to_owned();
                    }
                }
                (Some(key), Some(value)) => {
                    if key == "host" {
                        host = value.to_string();
                    }
                }
                _ => {}
            }
        }

        // Process the runtime arguments, omitting any private keys in the process.
        let args = env::args().filter(|arg| !arg.starts_with("APrivateKey")).collect::<Vec<_>>();

        // Collect the information.
        let env_info = EnvInfo {
            package: env::var("CARGO_PKG_VERSION").unwrap_or_default(),
            host,
            rustc,
            args,
            repo: env::var("CARGO_PKG_REPOSITORY").unwrap_or_default(),
            branch: command(&["git", "branch", "--show-current"]),
            commit: command(&["git", "rev-parse", "HEAD"]),
        };

        // Set the static containing the information.
        ENV_INFO.set(env_info).unwrap();
    }
}
