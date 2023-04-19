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

use snarkvm::{
    prelude::{ConsensusStore, Identifier, Locator, Plaintext, PrivateKey, ProgramID, Query, Record, Value, VM},
    synthesizer::store::helpers::memory::ConsensusMemory,
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
    /// The transaction fee in microcredits.
    #[clap(short, long)]
    fee: Option<u64>,
    /// The record to spend the fee from.
    #[clap(short, long)]
    record: Option<String>,
    /// Display the generated transaction.
    #[clap(short, long, conflicts_with = "broadcast")]
    display: bool,
    /// The endpoint used to broadcast the generated transaction.
    #[clap(short, long, conflicts_with = "display")]
    broadcast: Option<String>,
    /// Store generated deployment transaction to a local file.
    #[clap(long)]
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
            let credits = ProgramID::<CurrentNetwork>::try_from("credits.aleo")?;
            if program.id() != &credits {
                let deployment = vm.deploy_raw(&program, rng)?;
                vm.process().write().load_deployment(&deployment)?;
            }

            // Prepare the fees.
            let fee = match self.record {
                Some(record) => Some((
                    Record::<CurrentNetwork, Plaintext<CurrentNetwork>>::from_str(&record)?,
                    self.fee.unwrap_or(0),
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
        let locator = Locator::<CurrentNetwork>::from_str(&format!("{}/{}", self.program_id, self.function))?;
        println!("✅ Created execution transaction for '{}'", locator.to_string().bold());

        // Determine if the transaction should be broadcast, stored, or displayed to user.
        Developer::handle_transaction(self.broadcast, self.display, self.store, execution, locator.to_string())
    }
}
