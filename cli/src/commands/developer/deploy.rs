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
    circuit::{Aleo, AleoCanaryV0, AleoTestnetV0, AleoV0},
    console::{
        network::{CanaryV0, MainnetV0, Network, TestnetV0},
        program::ProgramOwner,
    },
    prelude::{
        block::Transaction,
        deployment_cost,
        query::Query,
        store::{helpers::memory::ConsensusMemory, ConsensusStore},
        PrivateKey,
        ProgramID,
        VM,
    },
};

use aleo_std::StorageMode;
use anyhow::{bail, Result};
use clap::Parser;
use colored::Colorize;
use std::{path::PathBuf, str::FromStr};
use zeroize::Zeroize;

/// Deploys an Aleo program.
#[derive(Debug, Parser)]
pub struct Deploy {
    /// The name of the program to deploy.
    program_id: String,
    /// Specify the network to create a deployment for.
    #[clap(default_value = "0", long = "network")]
    pub network: u16,
    /// A path to a directory containing a manifest file. Defaults to the current working directory.
    #[clap(long)]
    path: Option<String>,
    /// The private key used to generate the deployment.
    #[clap(short, long)]
    private_key: String,
    /// The endpoint to query node state from.
    #[clap(short, long)]
    query: String,
    /// The priority fee in microcredits.
    #[clap(long)]
    priority_fee: u64,
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
    storage_path: Option<PathBuf>,
}

impl Drop for Deploy {
    /// Zeroize the private key when the `Deploy` struct goes out of scope.
    fn drop(&mut self) {
        self.private_key.zeroize();
    }
}

impl Deploy {
    /// Deploys an Aleo program.
    pub fn parse(self) -> Result<String> {
        // Ensure that the user has specified an action.
        if !self.dry_run && self.broadcast.is_none() && self.store.is_none() {
            bail!("❌ Please specify one of the following actions: --broadcast, --dry-run, --store");
        }

        // Construct the deployment for the specified network.
        match self.network {
            MainnetV0::ID => self.construct_deployment::<MainnetV0, AleoV0>(),
            TestnetV0::ID => self.construct_deployment::<TestnetV0, AleoTestnetV0>(),
            CanaryV0::ID => self.construct_deployment::<CanaryV0, AleoCanaryV0>(),
            unknown_id => bail!("Unknown network ID ({unknown_id})"),
        }
    }

    /// Construct and process the deployment transaction.
    fn construct_deployment<N: Network, A: Aleo<Network = N, BaseField = N::Field>>(&self) -> Result<String> {
        // Specify the query
        let query = Query::from(&self.query);

        // Retrieve the private key.
        let private_key = PrivateKey::from_str(&self.private_key)?;

        // Retrieve the program ID.
        let program_id = ProgramID::from_str(&self.program_id)?;

        // Fetch the package from the directory.
        let package = Developer::parse_package(program_id, &self.path)?;

        println!("📦 Creating deployment transaction for '{}'...\n", &program_id.to_string().bold());

        // Generate the deployment
        let deployment = package.deploy::<A>(None)?;
        let deployment_id = deployment.to_deployment_id()?;

        // Generate the deployment transaction.
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

            // Compute the minimum deployment cost.
            let (minimum_deployment_cost, (_, _, _)) = deployment_cost(&deployment)?;

            // Prepare the fees.
            let fee = match &self.record {
                Some(record) => {
                    let fee_record = Developer::parse_record(&private_key, record)?;
                    let fee_authorization = vm.authorize_fee_private(
                        &private_key,
                        fee_record,
                        minimum_deployment_cost,
                        self.priority_fee,
                        deployment_id,
                        rng,
                    )?;
                    vm.execute_fee_authorization(fee_authorization, Some(query), rng)?
                }
                None => {
                    let fee_authorization = vm.authorize_fee_public(
                        &private_key,
                        minimum_deployment_cost,
                        self.priority_fee,
                        deployment_id,
                        rng,
                    )?;
                    vm.execute_fee_authorization(fee_authorization, Some(query), rng)?
                }
            };
            // Construct the owner.
            let owner = ProgramOwner::new(&private_key, deployment_id, rng)?;

            // Create a new transaction.
            Transaction::from_deployment(owner, deployment, fee)?
        };
        println!("✅ Created deployment transaction for '{}'", program_id.to_string().bold());

        // Determine if the transaction should be broadcast, stored, or displayed to the user.
        Developer::handle_transaction(&self.broadcast, self.dry_run, &self.store, transaction, program_id.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::{Command, CLI};

    #[test]
    fn clap_snarkos_deploy() {
        let arg_vec = vec![
            "snarkos",
            "developer",
            "deploy",
            "--private-key",
            "PRIVATE_KEY",
            "--query",
            "QUERY",
            "--priority-fee",
            "77",
            "--record",
            "RECORD",
            "hello.aleo",
        ];
        let cli = CLI::parse_from(arg_vec);

        if let Command::Developer(Developer::Deploy(deploy)) = cli.command {
            assert_eq!(deploy.network, 0);
            assert_eq!(deploy.program_id, "hello.aleo");
            assert_eq!(deploy.private_key, "PRIVATE_KEY");
            assert_eq!(deploy.query, "QUERY");
            assert_eq!(deploy.priority_fee, 77);
            assert_eq!(deploy.record, Some("RECORD".to_string()));
        } else {
            panic!("Unexpected result of clap parsing!");
        }
    }
}
