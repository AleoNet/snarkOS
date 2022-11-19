// Copyright (C) 2019-2022 Aleo Systems Inc.
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

mod router;

use crate::traits::NodeInterface;
use snarkos_account::Account;
use snarkos_node_executor::{Executor, NodeType, Status};
use snarkos_node_messages::{Data, Message, PuzzleResponse, UnconfirmedSolution};
use snarkos_node_router::{Heartbeat, Inbound, Outbound, Router, Routing};
use snarkos_node_tcp::{
    protocols::{Disconnect, Handshake, Reading, Writing},
    P2P,
};
use snarkvm::prelude::{
    Address,
    Block,
    CoinbasePuzzle,
    ConsensusStorage,
    EpochChallenge,
    Network,
    PrivateKey,
    ProverSolution,
    ViewKey,
};

use anyhow::{bail, Result};
use colored::Colorize;
use core::{marker::PhantomData, time::Duration};
use rand::Rng;
use std::{
    net::SocketAddr,
    sync::{
        atomic::{AtomicBool, AtomicU8, Ordering},
        Arc,
    },
};
use time::OffsetDateTime;
use tokio::{sync::RwLock, task::JoinHandle};

/// A prover is a full node, capable of producing proofs for consensus.
#[derive(Clone)]
pub struct Prover<N: Network, C: ConsensusStorage<N>> {
    /// The account of the node.
    account: Account<N>,
    /// The router of the node.
    router: Router<N>,
    /// The coinbase puzzle.
    coinbase_puzzle: CoinbasePuzzle<N>,
    /// The latest epoch challenge.
    latest_epoch_challenge: Arc<RwLock<Option<EpochChallenge<N>>>>,
    /// The latest block.
    latest_block: Arc<RwLock<Option<Block<N>>>>,
    /// The number of puzzle instances.
    puzzle_instances: Arc<AtomicU8>,
    /// The maximum number of puzzle instances.
    max_puzzle_instances: u8,
    /// The spawned handles.
    handles: Arc<RwLock<Vec<JoinHandle<()>>>>,
    /// The shutdown signal.
    shutdown: Arc<AtomicBool>,
    /// PhantomData.
    _phantom: PhantomData<C>,
}

impl<N: Network, C: ConsensusStorage<N>> Prover<N, C> {
    /// Initializes a new prover node.
    pub async fn new(
        node_ip: SocketAddr,
        private_key: PrivateKey<N>,
        trusted_peers: &[SocketAddr],
        dev: Option<u16>,
    ) -> Result<Self> {
        // Initialize the node account.
        let account = Account::from(private_key)?;
        // Initialize the node router.
        let router = Router::new(
            node_ip,
            NodeType::Prover,
            account.address(),
            trusted_peers,
            Self::MAXIMUM_NUMBER_OF_PEERS as u16,
            dev.is_some(),
        )
        .await?;
        // Load the coinbase puzzle.
        let coinbase_puzzle = CoinbasePuzzle::<N>::load()?;
        // Compute the maximum number of puzzle instances.
        let max_puzzle_instances = num_cpus::get().saturating_sub(3).min(8).max(1);
        // Initialize the node.
        let node = Self {
            account,
            router,
            coinbase_puzzle,
            latest_epoch_challenge: Default::default(),
            latest_block: Default::default(),
            puzzle_instances: Default::default(),
            max_puzzle_instances: u8::try_from(max_puzzle_instances)?,
            handles: Default::default(),
            shutdown: Default::default(),
            _phantom: Default::default(),
        };
        // Initialize the routing.
        node.initialize_routing().await;
        // Initialize the coinbase puzzle.
        node.initialize_coinbase_puzzle().await;
        // Initialize the signal handler.
        node.handle_signals();
        // Return the node.
        Ok(node)
    }
}

#[async_trait]
impl<N: Network, C: ConsensusStorage<N>> Executor for Prover<N, C> {
    /// Disconnects from peers and shuts down the node.
    async fn shut_down(&self) {
        // Update the node status.
        info!("Shutting down...");
        Self::status().update(Status::ShuttingDown);

        // Shut down the coinbase puzzle.
        trace!("Shutting down the coinbase puzzle...");
        self.shutdown.store(true, Ordering::SeqCst);

        // Abort the tasks.
        trace!("Shutting down the prover...");
        self.handles.read().await.iter().for_each(|handle| handle.abort());

        // Shut down the router.
        trace!("Shutting down the router...");
        self.router.shut_down().await;

        // Flush the tasks.
        Self::resources().shut_down();
        trace!("Node has shut down.");
    }
}

impl<N: Network, C: ConsensusStorage<N>> NodeInterface<N> for Prover<N, C> {
    /// Returns the node type.
    fn node_type(&self) -> NodeType {
        self.router.node_type()
    }

    /// Returns the account private key of the node.
    fn private_key(&self) -> &PrivateKey<N> {
        self.account.private_key()
    }

    /// Returns the account view key of the node.
    fn view_key(&self) -> &ViewKey<N> {
        self.account.view_key()
    }

    /// Returns the account address of the node.
    fn address(&self) -> Address<N> {
        self.account.address()
    }

