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

use anyhow::Result;
use core::{marker::PhantomData, time::Duration};
use parking_lot::RwLock;
use rand::Rng;
use std::{
    net::SocketAddr,
    sync::{
        atomic::{AtomicU8, Ordering},
        Arc,
    },
};
use time::OffsetDateTime;
use tokio::task::JoinHandle;

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
    /// The spawned handles.
    handles: Arc<RwLock<Vec<JoinHandle<()>>>>,
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
        // Initialize the node.
        let node = Self {
            account,
            router,
            coinbase_puzzle,
            latest_epoch_challenge: Default::default(),
            latest_block: Default::default(),
            puzzle_instances: Default::default(),
            handles: Default::default(),
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
    /// The node type.
    const NODE_TYPE: NodeType = NodeType::Prover;

    /// Disconnects from peers and shuts down the node.
    async fn shut_down(&self) {
        // Update the node status.
        info!("Shutting down...");
        Self::status().update(Status::ShuttingDown);

        // Abort the tasks.
        trace!("Shutting down the prover...");
        self.handles.read().iter().for_each(|handle| handle.abort());

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
        Self::NODE_TYPE
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
    /// The maximum number of puzzle instances at any given time.
    const MAXIMUM_PUZZLE_INSTANCES: u8 = 4;

    /// Initialize a new instance of the coinbase puzzle.
    async fn initialize_coinbase_puzzle(&self) {
        let prover = self.clone();
        self.handles.write().push(tokio::spawn(async move {
            loop {
                // If the node is not connected to any peers, then skip this iteration.
                if prover.router.number_of_connected_peers() == 0 {
                    warn!("Skipping an iteration of the prover solution (no connected peers)");
                    tokio::time::sleep(Duration::from_secs(N::ANCHOR_TIME as u64)).await;
                    continue;
                }

                // If the number of instances of the coinbase puzzle exceeds the maximum, then skip this iteration.
                if prover.puzzle_instances.load(Ordering::SeqCst) >= Self::MAXIMUM_PUZZLE_INSTANCES {
                    // Sleep for `N::ANCHOR_TIME` seconds.
                    tokio::time::sleep(Duration::from_secs(N::ANCHOR_TIME as u64)).await;
                    continue;
                }

                // If the latest block timestamp exceeds a multiple of the anchor time, then skip this iteration.
                let latest_timestamp = prover.latest_block.read().as_ref().map(|block| block.timestamp());
                if let Some(latest_timestamp) = latest_timestamp {
                    // Compute the elapsed time since the latest block.
                    let elapsed = OffsetDateTime::now_utc().unix_timestamp().saturating_sub(latest_timestamp);
                    // If the elapsed time exceeds a multiple of the anchor time, then skip this iteration.
                    if elapsed > N::ANCHOR_TIME as i64 * 6 {
                        warn!("Skipping an iteration of the prover solution (latest block is stale)");
                        // Send a "PuzzleRequest" to a beacon node.
                        prover.send_puzzle_request();
                        // Sleep for `N::ANCHOR_TIME` seconds.
                        tokio::time::sleep(Duration::from_secs(N::ANCHOR_TIME as u64)).await;
                        continue;
                    }
                }

                // Execute the coinbase puzzle.
                let prover = prover.clone();
                tokio::spawn(async move { prover.coinbase_puzzle_loop().await });
                // Sleep briefly to give this instance a chance to clear state.
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        }));
    }

    /// Executes an instance of the coinbase puzzle.
    async fn coinbase_puzzle_loop(&self) {
        // Set the status to `Proving`.
        Self::status().update(Status::Proving);
        // Increment the number of puzzle instances.
        self.puzzle_instances.fetch_add(1, Ordering::SeqCst);
        // Iterate until a prover solution is found.
        loop {
            // Perform one iteration of the coinbase puzzle.
            if let Some((prover_solution_target, prover_solution)) = self.coinbase_puzzle_iteration().await {
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

                // Terminate the loop.
                break;
            }
        }
        // Set the status to `Ready`.
        Self::status().update(Status::Ready);
        // Decrement the number of puzzle instances.
        self.puzzle_instances.fetch_sub(1, Ordering::SeqCst);
    }

    /// Performs one iteration of the coinbase puzzle.
    async fn coinbase_puzzle_iteration(&self) -> Option<(u64, ProverSolution<N>)> {
        // Read the latest epoch challenge.
        let latest_epoch_challenge = self.latest_epoch_challenge.read().clone();
        // Read the latest block.
        let latest_block = self.latest_block.read().clone();

        // If the latest epoch challenge and latest block exists, then generate a prover solution.
        if let (Some(epoch_challenge), Some(block)) = (latest_epoch_challenge, latest_block) {
            debug!(
                "Proving 'CoinbasePuzzle' (Epoch {}, Coinbase Target {}, Proof Target {})",
                epoch_challenge.epoch_number(),
                block.coinbase_target(),
                block.proof_target(),
            );

            // Compute the prover solution.
            match self.coinbase_puzzle.prove(
                &epoch_challenge,
                self.address(),
                rand::thread_rng().gen(),
                Some(block.proof_target()),
            ) {
                Ok(solution) => solution.to_target().ok().map(|solution_target| (solution_target, solution)),
                _ => None,
            }
        } else {
            tokio::time::sleep(Duration::from_secs(1)).await;
            None
        }
    }
}
