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

use crate::{
    helpers::{block_requests::*, BlockRequest, CircularMap},
    PeersRequest,
    PeersRouter,
    ProverRequest,
    ProverRouter,
};
use snarkos_environment::{
    helpers::{block_locators::*, NodeType, State},
    network::{Data, DisconnectReason, Message},
    Environment,
};
use snarkos_storage::{storage::Storage, LedgerState};
use snarkvm::dpc::prelude::*;

#[cfg(any(feature = "test", feature = "prometheus"))]
use snarkos_metrics as metrics;

use anyhow::Result;
use std::{
    collections::HashMap,
    net::SocketAddr,
    path::Path,
    sync::{atomic::Ordering, Arc},
    time::{Duration, Instant},
};
use time::OffsetDateTime;
use tokio::{
    sync::{mpsc, oneshot, Mutex, RwLock},
    task,
};

/// The maximum number of unconfirmed blocks that can be held by the ledger.
const MAXIMUM_UNCONFIRMED_BLOCKS: u32 = 250;

pub type LedgerReader<N> = std::sync::Arc<snarkos_storage::LedgerState<N>>;

/// Shorthand for the parent half of the `Ledger` message channel.
pub type LedgerRouter<N> = mpsc::Sender<LedgerRequest<N>>;
#[allow(unused)]
/// Shorthand for the child half of the `Ledger` message channel.
type LedgerHandler<N> = mpsc::Receiver<LedgerRequest<N>>;

///
/// An enum of requests that the `Ledger` struct processes.
///
#[derive(Debug)]
pub enum LedgerRequest<N: Network> {
    /// BlockResponse := (peer_ip, block, prover_router)
    BlockResponse(SocketAddr, Block<N>, ProverRouter<N>),
    /// Disconnect := (peer_ip, reason)
    Disconnect(SocketAddr, DisconnectReason),
    /// Failure := (peer_ip, failure)
    Failure(SocketAddr, String),
    /// Heartbeat := (prover_router)
    Heartbeat(ProverRouter<N>),
    /// Pong := (peer_ip, node_type, status, is_fork, block_locators)
    Pong(SocketAddr, NodeType, State, Option<bool>, BlockLocators<N>, Option<Instant>),
    /// UnconfirmedBlock := (peer_ip, block, prover_router)
    UnconfirmedBlock(SocketAddr, Block<N>, ProverRouter<N>),
}

pub type PeersState<N> = HashMap<SocketAddr, Option<(NodeType, State, Option<bool>, u32, BlockLocators<N>)>>;

///
/// A ledger for a specific network on the node server.
///
#[derive(Debug)]
#[allow(clippy::type_complexity)]
pub struct Ledger<N: Network, E: Environment> {
    /// The ledger router of the node.
    ledger_router: LedgerRouter<N>,
    /// The canonical chain of blocks.
    canon: Arc<LedgerState<N>>,
    /// The canonical chain of blocks in read-only mode.
    canon_reader: Arc<LedgerState<N>>,
    /// A lock to ensure methods that need to be mutually-exclusive are enforced.
    /// In this context, `add_block`, and `revert_to_block_height` must be mutually-exclusive.
    canon_lock: Arc<Mutex<()>>,
    /// A map of previous block hashes to unconfirmed blocks.
    unconfirmed_blocks: RwLock<CircularMap<N::BlockHash, Block<N>, { MAXIMUM_UNCONFIRMED_BLOCKS }>>,
    /// The map of each peer to their ledger state := (node_type, status, is_fork, latest_block_height, block_locators).
    peers_state: RwLock<PeersState<N>>,
    /// The map of each peer to their block requests := HashMap<(block_height, block_hash), timestamp>
    block_requests: RwLock<HashMap<SocketAddr, HashMap<BlockRequest<N>, i64>>>,
    /// A lock to ensure methods that need to be mutually-exclusive are enforced.
    /// In this context, `update_ledger`, `add_block`, and `update_block_requests` must be mutually-exclusive.
    block_requests_lock: Arc<Mutex<()>>,
    /// The timestamp of the last successful block update.
    last_block_update_timestamp: RwLock<Instant>,
    /// The map of each peer to their failure messages := (failure_message, timestamp).
    failures: RwLock<HashMap<SocketAddr, Vec<(String, i64)>>>,
    /// The peers router of the node.
    peers_router: PeersRouter<N, E>,
}

