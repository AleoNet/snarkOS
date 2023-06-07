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

use arc_swap::ArcSwap;
use std::{
    collections::{HashMap, HashSet},
    fmt,
    sync::Arc,
};

use async_trait::async_trait;
use narwhal_config::{Committee, SharedCommittee};
use narwhal_executor::ExecutionState;
use narwhal_types::ConsensusOutput;
use parking_lot::Mutex;
use rand::prelude::{IteratorRandom, Rng, SliceRandom};
use snarkos_node_bft_consensus::batched_transactions;
use tempfile::TempDir;
use tracing::*;

use super::transaction::*;

// Simple transfer-related types.
pub type Address = String;
pub type Amount = u64;

// A simple state for BFT consensus tests.
pub struct TestBftExecutionState {
    pub committee: SharedCommittee,
    pub balances: Mutex<HashMap<Address, Amount>>,
    pub processed_txs: Mutex<HashSet<u64>>,
    pub storage_dir: Arc<TempDir>,
}

impl Clone for TestBftExecutionState {
    fn clone(&self) -> Self {
        Self {
            committee: self.committee.clone(),
            balances: Mutex::new(self.balances.lock().clone()),
            processed_txs: Mutex::new(self.processed_txs.lock().clone()),
            storage_dir: Arc::clone(&self.storage_dir),
        }
    }
}

impl PartialEq for TestBftExecutionState {
    fn eq(&self, other: &Self) -> bool {
        *self.processed_txs.lock() == *other.processed_txs.lock() && *self.balances.lock() == *other.balances.lock()
    }
}

impl Eq for TestBftExecutionState {}

impl fmt::Debug for TestBftExecutionState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "processed txs: {:?}, balances: {:?}", &*self.processed_txs.lock(), &*self.balances.lock())
    }
}

impl TestBftExecutionState {
    pub fn new(committee: Committee, balances: HashMap<Address, Amount>) -> Self {
        Self {
            committee: Arc::new(ArcSwap::from_pointee(committee)),
            balances: Mutex::new(balances),
            processed_txs: Default::default(),
            storage_dir: Arc::new(TempDir::new().unwrap()),
        }
    }

    #[allow(dead_code)]
    pub fn generate_random_transfers<T: Rng>(&self, num_transfers: usize, rng: &mut T) -> Vec<Transaction> {
        let balances = self.balances.lock();

        let mut transfers = Vec::with_capacity(num_transfers);
        for i in 0..num_transfers {
            let mut sides = balances.keys().cloned().choose_multiple(rng, 2);
            sides.shuffle(rng);
            let amount = rng.gen_range(1..=MAX_TRANSFER_AMOUNT);

            let transfer = Transfer { id: i as u64, from: sides.pop().unwrap(), to: sides.pop().unwrap(), amount };
            transfers.push(Transaction::Transfer(transfer));
        }

        transfers
    }

    fn process_transactions(&self, transactions: Vec<Transaction>) {
        let mut balances = self.balances.lock();

        for transaction in transactions {
            // Skip transactions that have already been processed to avoid corrupting state.
            if !self.processed_txs.lock().insert(transaction.id()) {
                continue;
            }

            match transaction {
                Transaction::Transfer(Transfer { from, to, amount, .. }) => {
                    if amount > MAX_TRANSFER_AMOUNT {
                        continue;
                    }

                    if !balances.contains_key(&from) || !balances.contains_key(&to) {
                        continue;
                    }

                    if let Some(from_balance) = balances.get_mut(&from) {
                        if amount > *from_balance {
                            continue;
                        } else {
                            *from_balance -= amount;
                        }
                    }

                    if let Some(to_balance) = balances.get_mut(&to) {
                        *to_balance += amount;
                    }
                }

                Transaction::StakeChange(StakeChange { pub_key, change, .. }) => {
                    // Load the committee so that state is consistent between reads and writes.
                    self.committee.rcu(|committee| {
                        let previous_stake = committee.stake(&pub_key);
                        let new_stake = previous_stake.saturating_add_signed(change);

                        // Update the committee.
                        let mut committee = Committee::clone(committee);

                        if let Some(authority) = committee.authorities.get_mut(&pub_key) {
                            authority.stake = new_stake;
                        }

                        committee
                    });
                }
            }
        }
    }
}

#[async_trait]
impl ExecutionState for TestBftExecutionState {
    async fn handle_consensus_output(&self, consensus_output: ConsensusOutput) {
        // Register and log some useful information.
        let mut leader = consensus_output.sub_dag.leader.header.author.to_string();
        leader.truncate(8);
        let round = consensus_output.sub_dag.round();

        // Collect the batched transactions.
        let mut transactions = Vec::new();
        for transaction in batched_transactions(&consensus_output) {
            let transaction: Transaction = bincode::deserialize(transaction).unwrap();
            transactions.push(transaction);
        }

        info!("Consensus [leader: {leader}, round: {round}, txs: {}]", transactions.len());

        // Return early if there's no transactions.
        if consensus_output.batches.is_empty() {
            return;
        }

        self.process_transactions(transactions);
    }

    async fn last_executed_sub_dag_index(&self) -> u64 {
        0
    }
}
