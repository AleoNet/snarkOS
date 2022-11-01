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
use snarkos_node_ledger::{Consensus, Ledger};
use snarkos_node_messages::{Data, Message, PuzzleResponse, UnconfirmedBlock, UnconfirmedSolution};
use snarkos_node_rest::Rest;
use snarkos_node_router::{Handshake, Inbound, Outbound, Router, RouterRequest};
use snarkos_node_store::ConsensusDB;
use snarkvm::prelude::{Address, Block, Network, PrivateKey, ViewKey};

use anyhow::{bail, Result};
use core::time::Duration;
use parking_lot::RwLock;
use std::{
    net::SocketAddr,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc,
    },
};
use time::OffsetDateTime;
use tokio::time::timeout;

/// A beacon is a full node, capable of producing blocks.
#[derive(Clone)]
pub struct Beacon<N: Network> {
    /// The account of the node.
    account: Account<N>,
    /// The ledger of the node.
    ledger: Ledger<N, ConsensusDB<N>>,
    /// The router of the node.
    router: Router<N>,
    /// The REST server of the node.
    rest: Option<Arc<Rest<N, ConsensusDB<N>>>>,
    /// The time it to generate a block.
    block_generation_time: Arc<AtomicU64>,
    /// The shutdown signal.
    shutdown: Arc<AtomicBool>,
}

impl<N: Network> Beacon<N> {
    /// Initializes a new beacon node.
    pub async fn new(
        node_ip: SocketAddr,
        rest_ip: Option<SocketAddr>,
        private_key: PrivateKey<N>,
        trusted_peers: &[SocketAddr],
        genesis: Option<Block<N>>,
        dev: Option<u16>,
    ) -> Result<Self> {
        // Initialize the node account.
        let account = Account::from(private_key)?;
        // Initialize the ledger.
        let ledger = Ledger::load(private_key, genesis, dev)?;
        // Initialize the node router.
        let (router, router_receiver) = Router::new::<Self>(node_ip, trusted_peers).await?;
        // Initialize the REST server.
        let rest = match rest_ip {
            Some(rest_ip) => Some(Arc::new(Rest::start(rest_ip, ledger.clone(), router.clone())?)),
            None => None,
        };
        // Initialize the block generation time.
        let block_generation_time = Arc::new(AtomicU64::new(2));
        // Initialize the node.
        let node =
            Self { account, ledger, router: router.clone(), rest, block_generation_time, shutdown: Default::default() };
        // Initialize the router handler.
        router.initialize_handler(node.clone(), router_receiver).await;

        // Initialize the block production.
        node.initialize_block_production().await;
        // Initialize the signal handler.
        node.handle_signals();
        // Return the node.
        Ok(node)
    }

    /// Returns the ledger.
    pub fn ledger(&self) -> &Ledger<N, ConsensusDB<N>> {
        &self.ledger
    }

    /// Returns the REST server.
    pub fn rest(&self) -> &Option<Arc<Rest<N, ConsensusDB<N>>>> {
        &self.rest
    }
}

#[async_trait]
impl<N: Network> Executor for Beacon<N> {
    /// The node type.
    const NODE_TYPE: NodeType = NodeType::Beacon;

    /// Disconnects from peers and shuts down the node.
    async fn shut_down(&self) {
        info!("Shutting down...");
        // Update the node status.
        Self::status().update(Status::ShuttingDown);

        // Shut down the ledger.
        trace!("Proceeding to shut down the ledger...");
        self.shutdown.store(true, Ordering::Relaxed);

        // Flush the tasks.
        Self::resources().shut_down();
        trace!("Node has shut down.");
    }
}

