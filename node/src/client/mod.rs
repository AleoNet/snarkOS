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

mod router;

use crate::traits::NodeInterface;
use snarkos_account::Account;
use snarkos_node_bft::ledger_service::CoreLedgerService;
use snarkos_node_rest::Rest;
use snarkos_node_router::{
    messages::{Message, NodeType, UnconfirmedSolution, UnconfirmedTransaction},
    Heartbeat,
    Inbound,
    Outbound,
    Router,
    Routing,
};
use snarkos_node_sync::{BlockSync, BlockSyncMode};
use snarkos_node_tcp::{
    protocols::{Disconnect, Handshake, OnConnect, Reading, Writing},
    P2P,
};
use snarkvm::{
    console::network::Network,
    ledger::{
        block::{Block, Header},
        puzzle::{Puzzle, Solution, SolutionID},
        store::ConsensusStorage,
        Ledger,
    },
    prelude::block::Transaction,
};

use aleo_std::StorageMode;
use anyhow::Result;
use core::future::Future;
use lru::LruCache;
use parking_lot::Mutex;
use std::{
    net::SocketAddr,
    num::NonZeroUsize,
    sync::{
        atomic::{
            AtomicBool,
            AtomicUsize,
            Ordering::{Acquire, Relaxed},
        },
        Arc,
    },
    time::Duration,
};
use tokio::{task::JoinHandle, time::sleep};

/// The maximum number of deployments to verify in parallel.
/// Note: worst case memory to verify a deployment (MAX_DEPLOYMENT_CONSTRAINTS = 1 << 20) is ~2 GiB.
const MAX_PARALLEL_DEPLOY_VERIFICATIONS: usize = 5;
/// The maximum number of executions to verify in parallel.
/// Note: worst case memory to verify an execution is 0.01 GiB.
const MAX_PARALLEL_EXECUTE_VERIFICATIONS: usize = 1000;
/// The maximum number of solutions to verify in parallel.
/// Note: worst case memory to verify a solution is 0.5 GiB.
const MAX_PARALLEL_SOLUTION_VERIFICATIONS: usize = 20;
/// The capacity for storing unconfirmed deployments.
/// Note: This is an inbound queue capacity, not a Narwhal-enforced capacity.
const CAPACITY_FOR_DEPLOYMENTS: usize = 1 << 10;
/// The capacity for storing unconfirmed executions.
/// Note: This is an inbound queue capacity, not a Narwhal-enforced capacity.
const CAPACITY_FOR_EXECUTIONS: usize = 1 << 10;
/// The capacity for storing unconfirmed solutions.
/// Note: This is an inbound queue capacity, not a Narwhal-enforced capacity.
const CAPACITY_FOR_SOLUTIONS: usize = 1 << 10;

/// Transaction details needed for propagation.
/// We preserve the serialized transaction for faster propagation.
type TransactionContents<N> = (SocketAddr, UnconfirmedTransaction<N>, Transaction<N>);
/// Solution details needed for propagation.
/// We preserve the serialized solution for faster propagation.
type SolutionContents<N> = (SocketAddr, UnconfirmedSolution<N>, Solution<N>);

/// A client node is a full node, capable of querying with the network.
#[derive(Clone)]
pub struct Client<N: Network, C: ConsensusStorage<N>> {
    /// The ledger of the node.
    ledger: Ledger<N, C>,
    /// The router of the node.
    router: Router<N>,
    /// The REST server of the node.
    rest: Option<Rest<N, C, Self>>,
    /// The sync module.
    sync: Arc<BlockSync<N>>,
    /// The genesis block.
    genesis: Block<N>,
    /// The puzzle.
    puzzle: Puzzle<N>,
    /// The unconfirmed solutions queue.
    solution_queue: Arc<Mutex<LruCache<SolutionID<N>, SolutionContents<N>>>>,
    /// The unconfirmed deployments queue.
    deploy_queue: Arc<Mutex<LruCache<N::TransactionID, TransactionContents<N>>>>,
    /// The unconfirmed executions queue.
    execute_queue: Arc<Mutex<LruCache<N::TransactionID, TransactionContents<N>>>>,
    /// The amount of solutions currently being verified.
    num_verifying_solutions: Arc<AtomicUsize>,
    /// The amount of deployments currently being verified.
    num_verifying_deploys: Arc<AtomicUsize>,
    /// The amount of executions currently being verified.
    num_verifying_executions: Arc<AtomicUsize>,
    /// The spawned handles.
    handles: Arc<Mutex<Vec<JoinHandle<()>>>>,
    /// The shutdown signal.
    shutdown: Arc<AtomicBool>,
}