impl<N: Network, E: Environment> Ledger<N, E> {
    /// Initializes a new instance of the ledger.
    pub async fn open<S: Storage, P: AsRef<Path> + Copy>(path: P, peers_router: PeersRouter<N, E>) -> Result<Arc<Self>> {
        // Initialize an mpsc channel for sending requests to the `Ledger` struct.
        let (ledger_router, mut ledger_handler) = mpsc::channel(1024);

        let canon = Arc::new(LedgerState::open_writer::<S, P>(path)?);
        let (canon_reader, reader_resource) = LedgerState::open_reader::<S, P>(path)?;
        // Register the thread; no need to provide an id, as it will run indefinitely.
        E::resources().register(reader_resource, None);

        // Initialize the ledger.
        let ledger = Arc::new(Self {
            ledger_router,
            canon,
            canon_reader,
            canon_lock: Arc::new(Mutex::new(())),
            unconfirmed_blocks: Default::default(),
            peers_state: Default::default(),
            block_requests: Default::default(),
            block_requests_lock: Arc::new(Mutex::new(())),
            last_block_update_timestamp: RwLock::new(Instant::now()),
            failures: Default::default(),
            peers_router,
        });

        // Initialize the handler for the ledger.
        {
            let ledger = ledger.clone();
            let (router, handler) = oneshot::channel();
            E::resources().register_task(
                None, // No need to provide an id, as the task will run indefinitely.
                task::spawn(async move {
                    // Notify the outer function that the task is ready.
                    let _ = router.send(());
                    // Asynchronously wait for a ledger request.
                    while let Some(request) = ledger_handler.recv().await {
                        // Update the state of the ledger.
                        // Note: Do not wrap this call in a `task::spawn` as `BlockResponse` messages
                        // will end up being processed out of order.
                        ledger.update(request).await;
                    }
                }),
            );

            // Wait until the ledger handler is ready.
            let _ = handler.await;
        }

        Ok(ledger)
    }

    /// Returns an instance of the ledger reader.
    pub fn reader(&self) -> LedgerReader<N> {
        self.canon_reader.clone()
    }

    /// Returns an instance of the ledger router.
    pub fn router(&self) -> LedgerRouter<N> {
        self.ledger_router.clone()
    }

    pub async fn shut_down(&self) {
        debug!("Ledger is shutting down...");

        // Set the terminator bit to `true` to ensure it stops mining.
        E::terminator().store(true, Ordering::SeqCst);
        trace!("[ShuttingDown] Terminator bit has been enabled");

        // Clear the unconfirmed blocks.
        self.unconfirmed_blocks.write().await.clear();
        trace!("[ShuttingDown] Pending queue has been cleared");

        // Disconnect all connected peers.
        let connected_peers = self.peers_state.read().await.keys().copied().collect::<Vec<_>>();
        for peer_ip in connected_peers {
            self.disconnect(peer_ip, DisconnectReason::ShuttingDown).await;
        }
        trace!("[ShuttingDown] Disconnect message has been sent to all connected peers");
    }

    ///
    /// Performs the given `request` to the ledger.
    /// All requests must go through this `update`, so that a unified view is preserved.
    ///
    pub(super) async fn update(&self, request: LedgerRequest<N>) {
        match request {
            LedgerRequest::BlockResponse(peer_ip, block, prover_router) => {
                // Remove the block request from the ledger.
                if self.remove_block_request(peer_ip, block.height()).await {
                    // On success, process the block response.
                    self.add_block(block, &prover_router).await;
                    // Check if syncing with this peer is complete.
                    if self
                        .block_requests
                        .read()
                        .await
                        .get(&peer_ip)
                        .map(|requests| requests.is_empty())
                        .unwrap_or(false)
                    {
                        trace!("All block requests with {} have been processed", peer_ip);
                        self.update_block_requests().await;
                    }
                }
            }
            LedgerRequest::Disconnect(peer_ip, reason) => {
                self.disconnect(peer_ip, reason).await;
            }
            LedgerRequest::Failure(peer_ip, failure) => {
                self.add_failure(peer_ip, failure).await;
            }
            LedgerRequest::Heartbeat(prover_router) => {
                // Update for sync nodes.
                self.update_sync_nodes().await;
                // Update the ledger.
                self.update_ledger(&prover_router).await;
                // Update the status of the ledger.
                self.update_status().await;
                // Remove expired block requests.
                self.remove_expired_block_requests().await;
                // Remove expired failures.
                self.remove_expired_failures().await;
                // Disconnect from peers with frequent failures.
                self.disconnect_from_failing_peers().await;
                // Update the block requests.
                self.update_block_requests().await;

                let block_requests = self.number_of_block_requests().await;
                let connected_peers = self.peers_state.read().await.len();

                debug!(
                    "Status Report (type = {}, status = {}, block_height = {}, cumulative_weight = {}, block_requests = {}, connected_peers = {})",
                    E::NODE_TYPE,
                    E::status(),
                    self.canon.latest_block_height(),
                    self.canon.latest_cumulative_weight(),
                    block_requests,
                    connected_peers,
                );
            }
            LedgerRequest::Pong(peer_ip, node_type, status, is_fork, block_locators, _rtt_start) => {
                // Ensure the peer has been initialized in the ledger.
                self.initialize_peer(peer_ip).await;
                // Process the pong.
                self.update_peer(peer_ip, node_type, status, is_fork, block_locators).await;

                // Stop the clock on internal RTT.
                #[cfg(any(feature = "test", feature = "prometheus"))]
                metrics::histogram!(
                    metrics::internal_rtt::PONG,
                    _rtt_start.expect("rtt should be present with metrics enabled").elapsed()
                );
            }
            LedgerRequest::UnconfirmedBlock(peer_ip, block, prover_router) => {
                // Ensure the node is not peering.
                if !E::status().is_peering() {
                    // Process the unconfirmed block.
                    self.add_block(block.clone(), &prover_router).await;
                    // Propagate the unconfirmed block to the connected peers.
                    let message = Message::UnconfirmedBlock(block.height(), block.hash(), Data::Object(block));
                    let request = PeersRequest::MessagePropagate(peer_ip, message);
                    if let Err(error) = self.peers_router.send(request).await {
                        warn!("[UnconfirmedBlock] {}", error);
                    }
                }
            }
        }
    }

