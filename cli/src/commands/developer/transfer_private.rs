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

use super::{CurrentNetwork, Developer};

use snarkvm::{
    prelude::{Address, ConsensusStore, Locator, Plaintext, PrivateKey, Query, Record, Value, VM},
    synthesizer::store::helpers::memory::ConsensusMemory,
};

use anyhow::Result;
use clap::Parser;
use std::str::FromStr;

/// Executes the `transfer_private` function in the `credits.aleo` program.
#[derive(Debug, Parser)]
pub struct TransferPrivate {
    /// The input record used to craft the transfer.
    #[clap(long)]
    input_record: Record<CurrentNetwork, Plaintext<CurrentNetwork>>,
    /// The recipient address.
    #[clap(long)]
    recipient: Address<CurrentNetwork>,
    /// The number of microcredits to transfer.
    #[clap(long)]
    amount: u64,
    /// The private key used to generate the execution.
    #[clap(short, long)]
    private_key: String,
    /// The endpoint to query node state from.
    #[clap(short, long)]
    query: String,
    /// The transaction fee in microcredits.
    #[clap(short, long)]
    fee: u64,
    /// The record to spend the fee from.
    #[clap(long)]
    fee_record: String,
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

impl TransferPrivate {
    /// Creates an Aleo transfer with the provided inputs.
    #[allow(clippy::format_in_format_args)]
    pub fn parse(self) -> Result<String> {
        // Specify the query
        let query = Query::from(&self.query);

        // Retrieve the private key.
        let private_key = PrivateKey::from_str(&self.private_key)?;

        println!("📦 Creating private transfer of {} microcredits to {}...\n", self.amount, self.recipient);

        // Generate the transfer transaction.
        let execution = {
            // Initialize an RNG.
            let rng = &mut rand::thread_rng();

            // Initialize the VM.
            let store = ConsensusStore::<CurrentNetwork, ConsensusMemory<CurrentNetwork>>::open(None)?;
            let vm = VM::from(store)?;

            // Prepare the fees.
            let fee = (Record::<CurrentNetwork, Plaintext<CurrentNetwork>>::from_str(&self.fee_record)?, self.fee);

            // Prepare the inputs for a transfer.
            let inputs = vec![
                Value::Record(self.input_record.clone()),
                Value::from_str(&format!("{}", self.recipient))?,
                Value::from_str(&format!("{}u64", self.amount))?,
            ];

            // Create a new transaction.
            vm.execute(&private_key, ("credits.aleo", "transfer_private"), inputs.iter(), Some(fee), Some(query), rng)?
        };
        let locator = Locator::<CurrentNetwork>::from_str("credits.aleo/transfer_private")?;
        println!("✅ Created private transfer of {} microcredits to {}...\n", &self.amount, self.recipient);

        // Determine if the transaction should be broadcast, stored, or displayed to user.
        Developer::handle_transaction(self.broadcast, self.display, self.store, execution, locator.to_string())
    }
}
