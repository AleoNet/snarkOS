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
use snarkos_node_executor::{Executor, NodeType};
use snarkos_node_messages::{Message, PuzzleResponse, UnconfirmedSolution};
use snarkos_node_router::{Handshake, Inbound, Outbound, Router, RouterRequest};
use snarkvm::prelude::{Address, Block, CoinbasePuzzle, EpochChallenge, Network, PrivateKey, ProverSolution, ViewKey};

use anyhow::Result;
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::RwLock;

/// A client node is a full node, capable of querying with the network.
#[derive(Clone)]
pub struct Client<N: Network> {
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

impl<N: Network> Client<N> {
    /// Initializes a new client node.
    pub async fn new(node_ip: SocketAddr, private_key: PrivateKey<N>, trusted_peers: &[SocketAddr]) -> Result<Self> {
        // Initialize the node account.
        let account = Account::from(private_key)?;
        // Initialize the node router.
        let (router, router_receiver) = Router::new::<Self>(node_ip, account.address(), trusted_peers).await?;
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
        // Initialize the signal handler.
        node.handle_signals();
        // Return the node.
        Ok(node)
    }
}

#[async_trait]
impl<N: Network> Executor for Client<N> {
    /// The node type.
    const NODE_TYPE: NodeType = NodeType::Client;
}

impl<N: Network> NodeInterface<N> for Client<N> {
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
    fn address(&self) -> Address<N> {
        self.account.address()
    }
}