    ///
    /// Disconnects the given peer from the ledger.
    ///
    pub async fn disconnect(&self, peer_ip: SocketAddr, reason: DisconnectReason) {
        info!("Disconnecting from {} ({:?})", peer_ip, reason);
        // Remove all entries of the peer from the ledger.
        self.remove_peer(&peer_ip).await;
        // Update the status of the ledger.
        self.update_status().await;
        // Send a `Disconnect` message to the peer.
        if let Err(error) = self
            .peers_router
            .send(PeersRequest::MessageSend(peer_ip, Message::Disconnect(reason)))
            .await
        {
            warn!("[Disconnect] {}", error);
        }
        // Route a `PeerDisconnected` to the peers.
        if let Err(error) = self.peers_router.send(PeersRequest::PeerDisconnected(peer_ip)).await {
            warn!("[PeerDisconnected] {}", error);
        }
    }

    ///
    /// Disconnects and restricts the given peer from the ledger.
    ///
    async fn disconnect_and_restrict(&self, peer_ip: SocketAddr, reason: DisconnectReason) {
        info!("Disconnecting and restricting {} ({:?})", peer_ip, reason);
        // Remove all entries of the peer from the ledger.
        self.remove_peer(&peer_ip).await;
        // Update the status of the ledger.
        self.update_status().await;
        // Send a `Disconnect` message to the peer.
        if let Err(error) = self
            .peers_router
            .send(PeersRequest::MessageSend(peer_ip, Message::Disconnect(reason)))
            .await
        {
            warn!("[Disconnect] {}", error);
        }
        // Route a `PeerRestricted` to the peers.
        if let Err(error) = self.peers_router.send(PeersRequest::PeerRestricted(peer_ip)).await {
            warn!("[PeerRestricted] {}", error);
        }
    }

    ///
    /// Performs a heartbeat update for the sync nodes.
    ///
    async fn update_sync_nodes(&self) {
        if E::NODE_TYPE == NodeType::Sync {
            // Lock peers_state for further processing.
            let peers_state = self.peers_state.read().await;

            // Retrieve the latest cumulative weight of this ledger.
            let latest_cumulative_weight = self.canon.latest_cumulative_weight();

            // Initialize a list of peers to disconnect from.
            let mut peer_ips_to_disconnect = Vec::with_capacity(peers_state.len());

            // Check if any of the peers are ahead and have a larger block height.
            for (peer_ip, peer_state) in peers_state.iter() {
                if let Some((node_type, status, Some(_), block_height, block_locators)) = peer_state {
                    // Retrieve the cumulative weight, defaulting to the block height if it does not exist.
                    let cumulative_weight = match block_locators.get_cumulative_weight(*block_height) {
                        Some(cumulative_weight) => cumulative_weight,
                        None => *block_height as u128,
                    };

                    // If the peer is not a sync node and is syncing, and the peer is ahead, proceed to disconnect.
                    if *node_type != NodeType::Sync && *status == State::Syncing && cumulative_weight > latest_cumulative_weight {
                        // Append the peer to the list of disconnects.
                        peer_ips_to_disconnect.push(*peer_ip);
                    }
                }
            }

            // Release the lock over peers_state.
            drop(peers_state);

            trace!("Found {} peers to disconnect", peer_ips_to_disconnect.len());

            // Proceed to disconnect and restrict these peers.
            for peer_ip in peer_ips_to_disconnect {
                self.disconnect_and_restrict(peer_ip, DisconnectReason::SyncComplete).await;
            }
        }
    }

