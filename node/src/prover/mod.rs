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

mod router;

use crate::traits::NodeInterface;
use snarkos_account::Account;
use snarkos_node_bft::ledger_service::ProverLedgerService;
use snarkos_node_router::{
    messages::{Message, NodeType, UnconfirmedSolution},
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
    ledger::narwhal::Data,
    prelude::{
        block::{Block, Header},
        puzzle::{Puzzle, Solution},
        store::ConsensusStorage,
        Network,
    },
    synthesizer::VM,
};

use aleo_std::StorageMode;
use anyhow::Result;
use colored::Colorize;
use core::{marker::PhantomData, time::Duration};
use parking_lot::{Mutex, RwLock};
use rand::{rngs::OsRng, CryptoRng, Rng};
use snarkos_node_bft::helpers::fmt_id;
use std::{
    net::SocketAddr,
    sync::{
        atomic::{AtomicBool, AtomicU8, Ordering},
        Arc,
    },
};
use tokio::task::JoinHandle;

/// A prover is a light node, capable of producing proofs for consensus.
#[derive(Clone)]
pub struct Prover<N: Network, C: ConsensusStorage<N>> {
    /// The router of the node.
    router: Router<N>,
    /// The sync module.
    sync: Arc<BlockSync<N>>,
    /// The genesis block.
    genesis: Block<N>,
    /// The puzzle.
    puzzle: Puzzle<N>,
    /// The latest epoch hash.
    latest_epoch_hash: Arc<RwLock<Option<N::BlockHash>>>,
    /// The latest block header.
    latest_block_header: Arc<RwLock<Option<Header<N>>>>,
    /// The number of puzzle instances.
    puzzle_instances: Arc<AtomicU8>,
    /// The maximum number of puzzle instances.
    max_puzzle_instances: u8,
    /// The spawned handles.
    handles: Arc<Mutex<Vec<JoinHandle<()>>>>,
    /// The shutdown signal.
    shutdown: Arc<AtomicBool>,
    /// PhantomData.
    _phantom: PhantomData<C>,
}

impl<N: Network, C: ConsensusStorage<N>> Prover<N, C> {
    /// Initializes a new prover node.
    pub async fn new(
        node_ip: SocketAddr,
        account: Account<N>,
        trusted_peers: &[SocketAddr],
        genesis: Block<N>,
        storage_mode: StorageMode,
        shutdown: Arc<AtomicBool>,
    ) -> Result<Self> {
        // Initialize the signal handler.
        let signal_node = Self::handle_signals(shutdown.clone());

        // Initialize the ledger service.
        let ledger_service = Arc::new(ProverLedgerService::new());
        // Initialize the sync module.
        let sync = BlockSync::new(BlockSyncMode::Router, ledger_service.clone());
        // Determine if the prover should allow external peers.
        let allow_external_peers = true;

        // Initialize the node router.
        let router = Router::new(
            node_ip,
            NodeType::Prover,
            account,
            trusted_peers,
            Self::MAXIMUM_NUMBER_OF_PEERS as u16,
            allow_external_peers,
            matches!(storage_mode, StorageMode::Development(_)),
        )
        .await?;
        // Compute the maximum number of puzzle instances.
        let max_puzzle_instances = num_cpus::get().saturating_sub(2).clamp(1, 6);
        // Initialize the node.
        let node = Self {
            router,
            sync: Arc::new(sync),
            genesis,
            puzzle: VM::<N, C>::new_puzzle()?,
            latest_epoch_hash: Default::default(),
            latest_block_header: Default::default(),
            puzzle_instances: Default::default(),
            max_puzzle_instances: u8::try_from(max_puzzle_instances)?,
            handles: Default::default(),
            shutdown,
            _phantom: Default::default(),
        };
        // Initialize the routing.
        node.initialize_routing().await;
        // Initialize the puzzle.
        node.initialize_puzzle().await;
        // Initialize the notification message loop.
        node.handles.lock().push(crate::start_notification_message_loop());
        // Pass the node to the signal handler.
        let _ = signal_node.set(node.clone());
        // Return the node.
        Ok(node)
    }
}

#[async_trait]
impl<N: Network, C: ConsensusStorage<N>> NodeInterface<N> for Prover<N, C> {
    /// Shuts down the node.
    async fn shut_down(&self) {
        info!("Shutting down...");

        // Shut down the puzzle.
        debug!("Shutting down the puzzle...");
        self.shutdown.store(true, Ordering::Relaxed);

        // Abort the tasks.
        debug!("Shutting down the prover...");
        self.handles.lock().iter().for_each(|handle| handle.abort());

        // Shut down the router.
        self.router.shut_down().await;

        info!("Node has shut down.");
    }
}

