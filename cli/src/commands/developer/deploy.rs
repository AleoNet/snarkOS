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

use super::{CurrentNetwork, Developer};

use snarkvm::prelude::{
    ConsensusMemory,
    ConsensusStore,
    Plaintext,
    PrivateKey,
    ProgramID,
    Query,
    Record,
    ToBytes,
    Transaction,
    VM,
};

use anyhow::{ensure, Result};
use clap::Parser;
use colored::Colorize;
use std::{path::PathBuf, str::FromStr};

/// Deploys an Aleo program.
#[derive(Debug, Parser)]
pub struct Deploy {
    /// The name of the program to deploy.
    #[clap(parse(try_from_str), help = "The ID of the program to deploy")]
    program_id: ProgramID<CurrentNetwork>,
    /// A path to a directory containing a manifest file. Defaults to the current working directory.
    #[clap(long, help = "A path to a directory containing a manifest file")]
    path: Option<String>,
    /// The private key used to generate the deployment.
    #[clap(short = 'p', long, help = "The private key used to generate the deployment")]
    private_key: String,
    /// The endpoint to query node state from.
    #[clap(short = 'q', long, help = "The endpoint to query node state from")]
    query: String,
    // TODO (raychu86): Update the default.
    /// The deployment fee in gates.
    #[clap(short, long, help = "The deployment fee in gates, defaults to 0")]
    fee: Option<u64>,
    /// The record to spend the fee from.
    #[clap(short, long, help = "The record to spend the fee from")]
    record: String,
    /// Display the generated transaction.
    #[clap(
        short,
        long,
        default_value = "false",
        help = "Display the generated transaction",
        conflicts_with = "broadcast"
    )]
    display: bool,
    #[clap(short, long, help = "The endpoint used to broadcast the generated transaction", conflicts_with = "display")]
    broadcast: Option<String>,
    #[clap(long, help = "Store generated deployment transaction to a local file")]
    store: Option<String>,
}

impl Deploy {
    /// Deploys an Aleo program.
    pub fn parse(self) -> Result<String> {
        // Specify the query
        let query = Query::from(self.query);

        // Retrieve the private key.
        let private_key = PrivateKey::from_str(&self.private_key)?;

        // Validate the storage destination for the transaction.
        let store_transaction = match self.store {
            Some(path) => {
                let file_path = PathBuf::from_str(&path)?;
                // Ensure the file path doesnt already exist.
                ensure!(!file_path.exists(), "The file path already exists exist: {}", file_path.display());

                Some(file_path)
            }
            None => None,
        };

        // Fetch the program from the directory.
        let program = Developer::parse_program(self.program_id, self.path)?;

        println!("📦 Creating deployment transaction for '{}'...\n", &self.program_id.to_string().bold());

        // Generate the deployment transaction.
        let deployment = {
            // Initialize an RNG.
            let rng = &mut rand::thread_rng();

            // Initialize the VM.
            let store = ConsensusStore::<CurrentNetwork, ConsensusMemory<CurrentNetwork>>::open(None)?;
            let vm = VM::from(store)?;

            // Prepare the fees.
            let fee_record = Record::<CurrentNetwork, Plaintext<CurrentNetwork>>::from_str(&self.record)?;

            // TODO (raychu86): Handle default fee.
            let fee_amount = self.fee.unwrap_or(0);

            // Create a new transaction.
            Transaction::deploy(&vm, &private_key, &program, (fee_record, fee_amount), Some(query), rng)?
        };
        format!("✅ Created deployment transaction for '{}'", self.program_id.to_string().bold());

        // Store the deployment transaction to the specified file path.
        if let Some(file_path) = store_transaction {
            let deployment_bytes = deployment.to_bytes_le()?;
            std::fs::write(file_path, deployment_bytes)?;
        }

        // Determine if the transaction should be broadcast or displayed to user.
        Developer::handle_transaction(self.broadcast, self.display, deployment, self.program_id.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decrypt() {}

    #[test]
    fn test_failed_decryption() {}
}
