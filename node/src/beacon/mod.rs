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
use snarkos_node_ledger::Ledger;
use snarkos_node_router::{Handshake, Inbound, Outbound, Router};
use snarkvm::prelude::{Address, Network, PrivateKey, ViewKey};

use anyhow::Result;
use std::{
    net::SocketAddr,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

/// A beacon is a full node, capable of producing blocks.
#[derive(Clone)]
pub struct Beacon<N: Network> {
    /// The account of the node.
    account: Account<N>,
    /// The router of the node.
    router: Router<N>,
    /// The ledger of the node.
    ledger: Ledger<N>,
    /// The shutdown signal.
    shutdown: Arc<AtomicBool>,
}

impl<N: Network> Beacon<N> {
    /// Initializes a new beacon node.
    pub async fn new(
        node_ip: SocketAddr,
        private_key: PrivateKey<N>,
        trusted_peers: &[SocketAddr],
        dev: Option<u16>,
    ) -> Result<Self> {
        // Initialize the node account.
        let account = Account::from(private_key)?;
        // Initialize the node router.
        let router = Router::new::<Self>(node_ip, *account.address(), NodeType::Beacon, trusted_peers).await?;
        // Initialize the ledger.
        let ledger = Ledger::<N>::load(private_key, dev, router.clone())?;
        // Initialize the node.
        let node = Self { account, router, ledger, shutdown: Default::default() };

        // Initialize the block production.
        node.initialize_block_production().await;

        // Initialize the signal handler.
        let _ = node.handle_signals();
        // Return the node.
        Ok(node)
    }

    /// Initialize a new instance of the heartbeat.
    async fn initialize_block_production(&self) {
        let beacon = self.clone();
        spawn_task!(Self, {
            loop {
                // Produce a transaction if the mempool is empty.
                if beacon.ledger.ledger().read().memory_pool().len() == 0 {
                    // Create a transfer transaction.
                    let transaction = match beacon.ledger.create_transfer(beacon.address(), 1) {
                        Ok(transaction) => transaction,
                        Err(error) => {
                            error!("Failed to create a transfer transaction for the next block: {error}");
                            continue;
                        }
                    };
                    // Add the transaction to the memory pool.
                    if let Err(error) = beacon.ledger.ledger().write().add_to_memory_pool(transaction) {
                        error!("Failed to add a transfer transaction to the memory pool: {error}");
                        continue;
                    }
                }

                // Advance to the next block.
                match beacon.ledger.advance_to_next_block().await {
                    Ok(next_block) => trace!(
                        "Block {}: {}",
                        next_block.height(),
                        serde_json::to_string_pretty(&next_block).expect("Failed to print next block")
                    ),
                    Err(error) => error!("Failed to advance to the next block: {error}"),
                }
            }
        });
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
        // self.state.ledger().shut_down().await;
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