    /// Returns `true` if the node is in development mode.
    fn is_dev(&self) -> bool {
        self.router.is_dev()
    }
}

impl<N: Network, C: ConsensusStorage<N>> Prover<N, C> {
    /// Initialize a new instance of the coinbase puzzle.
    async fn initialize_coinbase_puzzle(&self) {
        let prover = self.clone();
        self.handles.write().await.push(tokio::spawn(async move {
            prover.coinbase_puzzle_loop().await;
        }));
    }

    /// Executes an instance of the coinbase puzzle.
    async fn coinbase_puzzle_loop(&self) {
        loop {
            // If the node is not connected to any peers, then skip this iteration.
            if self.router.number_of_connected_peers() == 0 {
                warn!("Skipping an iteration of the coinbase puzzle (no connected peers)");
                tokio::time::sleep(Duration::from_secs(N::ANCHOR_TIME as u64)).await;
                continue;
            }

            // If the number of instances of the coinbase puzzle exceeds the maximum, then skip this iteration.
            if self.puzzle_instances.load(Ordering::SeqCst) >= self.max_puzzle_instances {
                // Sleep for a brief period of time.
                tokio::time::sleep(Duration::from_millis(500)).await;
                continue;
            }

            // Read the latest epoch challenge.
            let latest_epoch_challenge = self.latest_epoch_challenge.read().await.clone();
            // Read the latest block.
            let latest_block = self.latest_block.read().await.clone();

            // If the latest block timestamp exceeds a multiple of the anchor time, then skip this iteration.
            if let Some(latest_timestamp) = latest_block.as_ref().map(|block| block.timestamp()) {
                // Compute the elapsed time since the latest block.
                let elapsed = OffsetDateTime::now_utc().unix_timestamp().saturating_sub(latest_timestamp);
                // If the elapsed time exceeds a multiple of the anchor time, then skip this iteration.
                if elapsed > N::ANCHOR_TIME as i64 * 6 {
                    warn!("Skipping an iteration of the coinbase puzzle (latest block is stale)");
                    // Send a "PuzzleRequest" to a beacon node.
                    self.send_puzzle_request();
                    // Sleep for `N::ANCHOR_TIME` seconds.
                    tokio::time::sleep(Duration::from_secs(N::ANCHOR_TIME as u64)).await;
                    continue;
                }
            }

            // If the latest epoch challenge and latest block exists, then proceed to generate a prover solution.
            if let (Some(challenge), Some(block)) = (latest_epoch_challenge, latest_block) {
                // Execute the coinbase puzzle.
                let prover = self.clone();
                match tokio::task::spawn_blocking(move || prover.coinbase_puzzle_iteration(challenge, block)).await {
                    Ok(_) => (),
                    Err(error) => error!("Failed to generate a prover solution: {error}"),
                }
            } else {
                // Otherwise, sleep for a brief period of time, to await for puzzle state.
                tokio::time::sleep(Duration::from_secs(1)).await;
            }

            // If the Ctrl-C handler registered the signal, stop the prover.
            if self.shutdown.load(Ordering::Relaxed) {
                trace!("Shutting down the coinbase puzzle...");
                break;
            }
        }
    }

    /// Performs one iteration of the coinbase puzzle.
    fn coinbase_puzzle_iteration(
        &self,
        epoch_challenge: EpochChallenge<N>,
        block: Block<N>,
    ) -> Option<(u64, ProverSolution<N>)> {
        // Set the status to `Proving`.
        Self::status().update(Status::Proving);
        // Increment the number of puzzle instances.
        self.puzzle_instances.fetch_add(1, Ordering::SeqCst);

        trace!(
            "Proving 'CoinbasePuzzle' {}",
            format!(
                "(Epoch {}, Coinbase Target {}, Proof Target {})",
                epoch_challenge.epoch_number(),
                block.coinbase_target(),
                block.proof_target()
            )
            .dimmed()
        );

        info!("\n\nNUM INSTANCES - {}\n\n", self.puzzle_instances.load(std::sync::atomic::Ordering::SeqCst));
        // Compute the prover solution.
        let result = self
            .coinbase_puzzle
            .prove(&epoch_challenge, self.address(), rand::thread_rng().gen(), Some(block.proof_target()))
            .ok()
            .and_then(|solution| solution.to_target().ok().map(|target| (target, solution)));

        // Set the status to `Ready`.
        Self::status().update(Status::Ready);
        // Decrement the number of puzzle instances.
        self.puzzle_instances.fetch_sub(1, Ordering::SeqCst);

        // If the prover found a solution, then broadcast it.
        if let Some((prover_solution_target, prover_solution)) = result {
            info!("Found a Solution '{}' (Proof Target {prover_solution_target})", prover_solution.commitment());

            // Broadcast the prover solution.
            let prover = self.clone();
            tokio::spawn(async move {
                // Prepare the unconfirmed solution message.
                let message = Message::UnconfirmedSolution(UnconfirmedSolution {
                    puzzle_commitment: prover_solution.commitment(),
                    solution: Data::Object(prover_solution),
                });
                // Propagate the "UnconfirmedSolution" to the network.
                prover.propagate(message, vec![]);
            });
        }

        // Return the result.
        result
    }
}
