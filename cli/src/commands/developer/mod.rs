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

mod decrypt;
pub use decrypt::*;

mod deploy;
pub use deploy::*;

mod execute;
pub use execute::*;

mod scan;
pub use scan::*;

mod transfer;
pub use transfer::*;

use snarkvm::{
    file::{AleoFile, Manifest},
    package::Package,
    prelude::{Program, ProgramID, ToBytes, Transaction},
};

use anyhow::{bail, ensure, Result};
use clap::Parser;
use colored::Colorize;
use std::{path::PathBuf, str::FromStr};

type CurrentNetwork = snarkvm::prelude::Testnet3;

/// Commands to manage Aleo accounts.
#[derive(Debug, Parser)]
pub enum Developer {
    /// Decrypt a ciphertext.
    Decrypt(Decrypt),
    /// Deploy a program.
    Deploy(Deploy),
    /// Execute a program function.
    Execute(Execute),
    /// Scan the node for records.
    Scan(Scan),
    /// Transfer credits.
    Transfer(Transfer),
}

impl Developer {
    pub fn parse(self) -> Result<String> {
        match self {
            Self::Decrypt(decrypt) => decrypt.parse(),
            Self::Deploy(deploy) => deploy.parse(),
            Self::Execute(execute) => execute.parse(),
            Self::Scan(scan) => scan.parse(),
            Self::Transfer(transfer) => transfer.parse(),
        }
    }

    /// Parse the program from the directory.
    fn parse_program(program_id: ProgramID<CurrentNetwork>, path: Option<String>) -> Result<Program<CurrentNetwork>> {
        // Instantiate a path to the directory containing the manifest file.
        let directory = match path {
            Some(path) => PathBuf::from_str(&path)?,
            None => std::env::current_dir()?,
        };

        // Ensure the directory path exists.
        ensure!(directory.exists(), "The program directory does not exist: {}", directory.display());
        // Ensure the manifest file exists.
        ensure!(
            Manifest::<CurrentNetwork>::exists_at(&directory),
            "Please ensure that the manifest file exists in the Aleo program directory (missing '{}' at '{}')",
            Manifest::<CurrentNetwork>::file_name(),
            directory.display()
        );

        // Open the manifest file.
        let manifest = Manifest::<CurrentNetwork>::open(&directory)?;
        ensure!(
            manifest.program_id() == &program_id,
            "The program name in the manifest file does not match the specified program name"
        );

        // Load the package.
        let package = Package::open(&directory)?;
        // Load the main program.
        let program = package.program();
        // Prepare the imports directory.
        let imports_directory = package.imports_directory();

        // TODO (raychu86): Handle additional checks in consensus.
        // Find the program that is being deployed.
        let program = match program.imports().keys().find(|id| **id == program_id) {
            Some(program_id) => {
                let file = AleoFile::open(&imports_directory, program_id, false)?;
                file.program().clone()
            }
            None => match program_id == *program.id() {
                true => program.clone(),
                false => bail!("The program '{}' does not exist in {}", program_id, directory.display()),
            },
        };

        Ok(program)
    }

    /// Determine if the transaction should be broadcast or displayed to user.
    fn handle_transaction(
        broadcast: Option<String>,
        display: bool,
        store: Option<String>,
        transaction: Transaction<CurrentNetwork>,
        operation: String,
    ) -> Result<String> {
        // Get the transaction id.
        let transaction_id = transaction.id();

        // Determine if the transaction should be stored.
        if let Some(path) = store {
            match PathBuf::from_str(&path) {
                Ok(file_path) => {
                    let transaction_bytes = transaction.to_bytes_le()?;
                    std::fs::write(&file_path, transaction_bytes)?;
                    println!("Transaction {transaction_id} was stored to {}", file_path.display());
                }
                Err(err) => {
                    println!("The transaction was unable to be stored due to: {err}");
                }
            }
        };

        // Determine if the transaction should be broadcast or displayed to user.
        if let Some(endpoint) = broadcast {
            // Send the deployment request to the local development node.
            match ureq::post(&endpoint).send_json(&transaction) {
                Ok(id) => {
                    ensure!(
                        id.into_string()? == transaction_id.to_string(),
                        "The response does not match the transaction id"
                    );

                    match transaction {
                        Transaction::Deploy(..) => {
                            println!("✅ Successfully deployed '{}' to {}.", operation.bold(), endpoint)
                        }
                        Transaction::Execute(..) => {
                            println!("✅ Successfully broadcast execution '{}' to the {}.", operation.bold(), endpoint)
                        }
                    }
                }
                Err(error) => {
                    let error_message = match error {
                        ureq::Error::Status(code, response) => {
                            format!("(status code {code}: {:?})", response.into_string()?)
                        }
                        ureq::Error::Transport(err) => format!("({err})"),
                    };

                    match transaction {
                        Transaction::Deploy(..) => {
                            bail!("❌ Failed to deploy '{}' to {}: {}", operation.bold(), &endpoint, error_message)
                        }
                        Transaction::Execute(..) => {
                            bail!(
                                "❌ Failed to broadcast execution '{}' to {}: {}",
                                operation.bold(),
                                &endpoint,
                                error_message
                            )
                        }
                    }
                }
            };

            // Output the transaction id.
            Ok(transaction_id.to_string())
        } else if display {
            // Output the transaction string.
            Ok(transaction.to_string())
        } else {
            // TODO (raychu86): Handle the case where the user does not specify a broadcast or display flag.
            Ok("".to_string())
        }
    }
}
