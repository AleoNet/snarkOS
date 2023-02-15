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

use super::{CurrentNetwork, Developer};

use snarkvm::prelude::{
    ConsensusMemory,
    ConsensusStore,
    Plaintext,
    PrivateKey,
    ProgramID,
    Query,
    Record,
    Transaction,
    VM,
};

use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use std::str::FromStr;

/// Deploys an Aleo program.
#[derive(Debug, Parser)]
pub struct Deploy {
    /// The name of the program to deploy.
    #[clap(parse(try_from_str))]
    program_id: ProgramID<CurrentNetwork>,
    /// A path to a directory containing a manifest file. Defaults to the current working directory.
    #[clap(long)]
    path: Option<String>,
    /// The private key used to generate the deployment.
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
    record: String,
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

impl Deploy {
    /// Deploys an Aleo program.
    pub fn parse(self) -> Result<String> {
        // Specify the query
        let query = Query::from(self.query);

        // Retrieve the private key.
        let private_key = PrivateKey::from_str(&self.private_key)?;

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

        // Determine if the transaction should be broadcast, stored, or displayed to user.
        Developer::handle_transaction(self.broadcast, self.display, self.store, deployment, self.program_id.to_string())
    }
}
