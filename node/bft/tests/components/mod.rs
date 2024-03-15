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

pub mod pending;
pub mod worker;

use crate::common::{primary, CurrentNetwork, TranslucentLedgerService};
use snarkos_account::Account;
use snarkos_node_bft::{helpers::Storage, Gateway, Worker};
use snarkos_node_bft_ledger_service::LedgerService;
use snarkos_node_bft_storage_service::BFTMemoryService;
use snarkvm::{
    console::{account::Address, network::Network},
    ledger::{narwhal::BatchHeader, store::helpers::memory::ConsensusMemory},
    prelude::TestRng,
};

use indexmap::IndexMap;
use parking_lot::RwLock;
use std::{str::FromStr, sync::Arc};

const ITERATIONS: u32 = 100;

/// Samples a new ledger with the given number of nodes.
pub fn sample_ledger(
    num_nodes: u16,
    rng: &mut TestRng,
) -> Arc<TranslucentLedgerService<CurrentNetwork, ConsensusMemory<CurrentNetwork>>> {
    let (accounts, committee) = primary::new_test_committee(num_nodes);
    let bonded_balances: IndexMap<_, _> =
        committee.members().iter().map(|(address, (amount, _))| (*address, (*address, *address, *amount))).collect();
    let gen_key = *accounts[0].private_key();
    let public_balance_per_validator =
        (1_500_000_000_000_000 - (num_nodes as u64) * 1_000_000_000_000) / (num_nodes as u64);
    let mut balances = IndexMap::<Address<CurrentNetwork>, u64>::new();
    for account in accounts.iter() {
        balances.insert(account.address(), public_balance_per_validator);
    }

    let gen_ledger =
        primary::genesis_ledger(gen_key, committee.clone(), balances.clone(), bonded_balances.clone(), rng);
    Arc::new(TranslucentLedgerService::new(gen_ledger, Default::default()))
}

/// Samples a new storage with the given ledger.
pub fn sample_storage<N: Network>(ledger: Arc<TranslucentLedgerService<N, ConsensusMemory<N>>>) -> Storage<N> {
    Storage::new(ledger, Arc::new(BFTMemoryService::new()), BatchHeader::<N>::MAX_GC_ROUNDS as u64)
}

/// Samples a new gateway with the given ledger.
pub fn sample_gateway<N: Network>(
    storage: Storage<N>,
    ledger: Arc<TranslucentLedgerService<N, ConsensusMemory<N>>>,
) -> Gateway<N> {
    let num_nodes: u16 = ledger.current_committee().unwrap().num_members().try_into().unwrap();
    let (accounts, _committee) = primary::new_test_committee(num_nodes);
    let account = Account::from_str(&accounts[0].private_key().to_string()).unwrap();
    // Initialize the gateway.
    Gateway::new(account, storage, ledger, None, &[], None).unwrap()
}

/// Samples a new worker with the given ledger.
pub fn sample_worker<N: Network>(id: u8, ledger: Arc<TranslucentLedgerService<N, ConsensusMemory<N>>>) -> Worker<N> {
    // Sample a storage.
    let storage = sample_storage(ledger.clone());
    // Sample a gateway.
    let gateway = sample_gateway(storage.clone(), ledger.clone());
    // Sample a dummy proposed batch.
    let proposed_batch = Arc::new(RwLock::new(None));
    // Construct the worker instance.
    Worker::new(id, Arc::new(gateway.clone()), storage.clone(), ledger, proposed_batch).unwrap()
}
