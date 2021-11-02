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
    /// BlockRequest := (peer_ip, block_height)
    BlockRequest(SocketAddr, u32),
    /// BlockResponse := (peer_ip, block_height, block)
    BlockResponse(SocketAddr, u32, Block<N>),
    /// Disconnect := (peer_ip)
    Disconnect(SocketAddr),
    /// Heartbeat := ()
    Heartbeat,
    /// Ping := (peer_ip, version, block_height, peers_router)
    Ping(SocketAddr, u32, u32, PeersRouter<N, E>),
    /// Pong := (peer_ip)
    Pong(SocketAddr),
    /// SyncRequest := (peer_ip, peers_router)
    SyncRequest(SocketAddr, PeersRouter<N, E>),
    /// SyncResponse := (peer_ip, \[(block height, block_hash)\])
    SyncResponse(SocketAddr, Vec<(u32, N::BlockHash)>),
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
    /// The map of each peer to their ledger state := (is_fork, common_ancestor, latest_block_height).
    ledger_state: HashMap<SocketAddr, Option<(bool, Option<u32>, u32)>>,
    /// The map of each peer to their block requests.
    block_requests: HashMap<SocketAddr, HashSet<u32>>,
    /// The map of each peer to their failure messages.
    failures: HashMap<SocketAddr, Vec<String>>,
    /// A boolean that is `true` when syncing.
    is_syncing: Arc<AtomicBool>,
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
            ledger_state: Default::default(),
            block_requests: Default::default(),
            failures: Default::default(),
            is_syncing: Arc::new(AtomicBool::new(false)),
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
            StateRequest::BlockRequest(peer_ip, block_height) => {
                // Fetch the block of the given block height from the ledger.
                let block = match ledger.read().await.get_block(block_height) {
                    Ok(block) => block,
                    Err(error) => {
                        self.add_failure(peer_ip, format!("{}", error));
                        return;
                    }
                };
                // Send a `BlockResponse` message to the peer.
                let request = PeersRequest::MessageSend(peer_ip, Message::BlockResponse(block.height(), block));
                if let Err(error) = peers_router.send(request).await {
                    warn!("[BlockResponse] {}", error);
                }
            }
            StateRequest::BlockResponse(peer_ip, block_height, block) => {
                // Ensure the block height corresponds to the requested block.
                if !self.contains_block_request(peer_ip, block_height) {
                    self.add_failure(peer_ip, "Received block response for an unrequested block".to_string());
                }
                // Process the sync response.
                else {
                    // Remove the block request from the state manager.
                    self.remove_block_request(peer_ip, block_height);
                    // Ensure the block height corresponds to the requested block.
                    if block_height != block.height() {
                        self.add_failure(peer_ip, "Block height does not match".to_string());
                    }
                    // Process the block response.
                    else {
                        // Route a `BlockResponse` to the ledger.
                        let request = LedgerRequest::BlockResponse(block.clone());
                        if let Err(error) = ledger_router.send(request).await {
                            warn!("[BlockResponse] {}", error);
                        }
                        // Check if syncing with this peer is complete.
                        if let Some(requests) = self.block_requests.get(&peer_ip) {
                            if requests.is_empty() {
                                trace!("All block requests with {} have been processed", peer_ip);
                                self.is_syncing.store(false, Ordering::SeqCst);
                            }
                        }
                    }
                }
            }
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
                // Ensure the state manager is not already syncing.
                if self.is_syncing() {
                    return;
                }

                // Retrieve the latest block height of the ledger.
                let latest_block_height = ledger.read().await.latest_block_height();
                // Iterate through the peers to check if this node needs to catch up.
                let mut maximal_peer = None;
                let mut maximum_block_height = 0;
                for (peer_ip, block_height) in self.block_height.iter() {
                    if *block_height > maximum_block_height {
                        maximal_peer = Some(*peer_ip);
                        maximum_block_height = *block_height;
                    }
                }
                // Proceed to add block requests if the maximum block height is higher than the latest.
                if let Some(peer_ip) = maximal_peer {
                    if maximum_block_height > latest_block_height {
                        let num_blocks = std::cmp::min(maximum_block_height - latest_block_height, 32);
                        debug!("Preparing to request the next {} blocks from {}", num_blocks, peer_ip);
                        self.is_syncing.store(true, Ordering::SeqCst);
                        // Add block requests for each block height up to the maximum block height.
                        for block_height in (latest_block_height + 1)..(latest_block_height + 1 + num_blocks) {
                            if !self.contains_block_request(peer_ip, block_height) {
                                // Add the block request to the state manager.
                                self.add_block_request(peer_ip, block_height);
                                // Request the next block from the peer.
                                let request = PeersRequest::MessageSend(peer_ip, Message::BlockRequest(block_height));
                                // Send a `BlockRequest` message to the peer.
                                if let Err(error) = peers_router.send(request).await {
                                    warn!("[BlockRequest] {}", error);
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
                // Sleep for 15 seconds.
                tokio::time::sleep(Duration::from_secs(15)).await;
                // Fetch the latest block height of this ledger.
                let block_height = ledger.read().await.latest_block_height();
                // Send a `Ping` message to the peer.
                let request = PeersRequest::MessageSend(peer_ip, Message::Ping(E::MESSAGE_VERSION, block_height));
                if let Err(error) = peers_router.send(request).await {
                    warn!("[Ping] {}", error);
                }
            }
            StateRequest::SyncRequest(peer_ip, peers_router) => {
                // Skip the sync request if this node is already syncing.
                if self.is_syncing() {
                    return;
                }
                // Process the sync request.
                else {
                    trace!("Received sync request from {}", peer_ip);
                    // Route a `SyncRequest` to the ledger.
                    let request = LedgerRequest::SyncRequest(peer_ip, peers_router.clone());
                    if let Err(error) = ledger_router.send(request).await {
                        warn!("[SyncRequest] {}", error);
                    }
                }
            }
            StateRequest::SyncResponse(peer_ip, block_locators) => {
                // Ensure the list of block locators is not empty.
                if block_locators.len() == 0 {
                    self.add_failure(peer_ip, "Received a sync response with no block locators".to_string());
                    return;
                }
                // Process the sync response.
                else {
                    trace!("Received sync response from {}", peer_ip);

                    // Construct a HashMap of the block locators.
                    let block_locators: HashMap<u32, N::BlockHash> = block_locators.iter().cloned().collect();
                    let (start_block_height, end_block_height) = match (block_locators.keys().min(), block_locators.keys().max()) {
                        (Some(min), Some(max)) => (*min, *max),
                        _ => {
                            error!("Failed to find the starting and ending block height in a sync response");
                            return;
                        }
                    };

                    // Ensure the block locators are linear.
                    for i in start_block_height..=end_block_height {
                        if !block_locators.contains_key(&i) {
                            self.add_failure(peer_ip, format!("Received a sync response missing a block locator for block {}", i));
                            return;
                        }
                    }

                    // Ensure the number of block locators is within the maximum fork depth.
                    if end_block_height - start_block_height + 1 > E::MAXIMUM_FORK_DEPTH {
                        self.add_failure(peer_ip, "Received a sync response that exceeds the maximum fork depth".to_string());
                        return;
                    }

                    // Acquire a reader for the ledger.
                    let ledger = ledger.read().await;

                    // Retrieve the latest block height of this ledger.
                    let latest_block_height = ledger.latest_block_height();
                    // Find the block hash, from the peer, corresponding to the latest block height.
                    if let Some(block_hash) = block_locators.get(&latest_block_height) {
                        // Retrieve the latest block hash of this ledger.
                        let latest_block_hash = ledger.latest_block_hash();
                        // Determine if the peer is a fork based on the block hash.
                        let is_fork = *block_hash == latest_block_hash;

                        // If the peer is on a fork, determine the block height that is the common ancestor.
                        match is_fork {
                            true => {
                                // Initialize a tracker of the common ancestor.
                                let mut common_ancestor = 0;

                                // TODO (howardwu): Clean up this logic. It should be working, however can be polished.

                                // Retrieve the block hashes of this ledger.
                                for block_height in start_block_height..=end_block_height {
                                    // Retrieve the block height from this ledger.
                                    // If the block height does not exist, this means we have found the common ancestor.
                                    let expected_block_hash = match ledger.get_block_hash(block_height) {
                                        Ok(block_hash) => block_hash,
                                        _ => match block_height == 0 {
                                            true => {
                                                common_ancestor = 0;
                                                break;
                                            }
                                            false => {
                                                common_ancestor = block_height - 1;
                                                break;
                                            }
                                        },
                                    };

                                    // Find the common ancestor of the two ledgers.
                                    if block_locators.get(&block_height) != Some(&expected_block_hash) {
                                        match block_height == 0 {
                                            true => {
                                                common_ancestor = 0;
                                                break;
                                            }
                                            false => {
                                                common_ancestor = block_height - 1;
                                                break;
                                            }
                                        }
                                    }
                                }

                                // Determine the common ancestor of the two ledgers.
                                match common_ancestor == 0 {
                                    true => {
                                        self.add_failure(peer_ip, "Peer has incorrect genesis block".to_string());
                                        // Update the ledger state of the peer.
                                        self.update_ledger_state(peer_ip, Some((true, Some(0), 0)));
                                    }
                                    false => {
                                        // Update the ledger state of the peer.
                                        self.update_ledger_state(peer_ip, Some((true, Some(common_ancestor), end_block_height)));
                                    }
                                }
                            }
                            // Update the ledger state of the peer.
                            false => self.update_ledger_state(peer_ip, Some((false, None, end_block_height))),
                        };
                    }
                }
            }
            StateRequest::UnconfirmedBlock(peer_ip, block_height, block) => {
                // Skip the request if this node is syncing.
                if self.is_syncing() {
                    return;
                }
                // Ensure the block height corresponds to the requested block.
                else if block_height != block.height() {
                    self.add_failure(peer_ip, "Block height does not match".to_string());
                }
                // Process the unconfirmed block.
                else {
                    trace!("Received unconfirmed block {} from {}", block_height, peer_ip);
                    // Route an `UnconfirmedBlock` to the ledger.
                    let request = LedgerRequest::UnconfirmedBlock(peer_ip, block.clone(), peers_router.clone());
                    if let Err(error) = ledger_router.send(request).await {
                        warn!("[UnconfirmedBlock] {}", error);
                    }
                }
            }
            StateRequest::UnconfirmedTransaction(peer_ip, transaction) => {
                // Skip the request if this node is syncing.
                if self.is_syncing() {
                    return;
                }
                // Process the unconfirmed transaction.
                else {
                    trace!("Received unconfirmed transaction {} from {}", transaction.transaction_id(), peer_ip);
                    // Route an `UnconfirmedTransaction` to the ledger.
                    let request = LedgerRequest::UnconfirmedTransaction(peer_ip, transaction, peers_router.clone());
                    if let Err(error) = ledger_router.send(request).await {
                        warn!("[UnconfirmedTransaction] {}", error);
                    }
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
        if !self.ledger_state.contains_key(&peer_ip) {
            self.ledger_state.insert(peer_ip, None);
        }
        if !self.block_requests.contains_key(&peer_ip) {
            self.block_requests.insert(peer_ip, Default::default());
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
        if self.ledger_state.contains_key(peer_ip) {
            self.ledger_state.remove(peer_ip);
        }
        if self.block_requests.contains_key(peer_ip) {
            self.block_requests.remove(peer_ip);
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
    /// Updates the ledger state of the given peer.
    ///
    fn update_ledger_state(&mut self, peer_ip: SocketAddr, is_forked: Option<(bool, Option<u32>, u32)>) {
        match self.ledger_state.get_mut(&peer_ip) {
            Some(status) => *status = is_forked,
            None => self.add_failure(peer_ip, format!("Missing ledger state for {}", peer_ip)),
        };
    }

    ///
    /// Adds a block request for the given block height to the specified peer.
    ///
    fn add_block_request(&mut self, peer_ip: SocketAddr, block_height: u32) {
        match self.block_requests.get_mut(&peer_ip) {
            Some(requests) => match requests.insert(block_height) {
                true => debug!("Requesting block {} from {}", block_height, peer_ip),
                false => self.add_failure(peer_ip, format!("Duplicate block request from {}", peer_ip)),
            },
            None => self.add_failure(peer_ip, format!("Missing block requests for {}", peer_ip)),
        };
    }

    ///
    /// Returns `true` if the block request for the given block height to the specified peer exists.
    ///
    fn contains_block_request(&self, peer_ip: SocketAddr, block_height: u32) -> bool {
        match self.block_requests.get(&peer_ip) {
            Some(requests) => requests.contains(&block_height),
            None => false,
        }
    }

    ///
    /// Removes a block request for the given block height to the specified peer.
    ///
    fn remove_block_request(&mut self, peer_ip: SocketAddr, block_height: u32) {
        match self.block_requests.get_mut(&peer_ip) {
            Some(requests) => {
                if !requests.remove(&block_height) {
                    self.add_failure(peer_ip, format!("Non-existent block request from {}", peer_ip))
                }
            }
            None => self.add_failure(peer_ip, format!("Missing block requests for {}", peer_ip)),
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
