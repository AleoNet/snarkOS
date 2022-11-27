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
use snarkos_node_consensus::Consensus;
use snarkos_node_ledger::Ledger;
use snarkos_node_messages::{BlockRequest, Message, NodeType, PuzzleResponse, Status, UnconfirmedSolution};
use snarkos_node_rest::Rest;
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
use parking_lot::RwLock;
use std::{
    net::SocketAddr,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::task::JoinHandle;

/// A validator is a full node, capable of validating blocks.
#[derive(Clone)]
pub struct Validator<N: Network, C: ConsensusStorage<N>> {
    /// The account of the node.
    account: Account<N>,
    /// The ledger of the node.
    ledger: Ledger<N, C>,
    /// The consensus module of the node.
    consensus: Consensus<N, C>,
    /// The router of the node.
    router: Router<N>,
    /// The REST server of the node.
    rest: Option<Arc<Rest<N, C, Self>>>,
    /// The coinbase puzzle.
    coinbase_puzzle: CoinbasePuzzle<N>,
    /// The latest epoch challenge.
    latest_epoch_challenge: Arc<RwLock<Option<EpochChallenge<N>>>>,
    /// The latest block.
    latest_block: Arc<RwLock<Option<Block<N>>>>,
    /// The latest puzzle response.
    latest_puzzle_response: Arc<RwLock<Option<PuzzleResponse<N>>>>,
    /// The spawned handles.
    handles: Arc<RwLock<Vec<JoinHandle<()>>>>,
    /// The shutdown signal.
    shutdown: Arc<AtomicBool>,
}

impl<N: Network, C: ConsensusStorage<N>> Validator<N, C> {
    /// Initializes a new validator node.
    pub async fn new(
        node_ip: SocketAddr,
        rest_ip: Option<SocketAddr>,
        account: Account<N>,
        trusted_peers: &[SocketAddr],
        genesis: Block<N>,
        cdn: Option<String>,
        dev: Option<u16>,
    ) -> Result<Self> {
        // Initialize the ledger.
        let ledger = Ledger::load(genesis, dev)?;
        // Initialize the CDN.
        if let Some(base_url) = cdn {
            // Sync the ledger with the CDN.
            if let Err((_, error)) = snarkos_node_cdn::sync_ledger_with_cdn(&base_url, ledger.clone()).await {
                crate::helpers::log_clean_error(dev);
                return Err(error);
            }
        }
        // Initialize the consensus.
        let consensus = Consensus::new(ledger.clone())?;

        // Initialize the node router.
        let router = Router::new(
            node_ip,
            NodeType::Validator,
            account.address(),
            trusted_peers,
            Self::MAXIMUM_NUMBER_OF_PEERS as u16,
            dev.is_some(),
        )
        .await?;

        // Load the coinbase puzzle.
        let coinbase_puzzle = CoinbasePuzzle::<N>::load()?;
        // Initialize the node.
        let mut node = Self {
            account,
            ledger: ledger.clone(),
            consensus: consensus.clone(),
            router,
            rest: None,
            coinbase_puzzle,
            latest_epoch_challenge: Default::default(),
            latest_block: Default::default(),
            latest_puzzle_response: Default::default(),
            handles: Default::default(),
            shutdown: Default::default(),
        };

        // Initialize the REST server.
        if let Some(rest_ip) = rest_ip {
            node.rest = Some(Arc::new(Rest::start(rest_ip, Some(consensus), ledger, Arc::new(node.clone()))?));
        }
        // Initialize the sync pool.
        node.initialize_sync().await;
        // Initialize the routing.
        node.initialize_routing().await;
        // Initialize the signal handler.
        node.handle_signals();
        // Return the node.
        Ok(node)
    }

    /// Returns the ledger.
    pub fn ledger(&self) -> &Ledger<N, C> {
        &self.ledger
    }

    /// Returns the REST server.
    pub fn rest(&self) -> &Option<Arc<Rest<N, C, Self>>> {
        &self.rest
    }
}

#[async_trait]
impl<N: Network, C: ConsensusStorage<N>> NodeInterface<N> for Validator<N, C> {
    /// Returns the node type.
    fn node_type(&self) -> NodeType {
        self.router.node_type()
    }

    /// Returns the node status.
    fn status(&self) -> Status {
        self.router.status()
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

    /// Shuts down the node.
    async fn shut_down(&self) {
        info!("Shutting down...");

        // Shut down the sync pool.
        trace!("Shutting down the sync pool...");
        self.shutdown.store(true, Ordering::SeqCst);

        // Abort the tasks.
        trace!("Shutting down the validator...");
        self.handles.read().iter().for_each(|handle| handle.abort());

        // Shut down the router.
        self.router.shut_down().await;

        // Shut down the ledger.
        trace!("Shutting down the ledger...");
        // self.ledger.shut_down().await;

        info!("Node has shut down.");
    }
}

impl<N: Network, C: ConsensusStorage<N>> Validator<N, C> {
    /// TODO: THIS IS A DUMMY IMPLEMENTATION. DO NOT USE.
    async fn initialize_sync(&self) {
        let validator = self.clone();
        self.handles.write().push(tokio::spawn(async move {
            // Expected time per block.
            const ROUND_TIME: u64 = 15; // 15 seconds per block

            validator
                .router
                .sync()
                .insert_canon_locators(crate::helpers::get_block_locators(&validator.ledger).unwrap())
                .unwrap();

            loop {
                // If the Ctrl-C handler registered the signal, stop the node.
                if validator.shutdown.load(Ordering::Relaxed) {
                    info!("Shutting down block production");
                    break;
                }

                let block_requests = validator.router.sync().prepare_block_requests();
                trace!("{:?} block requests", block_requests.len());

                for (height, (hash, previous_hash, sync_ips)) in block_requests {
                    if validator
                        .router
                        .sync()
                        .insert_block_request(height, (hash, previous_hash, sync_ips.clone()))
                        .is_ok()
                    {
                        for sync_ip in sync_ips {
                            validator.send(
                                sync_ip,
                                Message::BlockRequest(BlockRequest { start_height: height, end_height: height + 1 }),
                            );
                        }
                    }
                }

                tokio::time::sleep(Duration::from_secs(1)).await;

                // // Retrieve the latest block height.
                // let latest_height = validator.ledger.latest_height();
                // // Retrieve the peers with their heights.
                // let peers_by_height = validator.router.sync().get_sync_peers_by_height();
                //
                // if peers_by_height.is_empty() {
                //     // Wait for a bit.
                //     tokio::time::sleep(Duration::from_secs(ROUND_TIME)).await;
                //     continue;
                // }
                //
                // // Retrieve the first peer (the one with the highest height).
                // let (peer_ip, peer_height) = peers_by_height.first().unwrap();
                //
                // // Check if the peer is ahead of us.
                // if peer_height > &latest_height {
                //     if validator.router.sync().contains_request(latest_height + 1) {
                //         tokio::time::sleep(Duration::from_secs(1)).await;
                //         continue;
                //     }
                //     let hash = validator.router.sync().get_canon_hash(latest_height + 1);
                //     let previous_hash = validator.router.sync().get_canon_hash(latest_height);
                //     if let Err(error) =
                //         validator
                //             .router
                //             .sync()
                //             .insert_block_request(latest_height + 1, hash, previous_hash, indexset![*peer_ip])
                //     {
                //         error!("Failed to insert sync request: {}", error);
                //         // Wait for a bit.
                //         tokio::time::sleep(Duration::from_secs(ROUND_TIME)).await;
                //     } else {
                //         info!("Syncing with peer {}...", peer_ip);
                //         validator.send(
                //             *peer_ip,
                //             Message::BlockRequest(BlockRequest {
                //                 start_height: latest_height + 1,
                //                 end_height: latest_height + 2,
                //             }),
                //         );
                //     }
                // }
            }
        }));
    }
}
