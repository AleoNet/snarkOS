// Copyright 2024 Aleo Network Foundation
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

use snarkos_node::bft::helpers::proposal_cache_path;

use aleo_std::StorageMode;
use anyhow::{bail, Result};
use clap::Parser;
use colored::Colorize;
use std::path::PathBuf;

/// Cleans the snarkOS node storage.
#[derive(Debug, Parser)]
pub struct Clean {
    /// Specify the network to remove from storage.
    #[clap(default_value = "0", long = "network")]
    pub network: u16,
    /// Enables development mode, specify the unique ID of the local node to clean.
    #[clap(long)]
    pub dev: Option<u16>,
    /// Specify the path to a directory containing the ledger
    #[clap(long = "path")]
    pub path: Option<PathBuf>,
}

impl Clean {
    /// Cleans the snarkOS node storage.
    pub fn parse(self) -> Result<String> {
        // Remove the current proposal cache file, if it exists.
        let proposal_cache_path = proposal_cache_path(self.network, self.dev);
        if proposal_cache_path.exists() {
            if let Err(err) = std::fs::remove_file(&proposal_cache_path) {
                bail!("Failed to remove the current proposal cache file at {}: {err}", proposal_cache_path.display());
            }
        }
        // Remove the specified ledger from storage.
        Self::remove_ledger(self.network, match self.path {
            Some(path) => StorageMode::Custom(path),
            None => StorageMode::from(self.dev),
        })
    }

    /// Removes the specified ledger from storage.
    pub(crate) fn remove_ledger(network: u16, mode: StorageMode) -> Result<String> {
        // Construct the path to the ledger in storage.
        let path = aleo_std::aleo_ledger_dir(network, mode);

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
