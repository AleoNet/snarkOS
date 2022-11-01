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
use snarkos_node_ledger::Ledger;
use snarkos_node_rest::Rest;
use snarkos_node_router::{Handshake, Inbound, Outbound, Router};
use snarkos_node_store::ConsensusDB;
use snarkvm::prelude::{Address, Block, Network, PrivateKey, ViewKey};

use anyhow::Result;
use std::{net::SocketAddr, sync::Arc};

/// A validator is a full node, capable of validating blocks.
#[derive(Clone)]
pub struct Validator<N: Network> {
    /// The account of the node.
    account: Account<N>,
    /// The ledger of the node.
    ledger: Ledger<N, ConsensusDB<N>>,
    /// The router of the node.
    router: Router<N>,
    /// The REST server of the node.
    rest: Option<Arc<Rest<N, ConsensusDB<N>>>>,
}

impl<N: Network> Validator<N> {
    /// Initializes a new validator node.
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
        // Initialize the node.
        let node = Self { account, ledger, router: router.clone(), rest };
        // Initialize the router handler.
        router.initialize_handler(node.clone(), router_receiver).await;
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
impl<N: Network> Executor for Validator<N> {
    /// The node type.
    const NODE_TYPE: NodeType = NodeType::Validator;

    /// Disconnects from peers and shuts down the node.
    async fn shut_down(&self) {
        info!("Shutting down...");
        // Update the node status.
        Self::status().update(Status::ShuttingDown);

        // Shut down the ledger.
        trace!("Proceeding to shut down the ledger...");
        // self.state.ledger().shut_down().await;

        // Flush the tasks.
        Self::resources().shut_down();
        trace!("Node has shut down.");
    }
}

impl<N: Network> NodeInterface<N> for Validator<N> {
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