impl<N: Network> NodeInterface<N> for Beacon<N> {
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

/// A helper method to check if the coinbase target has been met.
async fn check_for_coinbase<N: Network>(consensus: Arc<RwLock<Consensus<N, ConsensusDB<N>>>>) {
    loop {
        // Check if the coinbase target has been met.
        match consensus.read().is_coinbase_target_met() {
            Ok(true) => break,
            Ok(false) => (),
            Err(error) => error!("Failed to check if coinbase target is met: {error}"),
        }
        // Sleep for one second.
        tokio::time::sleep(Duration::from_secs(1)).await
    }
}

impl<N: Network> Beacon<N> {
    /// Initialize a new instance of block production.
    async fn initialize_block_production(&self) {
        let beacon = self.clone();
        spawn_task!(Self, {
            // Expected time per block.
            const ROUND_TIME: u64 = 15; // 15 seconds per block

            // Produce blocks.
            loop {
                // Fetch the current timestamp.
                let current_timestamp = OffsetDateTime::now_utc().unix_timestamp();
                // Compute the elapsed time.
                let elapsed_time = match beacon.ledger.consensus().read().latest_timestamp() {
                    Ok(latest_timestamp) => current_timestamp.saturating_sub(latest_timestamp) as u64,
                    Err(_) => {
                        warn!("Failed to fetch the latest block timestamp");
                        0
                    }
                };

                // Do not produce a block if the elapsed time has not exceeded `ROUND_TIME - block_generation_time`.
                // This will ensure a block is produced at intervals of approximately `ROUND_TIME`.
                let time_to_wait = ROUND_TIME.saturating_sub(beacon.block_generation_time.load(Ordering::Relaxed));
                if elapsed_time < time_to_wait {
                    if let Err(error) = timeout(
                        Duration::from_secs(time_to_wait.saturating_sub(elapsed_time)),
                        check_for_coinbase(beacon.ledger.consensus().clone()),
                    )
                    .await
                    {
                        trace!("Check for coinbase - {error}");
                    }
                }

                let beacon_clone = beacon.clone();
                spawn_task!(Self, {
                    // Start a timer.
                    let timer = std::time::Instant::now();
                    // Produce the next block and propagate it to all peers.
                    match beacon_clone.produce_next_block().await {
                        // Update the block generation time.
                        Ok(()) => {
                            beacon_clone.block_generation_time.store(timer.elapsed().as_secs(), Ordering::Relaxed)
                        }
                        Err(error) => error!("{error}"),
                    }
                });

                // If the Ctrl-C handler registered the signal, stop the node once the current block is complete.
                if beacon.shutdown.load(Ordering::Relaxed) {
                    info!("Shutting down block production");
                    break;
                }
            }
        });
    }

    /// Produces the next block and propagates it to all peers.
    async fn produce_next_block(&self) -> Result<()> {
        // Produce a transaction if the mempool is empty.
        if self.ledger.consensus().read().memory_pool().num_unconfirmed_transactions() == 0 {
            // Create a transfer transaction.
            let transaction = match self.ledger.create_transfer(self.address(), 1) {
                Ok(transaction) => transaction,
                Err(error) => {
                    bail!("Failed to create a transfer transaction for the next block: {error}")
                }
            };
            // Add the transaction to the memory pool.
            if let Err(error) = self.ledger.consensus().write().add_unconfirmed_transaction(transaction) {
                bail!("Failed to add a transfer transaction to the memory pool: {error}")
            }
        }

        // Propose the next block.
        let next_block =
            match self.ledger.consensus().read().propose_next_block(self.private_key(), &mut rand::thread_rng()) {
                Ok(next_block) => next_block,
                Err(error) => {
                    bail!("Failed to propose the next block: {error}")
                }
            };
        let next_block_height = next_block.height();
        let next_block_hash = next_block.hash();

        // Advance to the next block.
        match self.ledger.consensus().write().add_next_block(&next_block) {
            Ok(()) => match serde_json::to_string_pretty(&next_block) {
                Ok(block) => info!("Block {next_block_height}: {block}"),
                Err(error) => info!("Block {next_block_height}: (serde failed: {error})"),
            },
            Err(error) => bail!("Failed to advance to the next block: {error}"),
        }

        // Serialize the block ahead of time to not do it for each peer.
        let serialized_block = match Data::Object(next_block).serialize().await {
            Ok(serialized_block) => serialized_block,
            Err(error) => bail!("Failed to serialize the next block for propagation: {error}"),
        };

        // Prepare the block to be sent to all peers.
        let message = Message::<N>::UnconfirmedBlock(UnconfirmedBlock {
            block_height: next_block_height,
            block_hash: next_block_hash,
            block: Data::Buffer(serialized_block),
        });

        // Propagate the block to all peers.
        if let Err(error) = self.router.process(RouterRequest::MessagePropagate(message, vec![])).await {
            trace!("Failed to broadcast the next block: {error}");
        }

        Ok(())
    }
}