    ///
    /// Attempt to fast-forward the ledger with unconfirmed blocks.
    ///
    async fn update_ledger(&self, prover_router: &ProverRouter<N>) {
        // Check for candidate blocks to fast forward the ledger.
        let mut block_hash = self.canon.latest_block_hash();
        let unconfirmed_blocks_snapshot = self.unconfirmed_blocks.read().await.clone();
        while let Some(unconfirmed_block) = unconfirmed_blocks_snapshot.get(&block_hash) {
            // Attempt to add the unconfirmed block.
            match self.add_block(unconfirmed_block.clone(), prover_router).await {
                // Upon success, update the block hash iterator.
                true => block_hash = unconfirmed_block.hash(),
                false => break,
            }
        }

        // If the timestamp of the last block increment has surpassed the preset limit,
        // the ledger is likely syncing from invalid state, and should revert by one block.
        if E::status().is_syncing()
            && self.last_block_update_timestamp.read().await.elapsed() > 2 * Duration::from_secs(E::RADIO_SILENCE_IN_SECS)
        {
            // Acquire the lock for block requests.
            let _block_request_lock = self.block_requests_lock.lock().await;

            trace!("Ledger state has become stale, clearing queue and reverting by one block");
            self.unconfirmed_blocks.write().await.clear();

            // Reset the memory pool of its transactions.
            if let Err(error) = prover_router.send(ProverRequest::MemoryPoolClear(None)).await {
                error!("[MemoryPoolClear]: {}", error);
            }

            self.block_requests
                .write()
                .await
                .values_mut()
                .for_each(|requests| *requests = Default::default());
            self.revert_to_block_height(self.canon.latest_block_height().saturating_sub(1))
                .await;
        }
    }

    ///
    /// Updates the status of the ledger.
    ///
    async fn update_status(&self) {
        // Retrieve the status variable.
        let mut status = E::status().get();

        // If the node is shutting down, skip the update.
        if status == State::ShuttingDown {
            trace!("Ledger is shutting down");
            // Set the terminator bit to `true` to ensure it stops mining.
            E::terminator().store(true, Ordering::SeqCst);
            return;
        }
        // If there is an insufficient number of connected peers, set the status to `Peering`.
        else if self.peers_state.read().await.len() < E::MINIMUM_NUMBER_OF_PEERS {
            status = State::Peering;
        }
        // If the ledger is out of date, set the status to `Syncing`.
        else {
            // Update the status to `Ready` or `Mining`.
            status = match status {
                State::Mining => State::Mining,
                _ => State::Ready,
            };

            // Retrieve the latest cumulative weight of this node.
            let latest_cumulative_weight = self.canon.latest_cumulative_weight();
            // Iterate through the connected peers, to determine if the ledger state is out of date.
            for (_, peer_state) in self.peers_state.read().await.iter() {
                if let Some((_, _, Some(_), block_height, block_locators)) = peer_state {
                    // Retrieve the cumulative weight, defaulting to the block height if it does not exist.
                    let cumulative_weight = match block_locators.get_cumulative_weight(*block_height) {
                        Some(cumulative_weight) => cumulative_weight,
                        None => *block_height as u128,
                    };
                    // If the cumulative weight is greater than MAXIMUM_LINEAR_BLOCK_LOCATORS, set the status to `Syncing`.
                    if cumulative_weight.saturating_sub(latest_cumulative_weight) > MAXIMUM_LINEAR_BLOCK_LOCATORS as u128 {
                        // Set the status to `Syncing`.
                        status = State::Syncing;
                        break;
                    }
                }
            }
        }

        // If the node is `Peering` or `Syncing`, it should not be mining.
        if status == State::Peering || status == State::Syncing {
            // Set the terminator bit to `true` to ensure it does not mine.
            E::terminator().store(true, Ordering::SeqCst);
        } else {
            // Set the terminator bit to `false` to ensure it is allowed to mine.
            E::terminator().store(false, Ordering::SeqCst);
        }

        // Update the ledger to the determined status.
        E::status().update(status);
    }

