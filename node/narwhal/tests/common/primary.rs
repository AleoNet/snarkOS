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

use crate::common::{
    utils::{fire_unconfirmed_solutions, fire_unconfirmed_transactions, initialize_logger},
    CurrentNetwork,
    MockLedgerService,
};
use snarkos_account::Account;
use snarkos_node_narwhal::{
    helpers::{init_primary_channels, PrimarySender, Storage},
    Primary,
    BFT,
    MAX_BATCH_DELAY,
    MAX_GC_ROUNDS,
};
use snarkos_node_narwhal_committee::{Committee, MIN_STAKE};
use snarkvm::prelude::TestRng;

use std::{collections::HashMap, sync::Arc, time::Duration};

use indexmap::IndexMap;
use itertools::Itertools;
use parking_lot::Mutex;
use tokio::{task::JoinHandle, time::sleep};
use tracing::*;

#[derive(Clone, Copy, Debug)]
pub struct TestNetworkConfig {
    pub num_nodes: u16,
    pub bft: bool,
    pub connect_all: bool,
    pub fire_cannons: bool,
    pub log_level: Option<u8>,
    pub log_connections: bool,
}

#[derive(Clone)]
pub struct TestNetwork {
    pub config: TestNetworkConfig,
    pub validators: HashMap<u16, TestValidator>,
}

#[derive(Clone)]
pub struct TestValidator {
    pub id: u16,
    pub primary: Primary<CurrentNetwork>,
    pub primary_sender: Option<PrimarySender<CurrentNetwork>>,
    pub bft: Option<BFT<CurrentNetwork>>,
    pub handles: Arc<Mutex<Vec<JoinHandle<()>>>>,
}

impl TestValidator {
    pub fn fire_cannons(&mut self) {
        let solution_handle = fire_unconfirmed_solutions(self.primary_sender.as_mut().unwrap(), self.id);
        let transaction_handle = fire_unconfirmed_transactions(self.primary_sender.as_mut().unwrap(), self.id);

        self.handles.lock().push(solution_handle);
        self.handles.lock().push(transaction_handle);
    }

    pub fn log_connections(&mut self) {
        let self_clone = self.clone();
        self.handles.lock().push(tokio::task::spawn(async move {
            loop {
                let connections = self_clone.primary.gateway().connected_peers().read().clone();
                info!("{} connections", connections.len());
                for connection in connections {
                    debug!("  {}", connection);
                }
                sleep(Duration::from_secs(5)).await;
            }
        }));
    }
}

impl TestNetwork {
    // Creates a new test network with the given configuration.
    pub fn new(config: TestNetworkConfig) -> Self {
        if let Some(log_level) = config.log_level {
            initialize_logger(log_level);
        }

        let (accounts, committee) = new_test_committee(config.num_nodes);

        let mut validators = HashMap::with_capacity(config.num_nodes as usize);
        for (id, account) in accounts.into_iter().enumerate() {
            let storage = Storage::new(committee.clone(), MAX_GC_ROUNDS);
            let ledger = Arc::new(MockLedgerService::new());

            let (primary, bft) = if config.bft {
                let bft = BFT::<CurrentNetwork>::new(account, storage, ledger, None, Some(id as u16)).unwrap();
                (bft.primary().clone(), Some(bft))
            } else {
                let primary = Primary::<CurrentNetwork>::new(account, storage, ledger, None, Some(id as u16)).unwrap();
                (primary, None)
            };

            let test_validator =
                TestValidator { id: id as u16, primary, primary_sender: None, bft, handles: Default::default() };
            validators.insert(id as u16, test_validator);
        }

        Self { config, validators }
    }

    // Starts each node in the network.
    pub async fn start(&mut self) {
        for validator in self.validators.values_mut() {
            let (primary_sender, primary_receiver) = init_primary_channels();
            if let Some(bft) = &mut validator.bft {
                // Setup the channels and start the bft.
                validator.primary_sender = Some(primary_sender.clone());
                bft.run(primary_sender, primary_receiver, None).await.unwrap();
            } else {
                // Setup the channels and start the primary.
                validator.primary_sender = Some(primary_sender.clone());
                validator.primary.run(primary_sender, primary_receiver, None).await.unwrap();
            }

            if self.config.fire_cannons {
                validator.fire_cannons();
            }

            if self.config.log_connections {
                validator.log_connections();
            }
        }

        if self.config.connect_all {
            self.connect_all().await;
        }
    }

    // Starts the solution and trasnaction cannons for node.
    pub fn fire_cannons_at(&mut self, id: u16) {
        self.validators.get_mut(&id).unwrap().fire_cannons();
    }

    // Connects a node to another node.
    pub async fn connect_validators(&self, first_id: u16, second_id: u16) {
        let first_validator = self.validators.get(&first_id).unwrap();
        let second_validator_ip = self.validators.get(&second_id).unwrap().primary.gateway().local_ip();
        first_validator.primary.gateway().connect(second_validator_ip);
        // Give the connection time to be established.
        sleep(Duration::from_millis(100)).await;
    }

    // Connects all nodes to each other.
    pub async fn connect_all(&self) {
        for (validator, other_validator) in self.validators.values().tuple_combinations() {
            // Connect to the node.
            let ip = other_validator.primary.gateway().local_ip();
            validator.primary.gateway().connect(ip);
            // Give the connection time to be established.
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    }

    // Disconnects N nodes from all other nodes.
    pub async fn disconnect(&self, num_nodes: u16) {
        for validator in self.validators.values().take(num_nodes as usize) {
            for peer_ip in validator.primary.gateway().connected_peers().read().iter() {
                validator.primary.gateway().disconnect(*peer_ip);
            }
        }

        // Give the connections time to be closed.
        sleep(Duration::from_millis(100)).await;
    }

    // Checks if at least 2f + 1 nodes have reached the given round.
    pub fn is_round_reached(&self, round: u64) -> bool {
        let quorum_threshold = self.validators.len() / 2 + 1;
        self.validators.values().filter(|v| v.primary.current_round() >= round).count() >= quorum_threshold
    }

    // Checks if all the nodes have stopped progressing.
    pub async fn is_halted(&self) -> bool {
        let halt_round = self.validators.values().map(|v| v.primary.current_round()).max().unwrap();
        sleep(Duration::from_millis(MAX_BATCH_DELAY * 2)).await;
        self.validators.values().all(|v| v.primary.current_round() <= halt_round)
    }
}

// Initializes a new test committee.
fn new_test_committee(n: u16) -> (Vec<Account<CurrentNetwork>>, Committee<CurrentNetwork>) {
    const INITIAL_STAKE: u64 = MIN_STAKE;

    let mut accounts = Vec::with_capacity(n as usize);
    let mut members = IndexMap::with_capacity(n as usize);
    for i in 0..n {
        // Sample the account.
        let account = Account::new(&mut TestRng::fixed(i as u64)).unwrap();

        info!("Validator {}: {}", i, account.address());

        members.insert(account.address(), INITIAL_STAKE);
        accounts.push(account);
    }
    // Initialize the committee.
    let committee = Committee::<CurrentNetwork>::new(1u64, members).unwrap();

    (accounts, committee)
}