impl<N: Network, C: ConsensusStorage<N>> Client<N, C> {
    /// Initializes a new client node.
    pub async fn new(
        node_ip: SocketAddr,
        rest_ip: Option<SocketAddr>,
        rest_rps: u32,
        account: Account<N>,
        trusted_peers: &[SocketAddr],
        genesis: Block<N>,
        cdn: Option<String>,
        storage_mode: StorageMode,
        shutdown: Arc<AtomicBool>,
    ) -> Result<Self> {
        // Initialize the signal handler.
        let signal_node = Self::handle_signals(shutdown.clone());

        // Initialize the ledger.
        let ledger = Ledger::<N, C>::load(genesis.clone(), storage_mode.clone())?;

        // Initialize the CDN.
        if let Some(base_url) = cdn {
            // Sync the ledger with the CDN.
            if let Err((_, error)) =
                snarkos_node_cdn::sync_ledger_with_cdn(&base_url, ledger.clone(), shutdown.clone()).await
            {
                crate::log_clean_error(&storage_mode);
                return Err(error);
            }
        }

        // Initialize the ledger service.
        let ledger_service = Arc::new(CoreLedgerService::<N, C>::new(ledger.clone(), shutdown.clone()));
        // Initialize the sync module.
        let sync = BlockSync::new(BlockSyncMode::Router, ledger_service.clone());
        // Determine if the client should allow external peers.
        let allow_external_peers = true;

        // Initialize the node router.
        let router = Router::new(
            node_ip,
            NodeType::Client,
            account,
            trusted_peers,
            Self::MAXIMUM_NUMBER_OF_PEERS as u16,
            allow_external_peers,
            matches!(storage_mode, StorageMode::Development(_)),
        )
        .await?;
        // Initialize the node.
        let mut node = Self {
            ledger: ledger.clone(),
            router,
            rest: None,
            sync: Arc::new(sync),
            genesis,
            puzzle: ledger.puzzle().clone(),
            solution_queue: Arc::new(Mutex::new(LruCache::new(NonZeroUsize::new(CAPACITY_FOR_SOLUTIONS).unwrap()))),
            deploy_queue: Arc::new(Mutex::new(LruCache::new(NonZeroUsize::new(CAPACITY_FOR_DEPLOYMENTS).unwrap()))),
            execute_queue: Arc::new(Mutex::new(LruCache::new(NonZeroUsize::new(CAPACITY_FOR_EXECUTIONS).unwrap()))),
            num_verifying_solutions: Default::default(),
            num_verifying_deploys: Default::default(),
            num_verifying_executions: Default::default(),
            handles: Default::default(),
            shutdown,
        };

        // Initialize the REST server.
        if let Some(rest_ip) = rest_ip {
            node.rest = Some(Rest::start(rest_ip, rest_rps, None, ledger.clone(), Arc::new(node.clone())).await?);
        }
        // Initialize the routing.
        node.initialize_routing().await;
        // Initialize the sync module.
        node.initialize_sync();
        // Initialize solution verification.
        node.initialize_solution_verification();
        // Initialize deployment verification.
        node.initialize_deploy_verification();
        // Initialize execution verification.
        node.initialize_execute_verification();
        // Initialize the notification message loop.
        node.handles.lock().push(crate::start_notification_message_loop());
        // Pass the node to the signal handler.
        let _ = signal_node.set(node.clone());
        // Return the node.
        Ok(node)
    }

    /// Returns the ledger.
    pub fn ledger(&self) -> &Ledger<N, C> {
        &self.ledger
    }

    /// Returns the REST server.
    pub fn rest(&self) -> &Option<Rest<N, C, Self>> {
        &self.rest
    }
}

