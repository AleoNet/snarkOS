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
    Transaction,
    Value,
    VM,
};

use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use std::str::FromStr;

/// Executes an Aleo program function.
#[derive(Debug, Parser)]
pub struct Execute {
    /// The program identifier.
    #[clap(parse(try_from_str))]
    program_id: ProgramID<CurrentNetwork>,
    /// The function name.
    #[clap(parse(try_from_str))]
    function: Identifier<CurrentNetwork>,
    /// The function inputs.
    #[clap(parse(try_from_str))]
    inputs: Vec<Value<CurrentNetwork>>,
    /// The private key used to generate the execution.
    #[clap(short, long)]
    private_key: String,
    /// The endpoint to query node state from.
    #[clap(short, long)]
    query: String,
    /// The deployment fee in gates, defaults to 0.
    #[clap(short, long)]
    fee: Option<u64>,
    /// The record to spend the fee from.
    #[clap(short, long)]
    record: Option<String>,
    /// Display the generated transaction.
    #[clap(short, long, default_value = "true", conflicts_with = "broadcast")]
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

        // Determine if the transaction should be broadcast, stored, or displayed to user.
        Developer::handle_transaction(self.broadcast, self.display, self.store, execution, locator.to_string())
    }
}
