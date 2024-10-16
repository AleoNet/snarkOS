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
        Locator,
        PrivateKey,
        Value,
        VM,
    },
};

use aleo_std::StorageMode;
use anyhow::{bail, Result};
use clap::Parser;
use std::{path::PathBuf, str::FromStr};
use zeroize::Zeroize;

/// Executes the `transfer_private` function in the `credits.aleo` program.
#[derive(Debug, Parser)]
pub struct TransferPrivate {
    /// Specify the network to create a `transfer_private` for.
    #[clap(default_value = "0", long = "network")]
    pub network: u16,
    /// The input record used to craft the transfer.
    #[clap(long)]
    input_record: String,
    /// The recipient address.
    #[clap(long)]
    recipient: String,
    /// The number of microcredits to transfer.
    #[clap(long)]
    amount: u64,
    /// The private key used to generate the execution.
    #[clap(short, long)]
    private_key: String,
    /// The endpoint to query node state from.
    #[clap(short, long)]
    query: String,
    /// The priority fee in microcredits.
    #[clap(long)]
    priority_fee: u64,
    /// The record to spend the fee from.
    #[clap(long)]
    fee_record: String,
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

impl Drop for TransferPrivate {
    /// Zeroize the private key when the `TransferPrivate` struct goes out of scope.
    fn drop(&mut self) {
        self.private_key.zeroize();
    }
}

impl TransferPrivate {
    /// Creates an Aleo transfer with the provided inputs.
    #[allow(clippy::format_in_format_args)]
    pub fn parse(self) -> Result<String> {
        // Ensure that the user has specified an action.
        if !self.dry_run && self.broadcast.is_none() && self.store.is_none() {
            bail!("❌ Please specify one of the following actions: --broadcast, --dry-run, --store");
        }

        // Construct the transfer for the specified network.
        match self.network {
            MainnetV0::ID => self.construct_transfer_private::<MainnetV0>(),
            TestnetV0::ID => self.construct_transfer_private::<TestnetV0>(),
            CanaryV0::ID => self.construct_transfer_private::<CanaryV0>(),
            unknown_id => bail!("Unknown network ID ({unknown_id})"),
        }
    }

    /// Construct and process the `transfer_private` transaction.
    fn construct_transfer_private<N: Network>(&self) -> Result<String> {
        // Specify the query
        let query = Query::from(&self.query);

        // Retrieve the recipient.
        let recipient = Address::<N>::from_str(&self.recipient)?;

        // Retrieve the private key.
        let private_key = PrivateKey::from_str(&self.private_key)?;

        println!("📦 Creating private transfer of {} microcredits to {}...\n", self.amount, recipient);

        // Generate the transfer_private transaction.
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

            // Prepare the fee.
            let fee_record = Developer::parse_record(&private_key, &self.fee_record)?;
            let priority_fee = self.priority_fee;

            // Prepare the inputs for a transfer.
            let input_record = Developer::parse_record(&private_key, &self.input_record)?;
            let inputs = vec![
                Value::Record(input_record),
                Value::from_str(&format!("{}", recipient))?,
                Value::from_str(&format!("{}u64", self.amount))?,
            ];

            // Create a new transaction.
            vm.execute(
                &private_key,
                ("credits.aleo", "transfer_private"),
                inputs.iter(),
                Some(fee_record),
                priority_fee,
                Some(query),
                rng,
            )?
        };
        let locator = Locator::<N>::from_str("credits.aleo/transfer_private")?;
        println!("✅ Created private transfer of {} microcredits to {}\n", &self.amount, recipient);

        // Determine if the transaction should be broadcast, stored, or displayed to the user.
        Developer::handle_transaction(&self.broadcast, self.dry_run, &self.store, transaction, locator.to_string())
    }
}
