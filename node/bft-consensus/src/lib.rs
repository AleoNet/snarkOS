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

pub mod setup;
mod state;
mod validation;

use setup::*;

pub use state::{batched_transactions, sort_transactions, BftExecutionState};
pub use validation::TransactionValidator;

use anyhow::Result;
use arc_swap::ArcSwap;
use multiaddr::Protocol;
use narwhal_config::{Committee, Import, Parameters, WorkerCache};
use narwhal_crypto::{KeyPair as NarwhalKeyPair, NetworkKeyPair};
use narwhal_executor::ExecutionState;
use narwhal_node::{
    keypair_file::{read_authority_keypair_from_file, read_network_keypair_from_file},
    primary_node::PrimaryNode,
    worker_node::WorkerNode,
    NodeStorage,
};
use narwhal_types::TransactionsClient;
use std::{net::IpAddr, sync::Arc};
use tonic::transport::Channel;
use tracing::*;

use snarkvm::prelude::{ConsensusStorage, Network};

// An instance of BFT consensus that hasn't been started yet.
pub struct InertConsensusInstance<S: ExecutionState, V: narwhal_worker::TransactionValidator> {
    pub primary_keypair: NarwhalKeyPair,
    pub network_keypair: NetworkKeyPair,
    pub worker_keypairs: Vec<NetworkKeyPair>,
    pub parameters: Parameters,
    pub primary_store: NodeStorage,
    pub worker_stores: Vec<NodeStorage>,
    pub committee: Arc<ArcSwap<Committee>>,
    pub worker_cache: Arc<ArcSwap<WorkerCache>>,
    pub state: S,
    pub validator: V,
}

impl<S: ExecutionState + Send + Sync + 'static, V: narwhal_worker::TransactionValidator> InertConsensusInstance<S, V> {
    // Creates a BFT consensus instance based on filesystem configuration.
    pub fn load<N: Network, C: ConsensusStorage<N>>(state: S, validator: V, dev: Option<u16>) -> Result<Self> {
        fn dev_subpath(dev: Option<u16>) -> &'static str {
            if dev.is_some() { ".dev/" } else { "" }
        }

        // If we're running dev mode, potentially use a different primary ID than 0.
        let primary_id = if let Some(dev_id) = dev { dev_id } else { 0 };

        let base_path = format!("{}/node/bft-consensus/committee/{}", workspace_dir(), dev_subpath(dev));

        // Load the primary's keys.
        let primary_key_file = format!("{base_path}.primary-{primary_id}-key.json");
        let primary_keypair =
            read_authority_keypair_from_file(primary_key_file).expect("Failed to load the node's primary keypair");
        let primary_network_key_file = format!("{base_path}.primary-{primary_id}-network.json");
        let network_keypair = read_network_keypair_from_file(primary_network_key_file)
            .expect("Failed to load the node's primary network keypair");

        // Load the workers' keys.
        // TODO: extend to multiple workers
        let mut worker_keypairs = vec![];
        for worker_id in 0..1 {
            let worker_key_file = format!("{base_path}.worker-{primary_id}-{worker_id}-network.json");
            let worker_keypair =
                read_network_keypair_from_file(worker_key_file).expect("Failed to load the node's worker keypair");

            worker_keypairs.push(worker_keypair);
        }

        // Read the shared files describing the committee, workers and parameters.
        let committee_file = format!("{base_path}.committee.json");
        let committee = Committee::import(&committee_file).expect("Failed to load the committee information").into();
        let workers_file = format!("{base_path}.workers.json");
        let worker_cache = WorkerCache::import(&workers_file).expect("Failed to load the worker information").into();
        let parameters_file = format!("{base_path}.parameters.json");
        let parameters = Parameters::import(&parameters_file).expect("Failed to load the node's parameters");

        // Create the primary storage instance.
        let primary_store_path = primary_storage_dir(N::ID, dev);
        let primary_store = NodeStorage::reopen(primary_store_path);

        // Create the worker storage instance(s).
        // TODO: extend to multiple workers
        let mut worker_stores = vec![];
        for worker_id in 0..1 {
            let mut worker_store_path = worker_storage_dir(N::ID, worker_id, dev);
            worker_store_path.push(format!("worker-{worker_id}"));
            let worker_store = NodeStorage::reopen(worker_store_path);
            worker_stores.push(worker_store);
        }

        Ok(Self {
            primary_keypair,
            network_keypair,
            worker_keypairs,
            parameters,
            primary_store,
            worker_stores,
            committee,
            worker_cache,
            state,
            validator,
        })
    }

    /// Start the primary and worker node(s).
    pub async fn start(self) -> Result<RunningConsensusInstance<S>> {
        let primary_pub = self.primary_keypair.public().clone();
        let primary_node = PrimaryNode::new(self.parameters.clone(), true);
        let state = Arc::new(self.state);

        // Start the primary.
        primary_node
            .start(
                self.primary_keypair,
                self.network_keypair,
                self.committee.clone(),
                self.worker_cache.clone(),
                &self.primary_store,
                Arc::clone(&state),
            )
            .await?;
        info!("Created a primary with public key {}.", primary_pub);

        // Start the workers associated with the primary.
        let num_workers = self.worker_keypairs.len();
        let mut worker_nodes = Vec::with_capacity(num_workers);
        for (worker_id, worker_keypair) in self.worker_keypairs.into_iter().enumerate() {
            let worker = WorkerNode::new(worker_id as u32, self.parameters.clone());

            worker
                .start(
                    primary_pub.clone(),
                    worker_keypair,
                    self.committee.clone(),
                    self.worker_cache.clone(),
                    &self.worker_stores[worker_id],
                    self.validator.clone(),
                )
                .await?;
            info!("Created a worker with id {worker_id}.");

            worker_nodes.push(worker);
        }

        let instance = RunningConsensusInstance { primary_node, worker_nodes, worker_cache: self.worker_cache, state };

        Ok(instance)
    }
}

