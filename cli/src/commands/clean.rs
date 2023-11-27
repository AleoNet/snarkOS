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

use anyhow::{bail, Result};
use clap::Parser;
use colored::Colorize;
use std::path::PathBuf;

/// Cleans the snarkOS node storage.
#[derive(Debug, Parser)]
pub struct Clean {
    /// Specify the network to remove from storage.
    #[clap(default_value = "3", long = "network")]
    pub network: u16,
    /// Specify the path to a directory containing the ledger
    #[clap(long = "storage")]
    pub storage: Option<PathBuf>,
    /// Enables development mode, specify the unique ID of the local node to clean.
    #[clap(long)]
    pub dev: Option<u16>,
}

impl Clean {
    /// Cleans the snarkOS node storage.
    pub fn parse(self) -> Result<String> {
        // Remove the specified ledger from storage.
        Self::remove_ledger(self.network, self.storage.clone(), self.dev)
    }

    /// Removes the specified ledger from storage.
    pub(crate) fn remove_ledger(network: u16, path: Option<PathBuf>, dev: Option<u16>) -> Result<String> {
        // Construct the path to the ledger in storage.
        let path = aleo_std::aleo_ledger_dir(network, path, dev);

        // Prepare the path string.
        let path_string = format!("(in \"{}\")", path.display()).dimmed();

        // Check if the path to the ledger exists in storage.
        if path.exists() {
            // Remove the ledger files from storage.
            match std::fs::remove_dir_all(&path) {
                Ok(_) => Ok(format!("✅ Cleaned the snarkOS node storage {path_string}")),
                Err(error) => {
                    bail!("Failed to remove the snarkOS node storage {path_string}\n{}", error.to_string().dimmed())
                }
            }
        } else {
            Ok(format!("✅ No snarkOS node storage was found {path_string}"))
        }
    }
}
