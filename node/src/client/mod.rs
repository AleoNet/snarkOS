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
use snarkos_node_messages::{Message, NodeType, UnconfirmedSolution};
use snarkos_node_router::{Heartbeat, Inbound, Outbound, Router, Routing};
use snarkos_node_tcp::{
    protocols::{Disconnect, Handshake, OnConnect, Reading, Writing},
    P2P,
};
use snarkvm::prelude::{Block, CoinbasePuzzle, ConsensusStorage, EpochChallenge, Header, Network, ProverSolution};

use anyhow::Result;
use core::marker::PhantomData;
use parking_lot::RwLock;
use std::{net::SocketAddr, sync::Arc};

/// A client node is a full node, capable of querying with the network.
#[derive(Clone)]
pub struct Client<N: Network, C: ConsensusStorage<N>> {
    /// The router of the node.
    router: Router<N>,
    /// The genesis block.
    genesis: Block<N>,
    /// The coinbase puzzle.
    coinbase_puzzle: CoinbasePuzzle<N>,
    /// The latest epoch challenge.
    latest_epoch_challenge: Arc<RwLock<Option<EpochChallenge<N>>>>,
    /// The latest block header.
    latest_block_header: Arc<RwLock<Option<Header<N>>>>,
    /// PhantomData.
    _phantom: PhantomData<C>,
}

impl<N: Network, C: ConsensusStorage<N>> Client<N, C> {
    /// Initializes a new client node.
    pub async fn new(
        node_ip: SocketAddr,
        account: Account<N>,
        trusted_peers: &[SocketAddr],
        genesis: Block<N>,
        dev: Option<u16>,
    ) -> Result<Self> {
        // Initialize the signal handler.
        let signal_node = Self::handle_signals();

        // Initialize the node router.
        let router = Router::new(
            node_ip,
            NodeType::Client,
            account,
            trusted_peers,
            Self::MAXIMUM_NUMBER_OF_PEERS as u16,
            dev.is_some(),
        )
        .await?;
        // Load the coinbase puzzle.
        let coinbase_puzzle = CoinbasePuzzle::<N>::load()?;
        // Initialize the node.
        let node = Self {
            router,
            genesis,
            coinbase_puzzle,
            latest_epoch_challenge: Default::default(),
            latest_block_header: Default::default(),
            _phantom: PhantomData,
        };
        // Initialize the routing.
        node.initialize_routing().await;
        // Pass the node to the signal handler.
        let _ = signal_node.set(node.clone());
        // Return the node.
        Ok(node)
    }
}

#[async_trait]
impl<N: Network, C: ConsensusStorage<N>> NodeInterface<N> for Client<N, C> {
    /// Shuts down the node.
    async fn shut_down(&self) {
        info!("Shutting down...");

        // Shut down the router.
        self.router.shut_down().await;

        info!("Node has shut down.");
    }
}
