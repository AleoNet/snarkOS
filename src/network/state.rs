// Copyright (C) 2019-2021 Aleo Systems Inc.
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

use crate::{
    ledger::{Ledger, LedgerRequest, LedgerRouter},
    Environment,
    Message,
    PeersRequest,
    PeersRouter,
};
use snarkvm::prelude::*;

use std::{
    collections::{HashMap, HashSet},
    marker::PhantomData,
    net::SocketAddr,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::sync::{mpsc, RwLock};

/// Shorthand for the parent half of the `State` message channel.
pub(crate) type StateRouter<N, E> = mpsc::Sender<StateRequest<N, E>>;
/// Shorthand for the child half of the `State` message channel.
type StateHandler<N, E> = mpsc::Receiver<StateRequest<N, E>>;

///
/// An enum of requests that the `State` struct processes.
///
#[derive(Debug)]
pub enum StateRequest<N: Network, E: Environment> {
    /// Disconnect := (peer_ip)
    Disconnect(SocketAddr),
    /// Heartbeat := ()
    Heartbeat,
    /// Ping := (peer_ip, version, block_height, peers_router)
    Ping(SocketAddr, u32, u32, PeersRouter<N, E>),
    /// Pong := (peer_ip)
    Pong(SocketAddr),
    /// SyncRequest := (peer_ip, block_height)
    SyncRequest(SocketAddr, u32),
    /// SyncResponse := (peer_ip, block_height, block)
    SyncResponse(SocketAddr, u32, Block<N>),
    /// UnconfirmedBlock := (peer_ip, block_height, block)
    UnconfirmedBlock(SocketAddr, u32, Block<N>),
    /// UnconfirmedTransaction := (peer_ip, transaction)
    UnconfirmedTransaction(SocketAddr, Transaction<N>),
}

pub(crate) struct State<N: Network, E: Environment> {
    /// The local address of this node.
    local_ip: SocketAddr,
    /// The map of each peer to their version number.
    version: HashMap<SocketAddr, u32>,
    /// The map of each peer to their block height.
    block_height: HashMap<SocketAddr, u32>,
    /// The map of each peer to their sync requests.
    sync_requests: HashMap<SocketAddr, HashSet<u32>>,
    /// The map of each peer to their failure messages.
    failures: HashMap<SocketAddr, Vec<String>>,
    /// A boolean that is `true` when syncing.
    is_syncing: Arc<AtomicBool>,

    candidate_blocks: HashMap<N::BlockHash, Block<N>>,
    _phantom: PhantomData<(N, E)>,
}

impl<N: Network, E: Environment> State<N, E> {
    ///
    /// Initializes a new instance of `State`.
    ///
    pub(crate) fn new(local_ip: SocketAddr) -> Self {
        Self {
            local_ip,
            version: Default::default(),
            block_height: Default::default(),
            sync_requests: Default::default(),
            failures: Default::default(),
            is_syncing: Arc::new(AtomicBool::new(false)),

            candidate_blocks: Default::default(),
            _phantom: PhantomData,
        }
    }

    ///
    /// Returns `true` if the state manager is syncing.
    ///
    pub(crate) fn is_syncing(&self) -> bool {
        self.is_syncing.load(Ordering::SeqCst)
    }

    ///
    /// Performs the given `request` to the state manager.
    /// All requests must go through this `update`, so that a unified view is preserved.
    ///
    pub(super) async fn update(
        &mut self,
        request: StateRequest<N, E>,
        peers_router: PeersRouter<N, E>,
        ledger: Arc<RwLock<Ledger<N>>>,
        ledger_router: LedgerRouter<N, E>,
    ) {
        match request {
            StateRequest::Disconnect(peer_ip) => {
                // Remove all entries of the peer from the state manager.
                self.remove_peer(&peer_ip);
                // Process the disconnect.
                info!("Disconnecting from {}", peer_ip);
                // Route a `PeerDisconnected` to the peers.
                if let Err(error) = peers_router.send(PeersRequest::PeerDisconnected(peer_ip)).await {
                    warn!("[Disconnect] {}", error);
                }
            }
            StateRequest::Heartbeat => {
                // Retrieve the latest block of the ledger.
                let latest_block = match ledger.read().await.latest_block() {
                    Ok(block) => block,
                    Err(error) => {
                        error!("{}", error);
                        return;
                    }
                };

                // Check for candidate blocks to fast forward the ledger.
                let mut block = latest_block.clone();
                while self.candidate_blocks.contains_key(&block.block_hash()) {
                    block = match self.candidate_blocks.get(&block.block_hash()) {
                        Some(block) => block.clone(),
                        None => {
                            error!("Failed to find the candidate block");
                            break;
                        }
                    };

                    // Route a `SyncResponse` to the ledger.
                    let request = LedgerRequest::SyncResponse(block.clone());
                    if let Err(error) = ledger_router.send(request).await {
                        warn!("[UnconfirmedBlock] {}", error);
                    }
                }

                // Ensure the state manager is not already syncing.
                if self.is_syncing() {
                    return;
                }

                // Retrieve the current block height of the ledger.
                let latest_block_height = latest_block.height();
                // Iterate through the peers to check if this node needs to catch up.
                let mut maximal_peer = None;
                let mut maximum_block_height = 0;
                for (peer_ip, block_height) in self.block_height.iter() {
                    if *block_height > maximum_block_height {
                        maximal_peer = Some(*peer_ip);
                        maximum_block_height = *block_height;
                    }
                }
                // Proceed to add sync requests if the maximum block height is higher than the latest.
                if let Some(peer_ip) = maximal_peer {
                    if maximum_block_height > latest_block_height {
                        let num_blocks = std::cmp::min(maximum_block_height - latest_block_height, 32);
                        trace!("Preparing to sync the next {} blocks from {}", num_blocks, peer_ip);
                        self.is_syncing.store(true, Ordering::SeqCst);
                        // Add sync requests for each block height up to the maximum block height.
                        for block_height in (latest_block_height + 1)..(latest_block_height + 1 + num_blocks) {
                            if !self.contains_sync_request(peer_ip, block_height) {
                                // Add the sync request to the state manager.
                                self.add_sync_request(peer_ip, block_height);
                                // Request the next block from the peer.
                                let request = PeersRequest::MessageSend(peer_ip, Message::SyncRequest(block_height));
                                // Send a `SyncRequest` message to the peer.
                                if let Err(error) = peers_router.send(request).await {
                                    warn!("[SyncRequest] {}", error);
                                }
                            }
                        }
                    }
                }
            }
            StateRequest::Ping(peer_ip, version, block_height, peers_router) => {
                // Ensure the peer has been initialized in the state manager.
                self.initialize_peer(peer_ip, version, block_height);
                // Update the version number for the peer.
                self.update_version(peer_ip, version);
                // Update the block height of the peer.
                self.update_block_height(peer_ip, block_height);
                // Send a `Pong` message to the peer.
                if let Err(error) = peers_router.send(PeersRequest::MessageSend(peer_ip, Message::Pong)).await {
                    warn!("[Pong] {}", error);
                }
            }
            StateRequest::Pong(peer_ip) => {
                // Sleep for 10 seconds.
                tokio::time::sleep(Duration::from_secs(10)).await;
                // Fetch the latest block height of this ledger.
                let block_height = ledger.read().await.latest_block_height();
                // Send a `Ping` message to the peer.
                let request = PeersRequest::MessageSend(peer_ip, Message::Ping(E::MESSAGE_VERSION, block_height));
                if let Err(error) = peers_router.send(request).await {
                    warn!("[Ping] {}", error);
                }
            }
            StateRequest::SyncRequest(peer_ip, block_height) => {
                // Fetch the block of the given block height from the ledger.
                let request = match ledger.read().await.get_block(block_height) {
                    Ok(block) => PeersRequest::MessageSend(peer_ip, Message::SyncResponse(block.height(), block)),
                    Err(error) => {
                        self.add_failure(peer_ip, format!("{}", error));
                        return;
                    }
                };
                // Send a `SyncResponse` message to the peer.
                if let Err(error) = peers_router.send(request).await {
                    warn!("[SyncResponse] {}", error);
                }
            }
            StateRequest::SyncResponse(peer_ip, block_height, block) => {
                // Ensure the block height corresponds to the requested block.
                if !self.contains_sync_request(peer_ip, block_height) {
                    self.add_failure(peer_ip, "Received sync response for an unrequested block".to_string());
                }
                // Process the sync response.
                else {
                    // Remove the sync request from the state manager.
                    self.remove_sync_request(peer_ip, block_height);
                    // Ensure the block height corresponds to the requested block.
                    if block_height != block.height() {
                        self.add_failure(peer_ip, "Block height does not match".to_string());
                    }
                    // Process the sync response.
                    else if block.is_valid() {
                        // Add the block to the candidate blocks.
                        self.candidate_blocks.insert(block.previous_block_hash(), block.clone());
                        // Check if syncing with this peer is complete.
                        if let Some(requests) = self.sync_requests.get(&peer_ip) {
                            if requests.is_empty() {
                                trace!("All sync requests with {} have been processed", peer_ip);
                                self.is_syncing.store(false, Ordering::SeqCst);
                            }
                        }
                    }
                }
            }
            StateRequest::UnconfirmedBlock(peer_ip, block_height, block) => {
                // Ensure the block height corresponds to the requested block.
                if block_height != block.height() {
                    self.add_failure(peer_ip, "Block height does not match".to_string());
                }
                // Process the unconfirmed block.
                trace!("Received unconfirmed block {} from {}", block_height, peer_ip);
                // Route an `UnconfirmedBlock` to the ledger.
                let request = LedgerRequest::UnconfirmedBlock(peer_ip, block.clone(), peers_router.clone());
                if let Err(error) = ledger_router.send(request).await {
                    warn!("[UnconfirmedBlock] {}", error);
                }
            }
            StateRequest::UnconfirmedTransaction(peer_ip, transaction) => {
                // Process the unconfirmed transaction.
                trace!("Received unconfirmed transaction {} from {}", transaction.transaction_id(), peer_ip);
                // Route an `UnconfirmedTransaction` to the ledger.
                let request = LedgerRequest::UnconfirmedTransaction(peer_ip, transaction.clone(), peers_router.clone());
                if let Err(error) = ledger_router.send(request).await {
                    warn!("[UnconfirmedTransaction] {}", error);
                }
            }
        }
    }

    ///
    /// Adds an entry for the given peer IP to every data structure in `State`.
    ///
    fn initialize_peer(&mut self, peer_ip: SocketAddr, version: u32, block_height: u32) {
        if !self.version.contains_key(&peer_ip) {
            self.version.insert(peer_ip, version);
        }
        if !self.block_height.contains_key(&peer_ip) {
            self.block_height.insert(peer_ip, block_height);
        }
        if !self.sync_requests.contains_key(&peer_ip) {
            self.sync_requests.insert(peer_ip, Default::default());
        }
        if !self.failures.contains_key(&peer_ip) {
            self.failures.insert(peer_ip, Default::default());
        }
    }

    ///
    /// Removes the entry for the given peer IP from every data structure in `State`.
    ///
    fn remove_peer(&mut self, peer_ip: &SocketAddr) {
        if self.version.contains_key(peer_ip) {
            self.version.remove(peer_ip);
        }
        if self.block_height.contains_key(peer_ip) {
            self.block_height.remove(peer_ip);
        }
        if self.sync_requests.contains_key(peer_ip) {
            self.sync_requests.remove(peer_ip);
        }
        if self.failures.contains_key(peer_ip) {
            self.failures.remove(peer_ip);
        }
    }

    ///
    /// Updates the version of the given peer.
    ///
    fn update_version(&mut self, peer_ip: SocketAddr, version: u32) {
        match self.version.get_mut(&peer_ip) {
            Some(previous_version) => *previous_version = version,
            None => self.add_failure(peer_ip, format!("Missing version for {}", peer_ip)),
        };
    }

    ///
    /// Updates the block height of the given peer.
    ///
    fn update_block_height(&mut self, peer_ip: SocketAddr, block_height: u32) {
        match self.block_height.get_mut(&peer_ip) {
            Some(height) => *height = block_height,
            None => self.add_failure(peer_ip, format!("Missing block height for {}", peer_ip)),
        };
    }

    ///
    /// Adds a sync request for the given block height to the specified peer.
    ///
    fn add_sync_request(&mut self, peer_ip: SocketAddr, block_height: u32) {
        match self.sync_requests.get_mut(&peer_ip) {
            Some(requests) => match requests.insert(block_height) {
                true => trace!("Added sync request for block {} from {}", block_height, peer_ip),
                false => self.add_failure(peer_ip, format!("Duplicate sync request from {}", peer_ip)),
            },
            None => self.add_failure(peer_ip, format!("Missing sync requests for {}", peer_ip)),
        };
    }

    ///
    /// Returns `true` if the sync request for the given block height to the specified peer exists.
    ///
    fn contains_sync_request(&self, peer_ip: SocketAddr, block_height: u32) -> bool {
        match self.sync_requests.get(&peer_ip) {
            Some(requests) => requests.contains(&block_height),
            None => false,
        }
    }

    ///
    /// Removes a sync request for the given block height to the specified peer.
    ///
    fn remove_sync_request(&mut self, peer_ip: SocketAddr, block_height: u32) {
        match self.sync_requests.get_mut(&peer_ip) {
            Some(requests) => {
                if !requests.remove(&block_height) {
                    self.add_failure(peer_ip, format!("Non-existent sync request from {}", peer_ip))
                }
            }
            None => self.add_failure(peer_ip, format!("Missing sync requests for {}", peer_ip)),
        };
    }

    ///
    /// Adds the given failure message to the specified peer IP.
    ///
    fn add_failure(&mut self, peer_ip: SocketAddr, failure: String) {
        trace!("Adding failure for {}: {}", peer_ip, failure);
        match self.failures.get_mut(&peer_ip) {
            Some(failures) => failures.push(failure),
            None => error!("Missing failure entry for {}", peer_ip),
        };
    }
}
