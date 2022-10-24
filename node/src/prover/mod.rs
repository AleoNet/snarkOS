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
use snarkos_node_executor::{spawn_task, Executor, NodeType, Status};
use snarkos_node_messages::{Data, Message, PuzzleResponse, UnconfirmedSolution};
use snarkos_node_router::{Handshake, Inbound, Outbound, Router, RouterRequest};
use snarkvm::prelude::{Address, Block, CoinbasePuzzle, EpochChallenge, Network, PrivateKey, ViewKey};

use anyhow::Result;
use rand::Rng;
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::RwLock;

/// A prover is a full node, capable of producing proofs for consensus.
#[derive(Clone)]
pub struct Prover<N: Network> {
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
}

impl<N: Network> Prover<N> {
    /// Initializes a new prover node.
    pub async fn new(node_ip: SocketAddr, private_key: PrivateKey<N>, trusted_peers: &[SocketAddr]) -> Result<Self> {
        // Initialize the node account.
        let account = Account::from(private_key)?;
        // Initialize the node router.
        let (router, router_receiver) = Router::new::<Self>(node_ip, trusted_peers).await?;
        // Load the coinbase puzzle.
        let coinbase_puzzle = CoinbasePuzzle::<N>::load()?;
        // Initialize the node.
        let node = Self {
            account,
            router: router.clone(),
            coinbase_puzzle,
            latest_epoch_challenge: Default::default(),
            latest_block: Default::default(),
        };
        // Initialize the router handler.
        router.initialize_handler(node.clone(), router_receiver).await;
        // Initialize coinbase proving.
        node.initialize_coinbase_proving().await;
        // Initialize the signal handler.
        let _ = node.handle_signals();
        // Return the node.
        Ok(node)
    }
}

impl<N: Network> Prover<N> {
    /// Initialize a new instance of coinbase proving.
    async fn initialize_coinbase_proving(&self) {
        let prover = self.clone();
        spawn_task!(Self, {
            loop {
                // If the status is `Peering`, then skip this iteration.
                if !Self::status().is_peering() {
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    continue;
                }

                // Read the latest epoch challenge.
                let latest_epoch_challenge = prover.latest_epoch_challenge.read().await.clone();
                // Read the latest block.
                let latest_block = prover.latest_block.read().await.clone();

                // If the latest epoch challenge and latest block exists, then generate a coinbase proof.
                if let (Some(epoch_challenge), Some(block)) = (latest_epoch_challenge, latest_block) {
                    let prover = prover.clone();
                    spawn_task!(Self, {
                        // Set the status to `Proving`.
                        Self::status().update(Status::Proving);

                        trace!("Generating a prover solution for epoch {}", epoch_challenge.epoch_number());

                        // Construct a coinbase solution.
                        let prover_solution = match prover.coinbase_puzzle.prove(
                            &epoch_challenge,
                            *prover.address(),
                            rand::thread_rng().gen(),
                        ) {
                            Ok(proof) => proof,
                            Err(error) => {
                                warn!("Failed to generate prover solution: {error}");
                                return;
                            }
                        };

                        // Fetch the prover solution target.
                        let prover_solution_target = match prover_solution.to_target() {
                            Ok(target) => target,
                            Err(error) => {
                                warn!("Failed to fetch prover solution target: {error}");
                                return;
                            }
                        };

                        // Ensure that the prover solution target is sufficient.
                        if prover_solution_target < block.proof_target() {
                            warn!(
                                "Generated prover solution does not meet the target requirement. {} < {}",
                                prover_solution_target,
                                block.proof_target()
                            );
                            return;
                        }

                        // Propagate the "UnconfirmedSolution" to the network.
                        let message = Message::UnconfirmedSolution(UnconfirmedSolution {
                            solution: Data::Object(prover_solution),
                        });
                        let request = RouterRequest::MessagePropagate(message, vec![]);
                        if let Err(error) = prover.router.process(request).await {
                            warn!("[UnconfirmedSolution] {}", error);
                        }

                        // Set the status to `Ready`.
                        Self::status().update(Status::Ready);
                    })
                } else {
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                }
            }
        });
    }
}

#[async_trait]
impl<N: Network> Executor for Prover<N> {
    /// The node type.
    const NODE_TYPE: NodeType = NodeType::Prover;
}

impl<N: Network> NodeInterface<N> for Prover<N> {
    /// Returns the node type.
    fn node_type(&self) -> NodeType {
        Self::NODE_TYPE
    }

    /// Returns the node router.
    fn router(&self) -> &Router<N> {
        &self.router
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
    fn address(&self) -> &Address<N> {
        self.account.address()
    }
}
