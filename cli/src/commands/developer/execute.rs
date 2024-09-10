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

use super::Developer;
use snarkvm::{
    console::network::{CanaryV0, MainnetV0, Network, TestnetV0},
    prelude::{
        query::Query,
        store::{helpers::memory::ConsensusMemory, ConsensusStore},
        Address,
        Identifier,
        Locator,
        PrivateKey,
        Process,
        ProgramID,
        Value,
        VM,
    },
};

use aleo_std::StorageMode;
use anyhow::{anyhow, bail, Result};
use clap::Parser;
use colored::Colorize;
use std::{path::PathBuf, str::FromStr};
use zeroize::Zeroize;

/// Executes an Aleo program function.
#[derive(Debug, Parser)]
pub struct Execute {
    /// The program identifier.
    program_id: String,
    /// The function name.
    function: String,
    /// The function inputs.
    inputs: Vec<String>,
    /// Specify the network to create an execution for.
    #[clap(default_value = "0", long = "network")]
    pub network: u16,
    /// The private key used to generate the execution.
    #[clap(short, long)]
    private_key: String,
    /// The endpoint to query node state from.
    #[clap(short, long)]
    query: String,
    /// The priority fee in microcredits.
    #[clap(long)]
    priority_fee: Option<u64>,
    /// The record to spend the fee from.
    #[clap(short, long)]
    record: Option<String>,
    /// The endpoint used to broadcast the generated transaction.
    #[clap(short, long, conflicts_with = "dry_run")]
    broadcast: Option<String>,
    /// Performs a dry-run of transaction generation.
    #[clap(short, long, conflicts_with = "broadcast")]
    dry_run: bool,
    /// Store generated deployment transaction to a local file.
    #[clap(long)]
    store: Option<String>,
    /// Specify the path to a directory containing the ledger
    #[clap(long = "storage_path")]
    pub storage_path: Option<PathBuf>,
}

impl Drop for Execute {
    /// Zeroize the private key when the `Execute` struct goes out of scope.
    fn drop(&mut self) {
        self.private_key.zeroize();
    }
}

impl Execute {
    /// Executes an Aleo program function with the provided inputs.
    #[allow(clippy::format_in_format_args)]
    pub fn parse(self) -> Result<String> {
        // Ensure that the user has specified an action.
        if !self.dry_run && self.broadcast.is_none() && self.store.is_none() {
            bail!("❌ Please specify one of the following actions: --broadcast, --dry-run, --store");
        }

        // Construct the execution for the specified network.
        match self.network {
            MainnetV0::ID => self.construct_execution::<MainnetV0>(),
            TestnetV0::ID => self.construct_execution::<TestnetV0>(),
            CanaryV0::ID => self.construct_execution::<CanaryV0>(),
            unknown_id => bail!("Unknown network ID ({unknown_id})"),
        }
    }

    /// Construct and process the execution transaction.
    fn construct_execution<N: Network>(&self) -> Result<String> {
        // Specify the query
        let query = Query::from(&self.query);

        // Retrieve the private key.
        let private_key = PrivateKey::from_str(&self.private_key)?;

        // Retrieve the program ID.
        let program_id = ProgramID::from_str(&self.program_id)?;

        // Retrieve the function.
        let function = Identifier::from_str(&self.function)?;

        // Retrieve the inputs.
        let inputs = self.inputs.iter().map(|input| Value::from_str(input)).collect::<Result<Vec<Value<N>>>>()?;

        let locator = Locator::<N>::from_str(&format!("{}/{}", program_id, function))?;
        println!("📦 Creating execution transaction for '{}'...\n", &locator.to_string().bold());

        // Generate the execution transaction.
        let transaction = {
            // Initialize an RNG.
            let rng = &mut rand::thread_rng();

            // Initialize the storage.
            let storage_mode = match &self.storage_path {
                Some(path) => StorageMode::Custom(path.clone()),
                None => StorageMode::Production,
            };
            let store = ConsensusStore::<N, ConsensusMemory<N>>::open(storage_mode)?;

            // Initialize the VM.
            let vm = VM::from(store)?;

            // Load the program and it's imports into the process.
            load_program(&self.query, &mut vm.process().write(), &program_id)?;

            // Prepare the fee.
            let fee_record = match &self.record {
                Some(record_string) => Some(Developer::parse_record(&private_key, record_string)?),
                None => None,
            };
            let priority_fee = self.priority_fee.unwrap_or(0);

            // Create a new transaction.
            vm.execute(&private_key, (program_id, function), inputs.iter(), fee_record, priority_fee, Some(query), rng)?
        };

        // Check if the public balance is sufficient.
        if self.record.is_none() {
            // Fetch the public balance.
            let address = Address::try_from(&private_key)?;
            let public_balance = Developer::get_public_balance(&address, &self.query)?;

            // Check if the public balance is sufficient.
            let storage_cost = transaction
                .execution()
                .ok_or_else(|| anyhow!("The transaction does not contain an execution"))?
                .size_in_bytes()?;

            // Calculate the base fee.
            // This fee is the minimum fee required to pay for the transaction,
            // excluding any finalize fees that the execution may incur.
            let base_fee = storage_cost.saturating_add(self.priority_fee.unwrap_or(0));

            // If the public balance is insufficient, return an error.
            if public_balance < base_fee {
                bail!(
                    "❌ The public balance of {} is insufficient to pay the base fee for `{}`",
                    public_balance,
                    locator.to_string().bold()
                );
            }
        }

        println!("✅ Created execution transaction for '{}'", locator.to_string().bold());

        // Determine if the transaction should be broadcast, stored, or displayed to the user.
        Developer::handle_transaction(&self.broadcast, self.dry_run, &self.store, transaction, locator.to_string())
    }
}

/// A helper function to recursively load the program and all of its imports into the process.
fn load_program<N: Network>(endpoint: &str, process: &mut Process<N>, program_id: &ProgramID<N>) -> Result<()> {
    // Fetch the program.
    let program = Developer::fetch_program(program_id, endpoint)?;

    // Return early if the program is already loaded.
    if process.contains_program(program.id()) {
        return Ok(());
    }

    // Iterate through the program imports.
    for import_program_id in program.imports().keys() {
        // Add the imports to the process if does not exist yet.
        if !process.contains_program(import_program_id) {
            // Recursively load the program and its imports.
            load_program(endpoint, process, import_program_id)?;
        }
    }

    // Add the program to the process if it does not already exist.
    if !process.contains_program(program.id()) {
        process.add_program(&program)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::{Command, CLI};

    #[test]
    fn clap_snarkos_execute() {
        let arg_vec = vec![
            "snarkos",
            "developer",
            "execute",
            "--private-key",
            "PRIVATE_KEY",
            "--query",
            "QUERY",
            "--priority-fee",
            "77",
            "--record",
            "RECORD",
            "hello.aleo",
            "hello",
            "1u32",
            "2u32",
        ];
        let cli = CLI::parse_from(arg_vec);

        if let Command::Developer(Developer::Execute(execute)) = cli.command {
            assert_eq!(execute.network, 0);
            assert_eq!(execute.private_key, "PRIVATE_KEY");
            assert_eq!(execute.query, "QUERY");
            assert_eq!(execute.priority_fee, Some(77));
            assert_eq!(execute.record, Some("RECORD".into()));
            assert_eq!(execute.program_id, "hello.aleo".to_string());
            assert_eq!(execute.function, "hello".to_string());
            assert_eq!(execute.inputs, vec!["1u32".to_string(), "2u32".to_string()]);
        } else {
            panic!("Unexpected result of clap parsing!");
        }
    }
}
