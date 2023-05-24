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
