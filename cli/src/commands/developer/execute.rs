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

use super::{CurrentNetwork, Developer, Program};

use snarkvm::prelude::{
    ConsensusMemory,
    ConsensusStore,
    Identifier,
    Locator,
    Plaintext,
    PrivateKey,
    ProgramID,
    Query,
    Record,
    ToBytes,
    Transaction,
    Value,
    VM,
};

use anyhow::{ensure, Result};
use clap::Parser;
use colored::Colorize;
use std::{path::PathBuf, str::FromStr};

/// Executes an Aleo program function.
#[derive(Debug, Parser)]
pub struct Execute {
    /// The program identifier.
    #[clap(parse(try_from_str), help = "The ID of the program")]
    program_id: ProgramID<CurrentNetwork>,
    /// The function name.
    #[clap(parse(try_from_str), help = "The name of the function")]
    function: Identifier<CurrentNetwork>,
    /// The function inputs.
    #[clap(parse(try_from_str), help = "The function inputs")]
    inputs: Vec<Value<CurrentNetwork>>,
    /// The private key used to generate the execution.
    #[clap(short = 'p', long, help = "The private key used to generate the execution")]
    private_key: String,
    /// The endpoint to query node state from.
    #[clap(short = 'q', long, help = "The endpoint to query node state from")]
    query: String,
    /// The deployment fee in gates.
    #[clap(short, long, help = "The deployment fee in gates, defaults to 0.")]
    fee: Option<u64>,
    /// The record to spend the fee from.
    #[clap(short, long, help = "The record to spend the fee from.")]
    record: Option<String>,
    /// Display the generated transaction.
    #[clap(
        short,
        long,
        default_value = "true",
        help = "Display the generated transaction",
        conflicts_with = "broadcast"
    )]
    display: bool,
    #[clap(short, long, help = "The endpoint used to broadcast the generated transaction", conflicts_with = "display")]
    broadcast: Option<String>,
    #[clap(long, help = "Store generated deployment transaction to a local file")]
    store: Option<String>,
}

impl Execute {
    /// Executes an Aleo program function with the provided inputs.
    #[allow(clippy::format_in_format_args)]
    pub fn parse(self) -> Result<String> {
        // Specify the query
        let query = Query::from(&self.query);

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

        // Fetch the program from query node.
        let program: Program<CurrentNetwork> =
            ureq::get(&format!("{}/testnet3/program/{}", self.query, self.program_id)).call()?.into_json()?;

        println!("📦 Creating execution transaction for '{}'...\n", &self.program_id.to_string().bold());

        // Generate the execution transaction.
        let execution = {
            // Initialize an RNG.
            let rng = &mut rand::thread_rng();

            // Initialize the VM.
            let store = ConsensusStore::<CurrentNetwork, ConsensusMemory<CurrentNetwork>>::open(None)?;
            let vm = VM::from(store)?;

            // Add the program deployment to the VM.
            if program.id() != &ProgramID::<CurrentNetwork>::try_from("credits.aleo")? {
                let deployment = vm.deploy(&program, rng)?;
                vm.process().write().finalize_deployment(vm.program_store(), &deployment)?;
            }

            // Prepare the fees.
            let fee = match self.record {
                Some(record) => {
                    let record = Record::<CurrentNetwork, Plaintext<CurrentNetwork>>::from_str(&record)?;
                    let fee_amount = self.fee.unwrap_or(0);

                    Some((record, fee_amount))
                }
                None => None,
            };

            // Create a new transaction.
            Transaction::execute(
                &vm,
                &private_key,
                self.program_id,
                self.function,
                self.inputs.iter(),
                fee,
                Some(query),
                rng,
            )?
        };
        let locator = Locator::<CurrentNetwork>::from_str(&format!("{}/{}", self.program_id, self.function))?;
        format!("✅ Created execution transaction for '{}'", locator.to_string().bold());

        // Store the execution transaction to the specified file path.
        if let Some(file_path) = store_transaction {
            let execution_bytes = execution.to_bytes_le()?;
            std::fs::write(file_path, execution_bytes)?;
        }

        // Determine if the transaction should be broadcast or displayed to user.
        Developer::handle_transaction(self.broadcast, self.display, execution, locator.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execute() {}

    #[test]
    fn test_failed_execution() {}

    // TODO (raychu86):
    // 1. Execution without deployment
    // 2. Execution with incorrect inputs
    // 3. Execution with incorrect function
}