// An instance of BFT consensus that has been started.
#[derive(Clone)]
pub struct RunningConsensusInstance<T: ExecutionState> {
    pub primary_node: PrimaryNode,
    pub worker_nodes: Vec<WorkerNode>, // TODO: possibly change to the WorkerNodes struct
    pub worker_cache: Arc<ArcSwap<WorkerCache>>,
    pub state: Arc<T>,
}

impl<T: ExecutionState> RunningConsensusInstance<T> {
    // Spawns transaction clients capable of sending txs to BFT workers.
    // TODO: consider alternatives to tonic's Channel?
    pub fn spawn_tx_clients(&self) -> Vec<TransactionsClient<Channel>> {
        let mut tx_uris = Vec::with_capacity(
            self.worker_cache.load().workers.values().map(|worker_index| worker_index.0.len()).sum(),
        );
        for worker_set in self.worker_cache.load().workers.values() {
            for worker_info in worker_set.0.values() {
                // Construct an address usable by the tonic channel based on the worker's tx Multiaddr.
                let mut tx_ip = None;
                let mut tx_port = None;
                for component in &worker_info.transactions {
                    match component {
                        Protocol::Ip4(ip) => tx_ip = Some(IpAddr::V4(ip)),
                        Protocol::Ip6(ip) => tx_ip = Some(IpAddr::V6(ip)),
                        Protocol::Tcp(port) => tx_port = Some(port),
                        _ => {} // TODO: do we expect other combinations?
                    }
                }
                // TODO: these may be known in advance, but shouldn't be trusted when we switch to a dynamic committee
                let tx_ip = tx_ip.unwrap();
                let tx_port = tx_port.unwrap();

                let tx_uri = format!("http://{tx_ip}:{tx_port}");
                tx_uris.push(tx_uri);
            }
        }

        // Sort the channel URIs by port for greater determinism in local tests.
        tx_uris.sort_unstable();

        // Create tx channels.
        tx_uris
            .into_iter()
            .map(|uri| {
                let channel = Channel::from_shared(uri).unwrap().connect_lazy();
                TransactionsClient::new(channel)
            })
            .collect()
    }
}