    ///
    /// Adds the given block:
    ///     1) as the next block in the ledger if the block height increments by one, or
    ///     2) to the pending queue for later use.
    ///
    /// Returns `true` if the given block is successfully added to the *canon* chain.
    ///
    async fn add_block(&self, unconfirmed_block: Block<N>, prover_router: &ProverRouter<N>) -> bool {
        // Retrieve the unconfirmed block height.
        let unconfirmed_block_height = unconfirmed_block.height();
        // Retrieve the unconfirmed block hash.
        let unconfirmed_block_hash = unconfirmed_block.hash();
        // Retrieve the unconfirmed previous block hash.
        let unconfirmed_previous_block_hash = unconfirmed_block.previous_block_hash();

        // Ensure the given block is new.
        if let Ok(true) = self.canon.contains_block_hash(&unconfirmed_block_hash) {
            trace!(
                "Canonical chain already contains block {} ({})",
                unconfirmed_block_height,
                unconfirmed_block_hash
            );
        } else if unconfirmed_block_height == self.canon.latest_block_height() + 1
            && unconfirmed_previous_block_hash == self.canon.latest_block_hash()
        {
            // Acquire the lock for block requests.
            let _block_requests_lock = self.block_requests_lock.lock().await;
            // Acquire the lock for the canon chain.
            let _canon_lock = self.canon_lock.lock().await;

            // Ensure the block height is not part of a block request on a fork.
            let mut is_block_on_fork = false;
            'outer: for requests in self.block_requests.read().await.values() {
                for request in requests.keys() {
                    // If the unconfirmed block conflicts with a requested block on a fork, skip.
                    if request.block_height() == unconfirmed_block_height {
                        if let Some(requested_block_hash) = request.block_hash() {
                            if unconfirmed_block.hash() != requested_block_hash {
                                is_block_on_fork = true;
                                break 'outer;
                            }
                        }
                    }
                }
            }

            // If the unconfirmed block is not on a fork, attempt to add it as the next block.
            match is_block_on_fork {
                // Filter out the undesirable unconfirmed blocks, if it exists.
                true => self.unconfirmed_blocks.write().await.remove(&unconfirmed_previous_block_hash),
                // Attempt to add the unconfirmed block as the next block in the canonical chain.
                false => match self.canon.add_next_block(&unconfirmed_block) {
                    Ok(()) => {
                        let latest_block_height = self.canon.latest_block_height();
                        info!(
                            "Ledger successfully advanced to block {} ({})",
                            latest_block_height,
                            self.canon.latest_block_hash()
                        );

                        #[cfg(any(feature = "test", feature = "prometheus"))]
                        metrics::gauge!(metrics::blocks::HEIGHT, latest_block_height as f64);

                        // Update the timestamp of the last block increment.
                        *self.last_block_update_timestamp.write().await = Instant::now();
                        // Set the terminator bit to `true` to ensure the miner updates state.
                        E::terminator().store(true, Ordering::SeqCst);
                        // On success, filter the unconfirmed blocks of this block, if it exists.
                        self.unconfirmed_blocks.write().await.remove(&unconfirmed_previous_block_hash);

                        // On success, filter the memory pool of its transactions, if they exist.
                        if let Err(error) = prover_router.send(ProverRequest::MemoryPoolClear(Some(unconfirmed_block))).await {
                            error!("[MemoryPoolClear]: {}", error);
                        }

                        return true;
                    }
                    Err(error) => warn!("{}", error),
                },
            }
        } else {
            // Add the block to the unconfirmed blocks.
            if self
                .unconfirmed_blocks
                .write()
                .await
                .insert(unconfirmed_previous_block_hash, unconfirmed_block)
            {
                trace!("Added unconfirmed block {} to the pending queue", unconfirmed_block_height);
            } else {
                trace!(
                    "Pending queue already contains unconfirmed block {} ({})",
                    unconfirmed_block_height,
                    unconfirmed_block_hash
                );
            }
        }
        false
    }

    ///
    /// Reverts the ledger state back to height `block_height`, returning `true` on success.
    ///
    async fn revert_to_block_height(&self, block_height: u32) -> bool {
        // Acquire the lock for the canon chain.
        let _canon_lock = self.canon_lock.lock().await;

        match self.canon.revert_to_block_height(block_height) {
            Ok(removed_blocks) => {
                let latest_block_height = self.canon.latest_block_height();
                info!("Ledger successfully reverted to block {}", latest_block_height);

                #[cfg(any(feature = "test", feature = "prometheus"))]
                metrics::gauge!(metrics::blocks::HEIGHT, latest_block_height as f64);

                // Update the last block update timestamp.
                *self.last_block_update_timestamp.write().await = Instant::now();
                // Set the terminator bit to `true` to ensure the miner resets state.
                E::terminator().store(true, Ordering::SeqCst);

                // Lock unconfirmed_blocks for further processing.
                let mut unconfirmed_blocks = self.unconfirmed_blocks.write().await;

                // Ensure the removed blocks are not in the unconfirmed blocks.
                for removed_block in removed_blocks {
                    unconfirmed_blocks.remove(&removed_block.previous_block_hash());
                }
                true
            }
            Err(error) => {
                warn!("{}", error);

                // Set the terminator bit to `true` to ensure the miner resets state.
                E::terminator().store(true, Ordering::SeqCst);
                // Reset the unconfirmed blocks.
                self.unconfirmed_blocks.write().await.clear();

                false
            }
        }
    }

    ///
    /// Adds an entry for the given peer IP to every data structure in `State`.
    ///
    async fn initialize_peer(&self, peer_ip: SocketAddr) {
        // Since the peer state already existing is the most probable scenario,
        // use a read() first to avoid using write() if possible.
        let peer_state_exists = self.peers_state.read().await.contains_key(&peer_ip);

        if !peer_state_exists {
            self.peers_state.write().await.entry(peer_ip).or_insert(None);
            self.block_requests.write().await.entry(peer_ip).or_insert_with(Default::default);
            self.failures.write().await.entry(peer_ip).or_insert_with(Default::default);
        }
    }

    ///
    /// Removes the entry for the given peer IP from every data structure in `State`.
    ///
    async fn remove_peer(&self, peer_ip: &SocketAddr) {
        self.peers_state.write().await.remove(peer_ip);
        self.block_requests.write().await.remove(peer_ip);
        self.failures.write().await.remove(peer_ip);
    }

    ///
    /// Updates the state of the given peer.
    ///
    async fn update_peer(
        &self,
        peer_ip: SocketAddr,
        node_type: NodeType,
        status: State,
        is_fork: Option<bool>,
        block_locators: BlockLocators<N>,
    ) {
        // Ensure the list of block locators is not empty.
        if block_locators.is_empty() {
            self.add_failure(peer_ip, "Received a sync response with no block locators".to_string())
                .await;
        } else {
            // Ensure the peer provided well-formed block locators.
            match self.canon.check_block_locators(&block_locators) {
                Ok(is_valid) => {
                    if !is_valid {
                        warn!("Invalid block locators from {}", peer_ip);
                        self.add_failure(peer_ip, "Invalid block locators".to_string()).await;
                        return;
                    }
                }
                Err(error) => warn!("Error checking block locators: {}", error),
            };

            // Determine the common ancestor block height between this ledger and the peer.
            let mut common_ancestor = 0;
            // Determine the latest block height of the peer.
            let mut latest_block_height_of_peer = 0;

            // Verify the integrity of the block hashes sent by the peer.
            for (block_height, (block_hash, _)) in block_locators.iter() {
                // Ensure the block hash corresponds with the block height, if the block hash exists in this ledger.
                if let Ok(expected_block_height) = self.canon.get_block_height(block_hash) {
                    if expected_block_height != *block_height {
                        let error = format!("Invalid block height {} for block hash {}", expected_block_height, block_hash);
                        trace!("{}", error);
                        self.add_failure(peer_ip, error).await;
                        return;
                    } else {
                        // Update the common ancestor, as this block hash exists in this ledger.
                        if expected_block_height > common_ancestor {
                            common_ancestor = expected_block_height
                        }
                    }
                }

                // Update the latest block height of the peer.
                if *block_height > latest_block_height_of_peer {
                    latest_block_height_of_peer = *block_height;
                }
            }

            // If the given fork status is None, check if it can be updated.
            let is_fork = match is_fork {
                Some(is_fork) => Some(is_fork),
                None => match common_ancestor == latest_block_height_of_peer || common_ancestor == self.canon.latest_block_height() {
                    // If the common ancestor matches the latest block height of (the peer || this node),
                    // the peer (is clearly || is likely) on the same canonical chain as this node.
                    true => Some(false),
                    false => None,
                },
            };

            let fork_status = match is_fork {
                Some(boolean) => format!("{}", boolean),
                None => "undecided".to_string(),
            };
            let cumulative_weight = match block_locators.get_cumulative_weight(latest_block_height_of_peer) {
                Some(weight) => format!("{}", weight),
                _ => "unknown".to_string(),
            };
            debug!(
                "Peer {} is at block {} (type = {}, status = {}, is_fork = {}, cumulative_weight = {}, common_ancestor = {})",
                peer_ip, latest_block_height_of_peer, node_type, status, fork_status, cumulative_weight, common_ancestor,
            );

            match self.peers_state.write().await.get_mut(&peer_ip) {
                Some(peer_state) => *peer_state = Some((node_type, status, is_fork, latest_block_height_of_peer, block_locators)),
                None => self.add_failure(peer_ip, format!("Missing ledger state for {}", peer_ip)).await,
            };
        }
    }

    ///
    /// Proceeds to send block requests to a connected peer, if the ledger is out of date.
    ///
    async fn update_block_requests(&self) {
        // Ensure the ledger is not awaiting responses from outstanding block requests.
        if self.number_of_block_requests().await > 0 {
            return;
        }

        // Retrieve the latest block height and cumulative weight of this ledger.
        let latest_block_height = self.canon.latest_block_height();
        let latest_cumulative_weight = self.canon.latest_cumulative_weight();

        // Iterate through the peers to check if this node needs to catch up, and determine a peer to sync with.
        // Prioritize the sync nodes before regular peers.
        let mut maximum_block_height = latest_block_height;
        let mut maximum_cumulative_weight = latest_cumulative_weight;

        // Check if any of the peers are ahead and have a larger block height.
        if let Some((peer_ip, maximal_peer_is_on_fork, maximum_block_locators)) = find_maximal_peer::<N, E>(
            &*self.peers_state.read().await,
            &mut maximum_block_height,
            &mut maximum_cumulative_weight,
        ) {
            // Case 1 - Ensure the peer has a heavier canonical chain than this ledger.
            // Note: this check is duplicated in `handle_block_requests`, as it is fast
            // and allows us to skip acquiring `_block_requests_lock`.
            if latest_cumulative_weight >= maximum_cumulative_weight {
                return;
            }

            // Acquire the lock for block requests.
            let _block_requests_lock = self.block_requests_lock.lock().await;

            // Determine the common ancestor block height between this ledger and the peer
            // and the first locator (smallest height) that does not exist in this ledger.
            let (maximum_common_ancestor, first_deviating_locator) = match find_common_ancestor(&self.canon, &maximum_block_locators) {
                Ok(ret) => ret,
                Err(error) => {
                    trace!("{}", error);
                    self.add_failure(peer_ip, error).await;
                    return;
                }
            };

            // Case 2 - Prepare to send block requests, as the peer is ahead of this ledger.
            let (start_block_height, end_block_height, ledger_is_on_fork) = match handle_block_requests::<N, E>(
                latest_block_height,
                latest_cumulative_weight,
                peer_ip,
                Some(maximal_peer_is_on_fork),
                maximum_block_height,
                maximum_cumulative_weight,
                maximum_common_ancestor,
                first_deviating_locator,
            ) {
                // Abort from the block request update.
                BlockRequestHandler::Abort(_) => return,
                // Disconnect from the peer if it is misbehaving and proceed to abort.
                BlockRequestHandler::AbortAndDisconnect(_, reason) => {
                    drop(_block_requests_lock);
                    self.disconnect(peer_ip, reason).await;
                    return;
                }
                // Proceed to send block requests to a connected peer, if the ledger is out of date.
                BlockRequestHandler::Proceed(_, proceed) => {
                    (proceed.start_block_height, proceed.end_block_height, proceed.ledger_is_on_fork)
                }
            };

            // Revert the ledger, if it is on a fork.
            if ledger_is_on_fork {
                // If the revert operation fails, abort.
                if !self.revert_to_block_height(maximum_common_ancestor).await {
                    warn!("Ledger failed to revert to block {}", maximum_common_ancestor);
                    return;
                }
            }

            // Send a `BlockRequest` message to the peer.
            debug!("Requesting blocks {} to {} from {}", start_block_height, end_block_height, peer_ip);
            let request = PeersRequest::MessageSend(peer_ip, Message::BlockRequest(start_block_height, end_block_height));
            if let Err(error) = self.peers_router.send(request).await {
                warn!("[BlockRequest] {}", error);
                return;
            }

            // Filter out any pre-existing block requests for the peer.
            let mut missing_block_requests = false;
            let mut new_block_heights = Vec::new();
            if let Some(block_requests) = self.block_requests.read().await.get(&peer_ip) {
                for block_height in start_block_height..=end_block_height {
                    if !block_requests.contains_key(&block_height.into()) {
                        new_block_heights.push(block_height);
                    }
                }
            } else {
                self.add_failure(peer_ip, format!("Missing block requests for {}", peer_ip)).await;
                missing_block_requests = true;
            }

            if !missing_block_requests && !new_block_heights.is_empty() {
                // Log each block request to ensure the peer responds with all requested blocks.
                if let Some(locked_block_requests) = self.block_requests.write().await.get_mut(&peer_ip) {
                    for block_height in new_block_heights {
                        // If the ledger is on a fork and was reverted, include the expected new block hash for the fork.
                        match ledger_is_on_fork {
                            true => {
                                self.add_block_request(
                                    peer_ip,
                                    block_height,
                                    maximum_block_locators.get_block_hash(block_height),
                                    locked_block_requests,
                                )
                                .await
                            }
                            false => self.add_block_request(peer_ip, block_height, None, locked_block_requests).await,
                        };
                    }
                }
            }
        }
    }

    ///
    /// Returns the number of outstanding block requests.
    ///
    async fn number_of_block_requests(&self) -> usize {
        self.block_requests.read().await.values().map(|r| r.len()).sum()
    }

    ///
    /// Adds a block request for the given block height to the specified peer.
    ///
    async fn add_block_request(
        &self,
        peer_ip: SocketAddr,
        block_height: u32,
        block_hash: Option<N::BlockHash>,
        locked_block_requests: &mut HashMap<BlockRequest<N>, i64>,
    ) {
        match locked_block_requests.insert((block_height, block_hash).into(), OffsetDateTime::now_utc().unix_timestamp()) {
            None => debug!("Requesting block {} from {}", block_height, peer_ip),
            Some(_old_request) => self.add_failure(peer_ip, format!("Duplicate block request for {}", peer_ip)).await,
        }
    }

    ///
    /// Returns `true` if the block request for the given block height to the specified peer exists.
    ///
    async fn contains_block_request(&self, peer_ip: SocketAddr, block_height: u32) -> bool {
        match self.block_requests.read().await.get(&peer_ip) {
            Some(requests) => requests.contains_key(&block_height.into()),
            None => false,
        }
    }

    ///
    /// Removes a block request for the given block height to the specified peer.
    /// On success, returns `true`, otherwise returns `false`.
    ///
    async fn remove_block_request(&self, peer_ip: SocketAddr, block_height: u32) -> bool {
        // Ensure the block height corresponds to a requested block.
        if !self.contains_block_request(peer_ip, block_height).await {
            self.add_failure(peer_ip, "Received an invalid block response".to_string()).await;
            false
        } else {
            if let Some(requests) = self.block_requests.write().await.get_mut(&peer_ip) {
                let is_success = requests.remove(&block_height.into()).is_some();
                match is_success {
                    true => return true,
                    false => {
                        self.add_failure(peer_ip, format!("Non-existent block request from {}", peer_ip))
                            .await
                    }
                }
            }
            false
        }
    }

    ///
    /// Removes block requests that have expired.
    ///
    async fn remove_expired_block_requests(&self) {
        // Clear all block requests that have lived longer than `E::RADIO_SILENCE_IN_SECS`.
        let now = OffsetDateTime::now_utc().unix_timestamp();
        self.block_requests.write().await.iter_mut().for_each(|(_peer, block_requests)| {
            block_requests.retain(|_, time_of_request| now.saturating_sub(*time_of_request) < E::RADIO_SILENCE_IN_SECS as i64)
        });
    }

    ///
    /// Adds the given failure message to the specified peer IP.
    ///
    async fn add_failure(&self, peer_ip: SocketAddr, failure: String) {
        trace!("Adding failure for {}: {}", peer_ip, failure);
        match self.failures.write().await.get_mut(&peer_ip) {
            Some(failures) => failures.push((failure, OffsetDateTime::now_utc().unix_timestamp())),
            None => error!("Missing failure entry for {}", peer_ip),
        };
    }

    ///
    /// Removes failures that have expired.
    ///
    async fn remove_expired_failures(&self) {
        // Clear all failures that have lived longer than `E::FAILURE_EXPIRY_TIME_IN_SECS`.
        let now = OffsetDateTime::now_utc().unix_timestamp();
        self.failures.write().await.iter_mut().for_each(|(_, failures)| {
            failures.retain(|(_, time_of_fail)| now.saturating_sub(*time_of_fail) < E::FAILURE_EXPIRY_TIME_IN_SECS as i64)
        });
    }

    ///
    /// Disconnects from connected peers who exhibit frequent failures.
    ///
    async fn disconnect_from_failing_peers(&self) {
        let peers_to_disconnect = self
            .failures
            .read()
            .await
            .iter()
            .filter(|(_, failures)| failures.len() > E::MAXIMUM_NUMBER_OF_FAILURES)
            .map(|(peer_ip, _)| *peer_ip)
            .collect::<Vec<_>>();

        for peer_ip in peers_to_disconnect {
            self.disconnect(peer_ip, DisconnectReason::TooManyFailures).await;
        }
    }
}
