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
use snarkos_node_bft::ledger_service::CoreLedgerService;
use snarkos_node_rest::Rest;
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
    console::network::Network,
    ledger::{
        block::{Block, Header},
        puzzle::{Puzzle, Solution},
        store::ConsensusStorage,
        Ledger,
    },
};

use aleo_std::StorageMode;
use anyhow::Result;
use core::future::Future;
use parking_lot::Mutex;
use std::{
    net::SocketAddr,
    sync::{atomic::AtomicBool, Arc},
};
use tokio::task::JoinHandle;

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