impl<N: Network, C: ConsensusStorage<N>> Client<N, C> {
    /// Initializes the sync pool.
    fn initialize_sync(&self) {
        // Start the sync loop.
        let node = self.clone();
        self.handles.lock().push(tokio::spawn(async move {
            loop {
                // If the Ctrl-C handler registered the signal, stop the node.
                if node.shutdown.load(std::sync::atomic::Ordering::Relaxed) {
                    info!("Shutting down block production");
                    break;
                }

                // Sleep briefly to avoid triggering spam detection.
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                // Perform the sync routine.
                node.sync.try_block_sync(&node).await;
            }
        }));
    }

    /// Initializes solution verification.
    fn initialize_solution_verification(&self) {
        // Start the solution verification loop.
        let node = self.clone();
        self.handles.lock().push(tokio::spawn(async move {
            loop {
                // If the Ctrl-C handler registered the signal, stop the node.
                if node.shutdown.load(Acquire) {
                    info!("Shutting down solution verification");
                    break;
                }

                // Determine if the queue contains txs to verify.
                let queue_is_empty = node.solution_queue.lock().is_empty();
                // Determine if our verification counter has space to verify new solutions.
                let counter_is_full = node.num_verifying_solutions.load(Acquire) >= MAX_PARALLEL_SOLUTION_VERIFICATIONS;

                // Sleep to allow the queue to be filled or solutions to be validated.
                if queue_is_empty || counter_is_full {
                    sleep(Duration::from_millis(50)).await;
                    continue;
                }

                // Try to verify solutions.
                let mut solution_queue = node.solution_queue.lock();
                while let Some((_, (peer_ip, serialized, solution))) = solution_queue.pop_lru() {
                    // Increment the verification counter.
                    let previous_counter = node.num_verifying_solutions.fetch_add(1, Relaxed);
                    let _node = node.clone();
                    // For each solution, spawn a task to verify it.
                    tokio::spawn(async move {
                        // Retrieve the latest epoch hash.
                        if let Ok(epoch_hash) = _node.ledger.latest_epoch_hash() {
                            // Retrieve the latest proof target.
                            let proof_target = _node.ledger.latest_block().header().proof_target();
                            // Ensure that the solution is valid for the given epoch.
                            let puzzle = _node.puzzle.clone();
                            let is_valid = tokio::task::spawn_blocking(move || {
                                puzzle.check_solution(&solution, epoch_hash, proof_target)
                            })
                            .await;

                            match is_valid {
                                // If the solution is valid, propagate the `UnconfirmedSolution`.
                                Ok(Ok(())) => {
                                    let message = Message::UnconfirmedSolution(serialized);
                                    // Propagate the "UnconfirmedSolution".
                                    _node.propagate(message, &[peer_ip]);
                                }
                                Ok(Err(_)) => {
                                    trace!("Invalid solution '{}' for the proof target.", solution.id())
                                }
                                // If error occurs after the first 10 blocks of the epoch, log it as a warning, otherwise ignore.
                                Err(error) => {
                                    if _node.ledger.latest_height() % N::NUM_BLOCKS_PER_EPOCH > 10 {
                                        warn!("Failed to verify the solution - {error}")
                                    }
                                }
                            }
                        } else {
                            warn!("Failed to retrieve the latest epoch hash.");
                        }
                        // Decrement the verification counter.
                        _node.num_verifying_solutions.fetch_sub(1, Relaxed);
                    });
                    // If we are already at capacity, don't verify more solutions.
                    if previous_counter + 1 >= MAX_PARALLEL_SOLUTION_VERIFICATIONS {
                        break;
                    }
                }
            }
        }));
    }

