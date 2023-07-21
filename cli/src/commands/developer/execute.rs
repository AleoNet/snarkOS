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

use super::{CurrentNetwork, Developer, Program};

use snarkvm::prelude::{
    query::Query,
    store::{helpers::memory::ConsensusMemory, ConsensusStore},
    Identifier,
    Locator,
    Plaintext,
    PrivateKey,
    ProgramID,
    Record,
    Value,
    VM,
};

use anyhow::{bail, Result};
use clap::Parser;
use colored::Colorize;
use std::str::FromStr;

/// Executes an Aleo program function.
#[derive(Debug, Parser)]
pub struct Execute {
    /// The program identifier.
    program_id: ProgramID<CurrentNetwork>,
    /// The function name.
    function: Identifier<CurrentNetwork>,
    /// The function inputs.
    inputs: Vec<Value<CurrentNetwork>>,
    /// The private key used to generate the execution.
    #[clap(short, long)]
    private_key: String,
    /// The endpoint to query node state from.
    #[clap(short, long)]
    query: String,
    /// The priority fee in microcredits.
    #[clap(short, long)]
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
}

impl Execute {
    /// Executes an Aleo program function with the provided inputs.
    #[allow(clippy::format_in_format_args)]
    pub fn parse(self) -> Result<String> {
        // Ensure that the user has specified an action.
        if !self.dry_run && self.broadcast.is_none() && self.store.is_none() {
            bail!("❌ Please specify one of the following actions: --broadcast, --dry-run, --store");
        }

        // Specify the query
        let query = Query::from(&self.query);

        // Retrieve the private key.
        let private_key = PrivateKey::from_str(&self.private_key)?;

        // Send a request to the query node.
        let response = ureq::get(&format!("{}/testnet3/program/{}", self.query, self.program_id)).call();

        // Deserialize the program.
        let program: Program<CurrentNetwork> = match response {
            Ok(response) => response.into_json()?,
            Err(err) => match err {
                ureq::Error::Status(_status, response) => {
                    bail!(response.into_string().unwrap_or("Response too large!".to_owned()))
                }
                err => bail!(err),
            },
        };

        let locator = Locator::<CurrentNetwork>::from_str(&format!("{}/{}", self.program_id, self.function))?;
        println!("📦 Creating execution transaction for '{}'...\n", &locator.to_string().bold());

        // Generate the execution transaction.
        let execution = {
            // Initialize an RNG.
            let rng = &mut rand::thread_rng();

            // Initialize the VM.
            let store = ConsensusStore::<CurrentNetwork, ConsensusMemory<CurrentNetwork>>::open(None)?;
            let vm = VM::from(store)?;

            // Add the program deployment to the VM.
            let credits = ProgramID::<CurrentNetwork>::try_from("credits.aleo")?;
            if program.id() != &credits {
                let deployment = vm.deploy_raw(&program, rng)?;
                vm.process().write().load_deployment(&deployment)?;
            }

            // Prepare the fees.
            let fee = match self.record {
                Some(record) => Some((
                    Record::<CurrentNetwork, Plaintext<CurrentNetwork>>::from_str(&record)?,
                    self.priority_fee.unwrap_or(0),
                )),
                None => {
                    // Ensure that only the `credits.aleo/split` call can be created without a fee.
                    if program.id() != &credits && self.function != Identifier::from_str("split")? {
                        bail!("❌ A record must be provided to pay for the transaction fee.");
                    }
                    None
                }
            };

            // Create a new transaction.
            vm.execute(&private_key, (self.program_id, self.function), self.inputs.iter(), fee, Some(query), rng)?
        };
        println!("✅ Created execution transaction for '{}'", locator.to_string().bold());

        // Determine if the transaction should be broadcast, stored, or displayed to user.
        Developer::handle_transaction(self.broadcast, self.dry_run, self.store, execution, locator.to_string())
    }
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
            "--fee",
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
            assert_eq!(execute.private_key, "PRIVATE_KEY");
            assert_eq!(execute.query, "QUERY");
            assert_eq!(execute.priority_fee, Some(77));
            assert_eq!(execute.record, Some("RECORD".into()));
            assert_eq!(execute.program_id, "hello.aleo".try_into().unwrap());
            assert_eq!(execute.function, "hello".try_into().unwrap());
            assert_eq!(execute.inputs, vec!["1u32".try_into().unwrap(), "2u32".try_into().unwrap()]);
        } else {
            panic!("Unexpected result of clap parsing!");
        }
    }
}
