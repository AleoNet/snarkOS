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

mod state;
mod transaction;
mod validation;

pub use state::*;
pub use transaction::*;
pub use validation::*;

use std::{sync::Arc, time::Duration};

use arc_swap::ArcSwap;
use narwhal_config::Parameters;
use narwhal_node::NodeStorage;
use tracing::*;
use tracing_subscriber::filter::{EnvFilter, LevelFilter};

use snarkos_node_bft_consensus::{setup::CommitteeSetup, InertConsensusInstance};

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

// Creates instances of BFT consensus based on the given committee setup and with
// a common initial state.
pub fn generate_consensus_instances(
    mut committee_setup: CommitteeSetup,
    state: TestBftExecutionState,
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
    let committee = Arc::new(ArcSwap::from_pointee(committee));

    // Generate the WorkerCache.
    let worker_cache = committee_setup.generate_worker_cache();
    let worker_cache = Arc::new(ArcSwap::from_pointee(worker_cache));

    // Create the consensus objects.
    let mut consensus_objects = Vec::with_capacity(committee_setup.primaries.len());
    for (primary_id, primary) in committee_setup.primaries.drain(..).enumerate() {
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
            committee: Arc::clone(&committee),
            worker_cache: Arc::clone(&worker_cache),
            state: state.clone(),
            validator: Default::default(),
        };

        consensus_objects.push(consensus);
    }

    consensus_objects
}