    /// Initializes deploy verification.
    fn initialize_deploy_verification(&self) {
        // Start the deploy verification loop.
        let node = self.clone();
        self.handles.lock().push(tokio::spawn(async move {
            loop {
                // If the Ctrl-C handler registered the signal, stop the node.
                if node.shutdown.load(Acquire) {
                    info!("Shutting down deployment verification");
                    break;
                }

                // Determine if the queue contains txs to verify.
                let queue_is_empty = node.deploy_queue.lock().is_empty();
                // Determine if our verification counter has space to verify new txs.
                let counter_is_full = node.num_verifying_deploys.load(Acquire) >= MAX_PARALLEL_DEPLOY_VERIFICATIONS;

                // Sleep to allow the queue to be filled or transactions to be validated.
                if queue_is_empty || counter_is_full {
                    sleep(Duration::from_millis(50)).await;
                    continue;
                }

                // Try to verify deployments.
                while let Some((_, (peer_ip, serialized, transaction))) = node.deploy_queue.lock().pop_lru() {
                    // Increment the verification counter.
                    let previous_counter = node.num_verifying_deploys.fetch_add(1, Relaxed);
                    let _node = node.clone();
                    // For each deployment, spawn a task to verify it.
                    tokio::spawn(async move {
                        // Check the deployment.
                        if _node.ledger.check_transaction_basic(&transaction, None, &mut rand::thread_rng()).is_ok() {
                            // Propagate the `UnconfirmedTransaction`.
                            _node.propagate(Message::UnconfirmedTransaction(serialized), &[peer_ip]);
                        }
                        // Decrement the verification counter.
                        _node.num_verifying_deploys.fetch_sub(1, Relaxed);
                    });
                    // If we are already at capacity, don't verify more deployments.
                    if previous_counter + 1 >= MAX_PARALLEL_DEPLOY_VERIFICATIONS {
                        break;
                    }
                }
            }
        }));
    }

    /// Initializes execute verification.
    fn initialize_execute_verification(&self) {
        // Start the execute verification loop.
        let node = self.clone();
        self.handles.lock().push(tokio::spawn(async move {
            loop {
                // If the Ctrl-C handler registered the signal, stop the node.
                if node.shutdown.load(Acquire) {
                    info!("Shutting down execution verification");
                    break;
                }

                // Determine if the queue contains txs to verify.
                let queue_is_empty = node.execute_queue.lock().is_empty();
                // Determine if our verification counter has space to verify new txs.
                let counter_is_full = node.num_verifying_executions.load(Acquire) >= MAX_PARALLEL_EXECUTE_VERIFICATIONS;

                // Sleep to allow the queue to be filled or transactions to be validated.
                if queue_is_empty || counter_is_full {
                    sleep(Duration::from_millis(50)).await;
                    continue;
                }

                // Try to verify executions.
                while let Some((_, (peer_ip, serialized, transaction))) = node.execute_queue.lock().pop_lru() {
                    // Increment the verification counter.
                    let previous_counter = node.num_verifying_executions.fetch_add(1, Relaxed);
                    let _node = node.clone();
                    // For each execution, spawn a task to verify it.
                    tokio::spawn(async move {
                        // Check the execution.
                        if _node.ledger.check_transaction_basic(&transaction, None, &mut rand::thread_rng()).is_ok() {
                            // Propagate the `UnconfirmedTransaction`.
                            _node.propagate(Message::UnconfirmedTransaction(serialized), &[peer_ip]);
                        }
                        // Decrement the verification counter.
                        _node.num_verifying_executions.fetch_sub(1, Relaxed);
                    });
                    // If we are already at capacity, don't verify more executions.
                    if previous_counter + 1 >= MAX_PARALLEL_EXECUTE_VERIFICATIONS {
                        break;
                    }
                }
            }
        }));
    }

    /// Spawns a task with the given future; it should only be used for long-running tasks.
    pub fn spawn<T: Future<Output = ()> + Send + 'static>(&self, future: T) {
        self.handles.lock().push(tokio::spawn(future));
    }
}

#[async_trait]
impl<N: Network, C: ConsensusStorage<N>> NodeInterface<N> for Client<N, C> {
    /// Shuts down the node.
    async fn shut_down(&self) {
        info!("Shutting down...");

        // Shut down the node.
        trace!("Shutting down the node...");
        self.shutdown.store(true, std::sync::atomic::Ordering::Relaxed);

        // Abort the tasks.
        trace!("Shutting down the validator...");
        self.handles.lock().iter().for_each(|handle| handle.abort());

        // Shut down the router.
        self.router.shut_down().await;

        info!("Node has shut down.");
    }
}
