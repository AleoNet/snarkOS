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
    /// BlockRequest := (peer_ip, start_block_height, end_block_height (inclusive))
    BlockRequest(SocketAddr, u32, u32),
    /// BlockResponse := (peer_ip, block_height, block)
    BlockResponse(SocketAddr, u32, Block<N>),
    /// Disconnect := (peer_ip)
    Disconnect(SocketAddr),
    /// Heartbeat := ()
    Heartbeat,
    /// Ping := (peer_ip, version, peers_router)
    Ping(SocketAddr, u32, PeersRouter<N, E>),
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
    /// The map of each peer to their ledger state := (is_fork, common_ancestor, latest_block_height).
    ledger_state: HashMap<SocketAddr, Option<(bool, u32, u32)>>,
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
            StateRequest::BlockRequest(peer_ip, start_block_height, end_block_height) => {
                // Ensure the request is within the tolerated limit.
                match end_block_height - start_block_height <= E::MAXIMUM_BLOCK_REQUEST {
                    true => {
                        match ledger.read().await.get_blocks(start_block_height, end_block_height) {
                            Ok(blocks) => {
                                for block in blocks {
                                    let request = PeersRequest::MessageSend(peer_ip, Message::BlockResponse(block.height(), block));
                                    if let Err(error) = peers_router.send(request).await {
                                        warn!("[BlockResponse] {}", error);
                                    }
                                }
                            }
                            Err(error) => {
                                error!("{}", error);
                                self.add_failure(peer_ip, format!("{}", error));
                                return;
                            }
                        }

                        // // Route a `BlockRequest` to the ledger.
                        // let request = LedgerRequest::BlockRequest(peer_ip, start_block_height, end_block_height, peers_router.clone());
                        // if let Err(error) = ledger_router.send(request).await {
                        //     warn!("[BlockRequest] {}", error);
                        // }
                    }
                    false => {
                        // Record the failed request from the peer.
                        let num_blocks = end_block_height - start_block_height;
                        let failure = format!("Attempted to request {} blocks", num_blocks);
                        warn!("{}", failure);
                        self.add_failure(peer_ip, failure);
                    }
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
                // Send a sync request to each connected peer.
                let request = PeersRequest::MessageBroadcast(Message::SyncRequest);
                // Send a `SyncRequest` message to the peer.
                if let Err(error) = peers_router.send(request).await {
                    warn!("[SyncRequest] {}", error);
                }

                // Ensure the state manager is not already syncing.
                if self.is_syncing() {
                    return;
                }

                // Iterate through the peers to check if this node needs to catch up.
                let mut maximal_peer = None;
                let mut maximal_peer_is_fork = false;
                let mut maximum_common_ancestor = 0;
                let mut maximum_block_height = 0;
                for (peer_ip, ledger_state) in self.ledger_state.iter() {
                    if let Some((is_fork, common_ancestor, block_height)) = ledger_state {
                        if *block_height > maximum_block_height {
                            maximal_peer = Some(*peer_ip);
                            maximal_peer_is_fork = *is_fork;
                            maximum_common_ancestor = *common_ancestor;
                            maximum_block_height = *block_height;
                        }
                    }
                }

                // Proceed to add block requests if the maximum block height is higher than the latest.
                if let Some(peer_ip) = maximal_peer {
                    // Retrieve the latest block height of the ledger.
                    let latest_block_height = ledger.read().await.latest_block_height();
                    if maximum_block_height > latest_block_height {
                        // // If the peer is on a fork, start by removing blocks until the common ancestor is reached.
                        // if maximal_peer_is_fork {
                        //     let num_blocks = latest_block_height - maximum_common_ancestor;
                        //     if num_blocks <= E::MAXIMUM_FORK_DEPTH {
                        //         if let Err(error) = ledger.write().await.remove_last_blocks(num_blocks) {
                        //             error!("Failed to roll ledger back: {}", error);
                        //         }
                        //     }
                        // }

                        // Determine the specific blocks to sync with the peer.
                        let num_blocks = std::cmp::min(maximum_block_height - latest_block_height, E::MAXIMUM_BLOCK_REQUEST);
                        let start_block_height = latest_block_height + 1;
                        let end_block_height = start_block_height + num_blocks - 1;

                        debug!(
                            "Preparing to request blocks {} to {} from {}",
                            start_block_height, end_block_height, peer_ip
                        );
                        self.is_syncing.store(true, Ordering::SeqCst);

                        // Add block requests for each block height up to the maximum block height.
                        for block_height in start_block_height..=end_block_height {
                            if !self.contains_block_request(peer_ip, block_height) {
                                // Add the block request to the state manager.
                                self.add_block_request(peer_ip, block_height);
                            }
                        }
                        // Request the blocks from the peer.
                        let request = PeersRequest::MessageSend(peer_ip, Message::BlockRequest(start_block_height, end_block_height));
                        // Send a `BlockRequest` message to the peer.
                        if let Err(error) = peers_router.send(request).await {
                            warn!("[BlockRequest] {}", error);
                        }
                    }
                }
            }
            StateRequest::Ping(peer_ip, version, peers_router) => {
                // Ensure the peer has been initialized in the state manager.
                self.initialize_peer(peer_ip, version);
                // Update the version number for the peer.
                self.update_version(peer_ip, version);
                // Send a `Pong` message to the peer.
                if let Err(error) = peers_router.send(PeersRequest::MessageSend(peer_ip, Message::Pong)).await {
                    warn!("[Pong] {}", error);
                }
            }
            StateRequest::Pong(peer_ip) => {
                // Sleep for the preset time before sending the next ping request.
                tokio::time::sleep(Duration::from_secs(E::PING_SLEEP_IN_SECS)).await;
                // Send a `Ping` message to the peer.
                let request = PeersRequest::MessageSend(peer_ip, Message::Ping(E::MESSAGE_VERSION));
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
                    // Acquire a reader for the ledger.
                    let ledger = ledger.read().await;

                    // Determine the common ancestor block height between this ledger and the peer.
                    let mut common_ancestor = 0;
                    // Determine the latest block height of the peer.
                    let mut latest_block_height_of_peer = 0;

                    // Verify the integrity of the block hashes sent by the peer.
                    for (block_height, block_hash) in block_locators {
                        // Ensure the block hash corresponds with the block height, if the block hash exists in this ledger.
                        if let Ok(expected_block_height) = ledger.get_block_height(&block_hash) {
                            if expected_block_height != block_height {
                                let error = format!("Invalid block height {} for block hash {}", expected_block_height, block_hash);
                                trace!("{}", error);
                                self.add_failure(peer_ip, error);
                                return;
                            } else {
                                // Update the common ancestor, as this block hash exists in this ledger.
                                if expected_block_height > common_ancestor {
                                    common_ancestor = expected_block_height
                                }
                            }
                        }

                        // Update the latest block height of the peer.
                        if block_height > latest_block_height_of_peer {
                            latest_block_height_of_peer = block_height;
                        }
                    }

                    // Ensure any potential fork is within the maximum fork depth.
                    if latest_block_height_of_peer - common_ancestor + 1 > E::MAXIMUM_FORK_DEPTH {
                        self.add_failure(peer_ip, "Received a sync response that exceeds the maximum fork depth".to_string());
                        return;
                    }

                    // TODO (howardwu): If the distance of (latest_block_height_of_peer - common_ancestor) is less than 10,
                    //  manually check the fork status 1 by 1, as a slow response from the peer could make it look like a fork, based on this simple logic.
                    // Determine if the peer is a fork.
                    let is_fork =
                        common_ancestor < ledger.latest_block_height() && latest_block_height_of_peer > ledger.latest_block_height();

                    trace!(
                        "{} is at block {} (is_fork = {}, common_ancestor = {})",
                        peer_ip,
                        latest_block_height_of_peer,
                        is_fork,
                        common_ancestor,
                    );

                    // Update the ledger state of the peer.
                    self.update_ledger_state(peer_ip, (is_fork, common_ancestor, latest_block_height_of_peer));

                    // // Construct a HashMap of the block locators.
                    // let block_locators: HashMap<u32, N::BlockHash> = block_locators.iter().cloned().collect();
                    // let (start_block_height, end_block_height) = match (block_locators.keys().min(), block_locators.keys().max()) {
                    //     (Some(min), Some(max)) => (*min, *max),
                    //     _ => {
                    //         error!("Failed to find the starting and ending block height in a sync response");
                    //         return;
                    //     }
                    // };
                    //
                }
            }
            StateRequest::UnconfirmedBlock(peer_ip, block_height, block) => {
                // Ensure the block height corresponds to the requested block.
                if block_height != block.height() {
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
                // Process the unconfirmed transaction.
                trace!("Received unconfirmed transaction {} from {}", transaction.transaction_id(), peer_ip);
                // Route an `UnconfirmedTransaction` to the ledger.
                let request = LedgerRequest::UnconfirmedTransaction(peer_ip, transaction, peers_router.clone());
                if let Err(error) = ledger_router.send(request).await {
                    warn!("[UnconfirmedTransaction] {}", error);
                }
            }
        }
    }

    ///
    /// Adds an entry for the given peer IP to every data structure in `State`.
    ///
    fn initialize_peer(&mut self, peer_ip: SocketAddr, version: u32) {
        if !self.version.contains_key(&peer_ip) {
            self.version.insert(peer_ip, version);
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
    /// Updates the ledger state of the given peer.
    ///
    fn update_ledger_state(&mut self, peer_ip: SocketAddr, ledger_state: (bool, u32, u32)) {
        match self.ledger_state.get_mut(&peer_ip) {
            Some(status) => *status = Some(ledger_state),
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
