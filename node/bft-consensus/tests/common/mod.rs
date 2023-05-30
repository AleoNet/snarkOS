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

mod state;
mod transaction;
mod validation;

pub use state::*;
pub use transaction::*;
pub use validation::*;

use std::{collections::HashMap, sync::Arc, time::Duration};

use arc_swap::ArcSwap;
use narwhal_config::Parameters;
use narwhal_node::NodeStorage;
use rand::thread_rng;
use tracing::*;
use tracing_subscriber::filter::{EnvFilter, LevelFilter};

use snarkos_node_bft_consensus::{
    setup::{CommitteeSetup, PrimarySetup},
    InertConsensusInstance,
    RunningConsensusInstance,
};

#[allow(dead_code)]
pub fn start_logger(default_level: LevelFilter) {
    let filter = match EnvFilter::try_from_default_env() {
        Ok(filter) => filter
            .add_directive("anemo=off".parse().unwrap())
            .add_directive("narwhal_config=off".parse().unwrap())
            .add_directive("rustls=off".parse().unwrap())
            .add_directive("tokio_util=off".parse().unwrap())
            .add_directive("typed_store=off".parse().unwrap()),
        _ => EnvFilter::default()
            .add_directive(default_level.into())
            .add_directive("anemo=off".parse().unwrap())
            .add_directive("narwhal_config=off".parse().unwrap())
            .add_directive("rustls=off".parse().unwrap())
            .add_directive("tokio_util=off".parse().unwrap())
            .add_directive("typed_store=off".parse().unwrap()),
    };

    tracing_subscriber::fmt().with_env_filter(filter).with_target(true).init();
}

// Creates running consensus instances based on the number of primaries and the initial stake.
pub async fn generate_running_consensus_instances(
    num_primaries: usize,
    primary_stake: u64,
) -> (TestBftExecutionState, Vec<RunningConsensusInstance<TestBftExecutionState>>) {
    // Prepare a source of randomness for key generation.
    let mut rng = thread_rng();

    // Generate the committee setup.
    let mut primaries = Vec::with_capacity(num_primaries);
    for _ in 0..num_primaries {
        let primary = PrimarySetup::new(None, primary_stake, vec![], &mut rng);
        primaries.push(primary);
    }

    // Generate the Committee.
    let committee_setup = CommitteeSetup::new(primaries, 0);
    let committee = committee_setup.generate_committee();

    // Prepare the initial balances.
    let mut balances = HashMap::new();
    balances.insert("Alice".into(), 1_000_000);
    balances.insert("Bob".into(), 2_000_000);
    balances.insert("Chad".into(), 3_000_000);

    // Set up a copy of the initial state so that we can use it to generate valid transactions in
    // tests.
    let state = TestBftExecutionState::new(committee, balances.clone());

    // Create the preconfigured consensus instances.
    let inert_consensus_instances = generate_consensus_instances(committee_setup, &balances);

    // Start the consensus instances.
    let mut running_consensus_instances = Vec::with_capacity(num_primaries);
    for instance in inert_consensus_instances {
        let running_instance = instance.start().await.unwrap();
        running_consensus_instances.push(running_instance);
    }

    (state, running_consensus_instances)
}

// Creates instances of BFT consensus based on the given committee setup and with
// a common initial state.
pub fn generate_consensus_instances(
    committee_setup: CommitteeSetup,
    balances: &HashMap<Address, Amount>,
) -> Vec<InertConsensusInstance<TestBftExecutionState, TestTransactionValidator>> {
    // Generate the Parameters.
    // TODO: tweak them further for test purposes?
    let mut parameters = Parameters::default();

    // These tweaks are necessary in order to avoid "address already in use" errors.
    parameters.network_admin_server.primary_network_admin_server_port = 0;
    parameters.network_admin_server.worker_network_admin_server_base_port = 0;

    // Tweaks that make log inspection a bit more practical etc.
    parameters.gc_depth = 100;
    parameters.max_header_num_of_batches = 50;
    parameters.min_header_delay = Duration::from_millis(500);
    parameters.max_header_delay = Duration::from_secs(2);

    debug!("Using the following consensus parameters: {:#?}", parameters);

    // Generate the Committee.
    let committee = committee_setup.generate_committee();

    // Generate the WorkerCache.
    let worker_cache = committee_setup.generate_worker_cache();
    let worker_cache = Arc::new(ArcSwap::from_pointee(worker_cache));

    // Create the consensus objects.
    let mut consensus_objects = Vec::with_capacity(committee_setup.primaries.len());
    for (primary_id, primary) in committee_setup.primaries.into_values().enumerate() {
        // Prepare the state and the committee. This needs to be done in the loop so that all nodes don't point to the same Arc'ed committee state.
        let state = TestBftExecutionState::new(committee.clone(), balances.clone());

        // Prepare the temporary folder for storage.
        let base_path = state.storage_dir.path();

        // Create the primary storage instance.
        let mut primary_store_path = base_path.to_owned();
        primary_store_path.push(format!("primary-{primary_id}"));
        let primary_store = NodeStorage::reopen(primary_store_path);

        // Create the worker storage instance(s).
        let mut worker_stores = Vec::with_capacity(primary.workers.len());
        for worker_id in 0..primary.workers.len() {
            let mut worker_store_path = base_path.to_owned();
            worker_store_path.push(format!("worker-{primary_id}-{worker_id}"));
            let worker_store = NodeStorage::reopen(worker_store_path);
            worker_stores.push(worker_store);
        }

        // Create the full consensus instance.
        let consensus = InertConsensusInstance {
            primary_keypair: primary.keypair,
            network_keypair: primary.network_keypair,
            worker_keypairs: primary.workers.into_iter().map(|w| w.network_keypair).collect(),
            parameters: parameters.clone(),
            primary_store,
            worker_stores,
            committee: Arc::clone(&state.committee),
            worker_cache: Arc::clone(&worker_cache),
            state: state.clone(),
            validator: Default::default(),
        };

        consensus_objects.push(consensus);
    }

    consensus_objects
}