impl<N: Network, C: ConsensusStorage<N>> Prover<N, C> {
    /// Initialize a new instance of the puzzle.
    async fn initialize_puzzle(&self) {
        for _ in 0..self.max_puzzle_instances {
            let prover = self.clone();
            self.handles.lock().push(tokio::spawn(async move {
                prover.puzzle_loop().await;
            }));
        }
    }

    /// Executes an instance of the puzzle.
    async fn puzzle_loop(&self) {
        loop {
            // If the node is not connected to any peers, then skip this iteration.
            if self.router.number_of_connected_peers() == 0 {
                debug!("Skipping an iteration of the puzzle (no connected peers)");
                tokio::time::sleep(Duration::from_secs(N::ANCHOR_TIME as u64)).await;
                continue;
            }

            // If the number of instances of the puzzle exceeds the maximum, then skip this iteration.
            if self.num_puzzle_instances() > self.max_puzzle_instances {
                // Sleep for a brief period of time.
                tokio::time::sleep(Duration::from_millis(500)).await;
                continue;
            }

            // Read the latest epoch hash.
            let latest_epoch_hash = *self.latest_epoch_hash.read();
            // Read the latest state.
            let latest_state = self
                .latest_block_header
                .read()
                .as_ref()
                .map(|header| (header.coinbase_target(), header.proof_target()));

            // If the latest epoch hash and latest state exists, then proceed to generate a solution.
            if let (Some(epoch_hash), Some((coinbase_target, proof_target))) = (latest_epoch_hash, latest_state) {
                // Execute the puzzle.
                let prover = self.clone();
                let result = tokio::task::spawn_blocking(move || {
                    prover.puzzle_iteration(epoch_hash, coinbase_target, proof_target, &mut OsRng)
                })
                .await;

                // If the prover found a solution, then broadcast it.
                if let Ok(Some((solution_target, solution))) = result {
                    info!("Found a Solution '{}' (Proof Target {solution_target})", solution.id());
                    // Broadcast the solution.
                    self.broadcast_solution(solution);
                }
            } else {
                // Otherwise, sleep for a brief period of time, to await for puzzle state.
                tokio::time::sleep(Duration::from_secs(1)).await;
            }

            // If the Ctrl-C handler registered the signal, stop the prover.
            if self.shutdown.load(Ordering::Relaxed) {
                debug!("Shutting down the puzzle...");
                break;
            }
        }
    }

    /// Performs one iteration of the puzzle.
    fn puzzle_iteration<R: Rng + CryptoRng>(
        &self,
        epoch_hash: N::BlockHash,
        coinbase_target: u64,
        proof_target: u64,
        rng: &mut R,
    ) -> Option<(u64, Solution<N>)> {
        // Increment the puzzle instances.
        self.increment_puzzle_instances();

        debug!(
            "Proving 'Puzzle' for Epoch '{}' {}",
            fmt_id(epoch_hash),
            format!("(Coinbase Target {coinbase_target}, Proof Target {proof_target})").dimmed()
        );

        // Compute the solution.
        let result =
            self.puzzle.prove(epoch_hash, self.address(), rng.gen(), Some(proof_target)).ok().and_then(|solution| {
                self.puzzle.get_proof_target(&solution).ok().map(|solution_target| (solution_target, solution))
            });

        // Decrement the puzzle instances.
        self.decrement_puzzle_instances();
        // Return the result.
        result
    }

    /// Broadcasts the solution to the network.
    fn broadcast_solution(&self, solution: Solution<N>) {
        // Prepare the unconfirmed solution message.
        let message = Message::UnconfirmedSolution(UnconfirmedSolution {
            solution_id: solution.id(),
            solution: Data::Object(solution),
        });
        // Propagate the "UnconfirmedSolution".
        self.propagate(message, &[]);
    }

    /// Returns the current number of puzzle instances.
    fn num_puzzle_instances(&self) -> u8 {
        self.puzzle_instances.load(Ordering::Relaxed)
    }

    /// Increments the number of puzzle instances.
    fn increment_puzzle_instances(&self) {
        self.puzzle_instances.fetch_add(1, Ordering::Relaxed);
        #[cfg(debug_assertions)]
        trace!("Number of Instances - {}", self.num_puzzle_instances());
    }

    /// Decrements the number of puzzle instances.
    fn decrement_puzzle_instances(&self) {
        self.puzzle_instances.fetch_sub(1, Ordering::Relaxed);
        #[cfg(debug_assertions)]
        trace!("Number of Instances - {}", self.num_puzzle_instances());
    }
}
