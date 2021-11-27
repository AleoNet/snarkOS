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
    helpers::{CircularMap, State, Status, Tasks},
    Data,
    Environment,
    LedgerReader,
    Message,
    NodeType,
    PeersRequest,
    PeersRouter,
    ProverRequest,
    ProverRouter,
};
use snarkos_ledger::{storage::Storage, BlockLocators, LedgerState};
use snarkvm::dpc::prelude::*;

use anyhow::Result;
use chrono::Utc;
use std::{
    collections::{HashMap, HashSet},
    hash::{Hash, Hasher},
    net::SocketAddr,
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
use tokio::{
    runtime::Runtime,
    sync::{mpsc, oneshot, Mutex, RwLock},
    task,
    task::JoinHandle,
};

/// The maximum number of unconfirmed blocks that can be held by the ledger.
const MAXIMUM_UNCONFIRMED_BLOCKS: u32 = 100;

/// Shorthand for the parent half of the `Ledger` message channel.
pub(crate) type LedgerRouter<N> = mpsc::Sender<LedgerRequest<N>>;
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
    /// Disconnect := (peer_ip)
    Disconnect(SocketAddr),
    /// Failure := (peer_ip, failure)
    Failure(SocketAddr, String),
    /// Heartbeat := (prover_router)
    Heartbeat(ProverRouter<N>),
    /// Pong := (peer_ip, node_type, status, is_fork, block_locators)
    Pong(SocketAddr, NodeType, State, Option<bool>, BlockLocators<N>),
    /// UnconfirmedBlock := (peer_ip, block, prover_router)
    UnconfirmedBlock(SocketAddr, Block<N>, ProverRouter<N>),
}

///
/// A request for a block with the specified height and possibly a hash.
///
#[derive(Clone, Debug)]
pub struct BlockRequest<N: Network> {
    height: u32,
    hash: Option<N::BlockHash>,
}

// The height is the primary key, so use only it for hashing purposes.
impl<N: Network> PartialEq for BlockRequest<N> {
    fn eq(&self, other: &Self) -> bool {
        self.height == other.height
    }
}

impl<N: Network> Eq for BlockRequest<N> {}

// The k1 == k2 -> hash(k1) == hash(k2) rule must hold.
impl<N: Network> Hash for BlockRequest<N> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.height.hash(state);
    }
}

impl<N: Network> From<u32> for BlockRequest<N> {
    fn from(height: u32) -> Self {
        Self { height, hash: None }
    }
}

impl<N: Network> From<(u32, Option<N::BlockHash>)> for BlockRequest<N> {
    fn from((height, hash): (u32, Option<N::BlockHash>)) -> Self {
        Self { height, hash }
    }
}

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
    #[allow(unused)]
    canon_reader: Arc<LedgerState<N>>,
    /// A map of previous block hashes to unconfirmed blocks.
    unconfirmed_blocks: RwLock<CircularMap<N::BlockHash, Block<N>, { MAXIMUM_UNCONFIRMED_BLOCKS }>>,
    /// The map of each peer to their ledger state := (node_type, status, is_fork, latest_block_height, block_locators).
    peers_state: RwLock<HashMap<SocketAddr, Option<(NodeType, State, Option<bool>, u32, BlockLocators<N>)>>>,
    /// The map of each peer to their block requests := HashMap<(block_height, block_hash), timestamp>
    block_requests: RwLock<HashMap<SocketAddr, HashMap<BlockRequest<N>, i64>>>,
    /// A lock to ensure methods that need to be mutually-exclusive are enforced.
    /// In this context, `update_ledger`, `add_block`, and `update_block_requests` must be mutually-exclusive.
    block_requests_lock: Mutex<()>,
    /// The timestamp of the last successful block update.
    last_block_update_timestamp: RwLock<Instant>,
    /// The map of each peer to their failure messages := (failure_message, timestamp).
    failures: RwLock<HashMap<SocketAddr, Vec<(String, i64)>>>,
    /// The status of the node.
    status: Status,
    /// A terminator bit for the prover.
    terminator: Arc<AtomicBool>,
    /// The peers router of the node.
    peers_router: PeersRouter<N, E>,
}

impl<N: Network, E: Environment> Ledger<N, E> {
    /// Initializes a new instance of the ledger.
    pub async fn open<S: Storage, P: AsRef<Path> + Copy>(
        tasks: &mut Tasks<JoinHandle<()>>,
        path: P,
        status: &Status,
        terminator: &Arc<AtomicBool>,
        peers_router: PeersRouter<N, E>,
    ) -> Result<Arc<Self>> {
        // Initialize an mpsc channel for sending requests to the `Ledger` struct.
        let (ledger_router, mut ledger_handler) = mpsc::channel(1024);

        // Initialize the ledger.
        let ledger = Arc::new(Self {
            ledger_router,
            canon: Arc::new(LedgerState::open_writer::<S, P>(path)?),
            canon_reader: LedgerState::open_reader::<S, P>(path)?,
            unconfirmed_blocks: Default::default(),
            peers_state: Default::default(),
            block_requests: Default::default(),
            block_requests_lock: Mutex::new(()),
            last_block_update_timestamp: RwLock::new(Instant::now()),
            failures: Default::default(),
            status: status.clone(),
            terminator: terminator.clone(),
            peers_router,
        });

        // Initialize the handler for the ledger.
        {
            let ledger = ledger.clone();
            let (router, handler) = oneshot::channel();
            tasks.append(task::spawn(async move {
                // Notify the outer function that the task is ready.
                let _ = router.send(());
                // Asynchronously wait for a ledger request.
                while let Some(request) = ledger_handler.recv().await {
                    // Hold the ledger write lock briefly, to update the state of the ledger.
                    // Note: Do not wrap this call in a `task::spawn` as `BlockResponse` messages
                    // will end up being processed out of order.
                    ledger.update(request).await;
                }
            }));
            // Wait until the ledger handler is ready.
            let _ = handler.await;
        }

        Ok(ledger)
    }

    /// Returns an instance of the ledger reader.
    pub fn reader(&self) -> LedgerReader<N> {
        // TODO (howardwu): Switch this from `canon` to `canon_reader`.
        //  RocksDB at v6.22 has a rollback error with its sequence numbers.
        //  Currently, v6.25 has this issue patched, however rust-rocksdb has not released it.
        self.canon.clone()
        // self.canon_reader.clone()
    }

    /// Returns an instance of the ledger router.
    pub fn router(&self) -> LedgerRouter<N> {
        self.ledger_router.clone()
    }

    /// Returns a snapshot of the peers state.
    pub fn peers_state_snapshot(&self) -> Result<HashMap<SocketAddr, Option<(NodeType, State, Option<bool>, u32, BlockLocators<N>)>>> {
        // Execute the future, blocking the current thread until completion.
        Runtime::new()?.block_on(async { Ok(self.peers_state.read().await.clone()) })
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
            LedgerRequest::Disconnect(peer_ip) => {
                info!("Disconnecting from {}", peer_ip);
                // Remove all entries of the peer from the ledger.
                self.remove_peer(&peer_ip).await;
                // Update the status of the ledger.
                self.update_status().await;
                // Route a `PeerDisconnected` to the peers.
                if let Err(error) = self.peers_router.send(PeersRequest::PeerDisconnected(peer_ip)).await {
                    warn!("[Disconnect] {}", error);
                }
            }
            LedgerRequest::Failure(peer_ip, failure) => {
                self.add_failure(peer_ip, failure).await;
            }
            LedgerRequest::Heartbeat(prover_router) => {
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

                debug!(
                    "Status Report (type = {}, status = {}, latest_block_height = {}, block_requests = {}, connected_peers = {})",
                    E::NODE_TYPE,
                    self.status,
                    self.canon.latest_block_height(),
                    self.number_of_block_requests().await,
                    self.peers_state.read().await.len()
                );
            }
            LedgerRequest::Pong(peer_ip, node_type, status, is_fork, block_locators) => {
                // Ensure the peer has been initialized in the ledger.
                self.initialize_peer(peer_ip).await;
                // Process the pong.
                self.update_peer(peer_ip, node_type, status, is_fork, block_locators).await;
            }
            LedgerRequest::UnconfirmedBlock(peer_ip, block, prover_router) => {
                // Ensure the node is not peering.
                if !self.status.is_peering() {
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
    /// Attempt to fast-forward the ledger with unconfirmed blocks.
    ///
    async fn update_ledger(&self, prover_router: &ProverRouter<N>) {
        // Check for candidate blocks to fast forward the ledger.
        let mut block = &self.canon.latest_block();

        let mut unconfirmed_blocks_to_remove = Vec::new();
        let unconfirmed_blocks_snapshot = self.unconfirmed_blocks.read().await.clone();
        while let Some(unconfirmed_block) = unconfirmed_blocks_snapshot.get(&block.hash()) {
            // Update the block iterator.
            block = unconfirmed_block;

            // Ensure the block height is not part of a block request in a fork.
            let mut is_forked_block = false;
            for requests in self.block_requests.read().await.values() {
                for block_request in requests.keys() {
                    // If the block is part of a fork, then don't attempt to add it again.
                    if block_request.height == block.height() && block_request.hash.is_some() {
                        is_forked_block = true;
                        break;
                    }
                }
            }

            // If the block is on a fork, remove the unconfirmed block, and break the loop.
            if is_forked_block {
                unconfirmed_blocks_to_remove.push(block.previous_block_hash());
                break;
            }
            // Attempt to add the unconfirmed block.
            else {
                match self.add_block(block.clone(), prover_router).await {
                    // Upon success, remove the unconfirmed block, as it is now confirmed.
                    true => unconfirmed_blocks_to_remove.push(block.previous_block_hash()),
                    false => break,
                }
            }
        }

        if !unconfirmed_blocks_to_remove.is_empty() {
            let mut unconfirmed_blocks = self.unconfirmed_blocks.write().await;
            for hash in unconfirmed_blocks_to_remove {
                unconfirmed_blocks.remove(&hash);
            }
        }

        // If the timestamp of the last block increment has surpassed the preset limit,
        // the ledger is likely syncing from invalid state, and should revert by one block.
        if self.status.is_syncing()
            && self.last_block_update_timestamp.read().await.elapsed() > 2 * Duration::from_secs(E::RADIO_SILENCE_IN_SECS)
        {
            // Acquire the lock for block requests.
            let _block_request_lock = self.block_requests_lock.lock().await;

            trace!("Ledger state has become stale, clearing queue and reverting by one block");
            *self.unconfirmed_blocks.write().await = Default::default();

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
        let mut status = self.status.get();

        // If the node is shutting down, skip the update.
        if status == State::ShuttingDown {
            trace!("Ledger is shutting down");
            // Set the terminator bit to `true` to ensure it stops mining.
            self.terminator.store(true, Ordering::SeqCst);
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

            // Retrieve the latest block height of this node.
            let latest_block_height = self.canon.latest_block_height();
            // Iterate through the connected peers, to determine if the ledger state is out of date.
            for (_, peer_state) in self.peers_state.read().await.iter() {
                if let Some((_, _, _, block_height, _)) = peer_state {
                    if *block_height > latest_block_height {
                        // Sync if this ledger has fallen behind by 3 or more blocks.
                        if block_height - latest_block_height > 2 {
                            // Set the status to `Syncing`.
                            status = State::Syncing;
                            break;
                        }
                    }
                }
            }
        }

        // If the node is `Peering` or `Syncing`, it should not be mining (yet).
        if status == State::Peering || status == State::Syncing {
            // Set the terminator bit to `true` to ensure it does not mine.
            self.terminator.store(true, Ordering::SeqCst);
        } else {
            // Set the terminator bit to `false` to ensure it is allowed to mine.
            self.terminator.store(false, Ordering::SeqCst);
        }

        // Update the ledger to the determined status.
        self.status.update(status);
    }

    ///
    /// Adds the given block:
    ///     1) as the next block in the ledger if the block height increments by one, or
    ///     2) to the unconfirmed queue for later use.
    ///
    /// Returns `true` if the given block is successfully added to the *canon* chain.
    ///
    async fn add_block(&self, block: Block<N>, prover_router: &ProverRouter<N>) -> bool {
        // Ensure the given block is new.
        if let Ok(true) = self.canon.contains_block_hash(&block.hash()) {
            trace!("Canon chain already contains block {}", block.height());
        } else if block.height() == self.canon.latest_block_height() + 1 && block.previous_block_hash() == self.canon.latest_block_hash() {
            // Acquire the lock for block requests.
            let _block_requests_lock = self.block_requests_lock.lock().await;

            match self.canon.add_next_block(&block) {
                Ok(()) => {
                    info!("Ledger successfully advanced to block {}", self.canon.latest_block_height());

                    // Update the timestamp of the last block increment.
                    *self.last_block_update_timestamp.write().await = Instant::now();
                    // Set the terminator bit to `true` to ensure the miner updates state.
                    self.terminator.store(true, Ordering::SeqCst);
                    // On success, filter the unconfirmed blocks of this block, if it exists.
                    self.unconfirmed_blocks.write().await.remove(&block.previous_block_hash());

                    // On success, filter the memory pool of its transactions, if they exist.
                    if let Err(error) = prover_router.send(ProverRequest::MemoryPoolClear(Some(block))).await {
                        error!("[MemoryPoolClear]: {}", error);
                    }

                    return true;
                }
                Err(error) => warn!("{}", error),
            }
        } else {
            // Retrieve the unconfirmed block height.
            let block_height = block.height();

            // Add the block to the unconfirmed blocks.
            if self.unconfirmed_blocks.write().await.insert(block.previous_block_hash(), block) {
                trace!("Added block {} to unconfirmed queue", block_height);
            } else {
                trace!("Unconfirmed queue already contains block {}", block_height);
            }
        }
        false
    }

    ///
    /// Reverts the ledger state back to height `block_height`, returning `true` on success.
    ///
    async fn revert_to_block_height(&self, block_height: u32) -> bool {
        match self.canon.revert_to_block_height(block_height) {
            Ok(removed_blocks) => {
                info!("Ledger successfully reverted to block {}", self.canon.latest_block_height());

                // Update the last block update timestamp.
                *self.last_block_update_timestamp.write().await = Instant::now();
                // Set the terminator bit to `true` to ensure the miner resets state.
                self.terminator.store(true, Ordering::SeqCst);

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
                self.terminator.store(true, Ordering::SeqCst);
                // Reset the unconfirmed blocks.
                *self.unconfirmed_blocks.write().await = Default::default();

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
                None => match common_ancestor == latest_block_height_of_peer {
                    // If the common ancestor matches the latest block height of the peer,
                    // the peer is clearly on the same canonical chain as this node.
                    true => Some(false),
                    false => None,
                },
            };

            // If the given fork status is None, check if it can be updated.
            let is_fork = match is_fork {
                Some(is_fork) => Some(is_fork),
                None => match common_ancestor == self.canon.latest_block_height() {
                    // If the common ancestor matches the latest block height of this node,
                    // the peer is likely on the same canonical chain as this node.
                    true => Some(false),
                    false => None,
                },
            };

            let fork_status = match is_fork {
                Some(boolean) => format!("{}", boolean),
                None => "undecided".to_string(),
            };
            debug!(
                "Peer {} is at block {} (type = {}, status = {}, is_fork = {}, common_ancestor = {})",
                peer_ip, latest_block_height_of_peer, node_type, status, fork_status, common_ancestor,
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
    /// Case 1 - You are ahead of your peer:
    ///     - Do nothing
    /// Case 2 - You are behind your peer:
    ///     Case 2(a) - `is_fork` is `None`:
    ///         - Peer is being malicious or thinks you are ahead. Both are issues,
    ///           pick a different peer to sync with.
    ///     Case 2(b) - `is_fork` is `Some(false)`:
    ///         - Request blocks from your latest state
    ///     Case 2(c) - `is_fork` is `Some(true)`:
    ///             Case 2(c)(a) - Common ancestor is within `MAXIMUM_FORK_DEPTH`:
    ///                  - Revert to common ancestor, and send block requests to sync.
    ///             Case 2(c)(b) - Common ancestor is NOT within `MAXIMUM_FORK_DEPTH`:
    ///                  Case 2(c)(b)(a) - You can calculate that you are outside of the `MAXIMUM_FORK_DEPTH`:
    ///                      - Disconnect from peer.
    ///                  Case 2(c)(b)(b) - You don't know if you are within the `MAXIMUM_FORK_DEPTH`:
    ///                      - Revert to most common ancestor and send block requests to sync.
    ///
    async fn update_block_requests(&self) {
        // Ensure the ledger is not awaiting responses from outstanding block requests.
        if self.number_of_block_requests().await > 0 {
            return;
        }

        // Retrieve the latest block height of this ledger.
        let latest_block_height = self.canon.latest_block_height();

        // Iterate through the peers to check if this node needs to catch up, and determine a peer to sync with.
        // Prioritize the sync nodes before regular peers.
        let mut maximal_peer = None;
        let mut maximal_peer_is_fork = None;
        let mut maximum_block_height = latest_block_height;
        let mut maximum_block_locators = Default::default();

        // Determine if the peers state has any sync nodes.
        let sync_nodes: HashSet<SocketAddr> = E::SYNC_NODES.iter().map(|ip| ip.parse().unwrap()).collect();

        // Lock peers_state for further processing.
        let peers_state = self.peers_state.read().await;

        let peers_contains_sync_node = false;
        // for ip in peers_state.keys() {
        //     peers_contains_sync_node |= sync_nodes.contains(ip);
        // }

        // Check if any of the peers are ahead and have a larger block height.
        for (peer_ip, peer_state) in peers_state.iter() {
            // Only update the maximal peer if there are no sync nodes or the peer is a sync node.
            if !peers_contains_sync_node || sync_nodes.contains(peer_ip) {
                if let Some((_, _, is_fork, block_height, block_locators)) = peer_state {
                    // Update the maximal peer state if the peer is ahead and the peer knows if you are a fork or not.
                    // This accounts for (Case 1 and Case 2(a))
                    if *block_height > maximum_block_height && is_fork.is_some() {
                        maximal_peer = Some(*peer_ip);
                        maximal_peer_is_fork = *is_fork;
                        maximum_block_height = *block_height;
                        maximum_block_locators = block_locators.clone();
                    }
                }
            }
        }

        // Release the lock over peers_state.
        drop(peers_state);

        // Case 1 - Ensure the peer has a higher block height than this ledger.
        if latest_block_height >= maximum_block_height {
            return;
        }

        // Acquire the lock for block requests.
        let _block_requests_lock = self.block_requests_lock.lock().await;

        // Case 2 - Proceed to send block requests, as the peer is ahead of this ledger.
        if let (Some(peer_ip), Some(is_fork)) = (maximal_peer, maximal_peer_is_fork) {
            // Determine the common ancestor block height between this ledger and the peer.
            let mut maximum_common_ancestor = 0;
            // Determine the first locator (smallest height) that does not exist in this ledger.
            let mut first_deviating_locator = None;

            // Verify the integrity of the block hashes sent by the peer.
            for (block_height, (block_hash, _)) in maximum_block_locators.iter() {
                // Ensure the block hash corresponds with the block height, if the block hash exists in this ledger.
                if let Ok(expected_block_height) = self.canon.get_block_height(block_hash) {
                    if expected_block_height != *block_height {
                        let error = format!("Invalid block height {} for block hash {}", expected_block_height, block_hash);
                        trace!("{}", error);
                        self.add_failure(peer_ip, error).await;
                        return;
                    } else {
                        // Update the common ancestor, as this block hash exists in this ledger.
                        if expected_block_height > maximum_common_ancestor {
                            maximum_common_ancestor = expected_block_height;
                        }
                    }
                } else {
                    // Update the first deviating locator.
                    match first_deviating_locator {
                        None => first_deviating_locator = Some(block_height),
                        Some(saved_height) => {
                            if block_height < saved_height {
                                first_deviating_locator = Some(block_height);
                            }
                        }
                    }
                }
            }

            // Ensure the latest common ancestor is not greater than the latest block request.
            if latest_block_height < maximum_common_ancestor {
                warn!(
                    "The common ancestor {} cannot be greater than the latest block {}",
                    maximum_common_ancestor, latest_block_height
                );
                return;
            }

            // Determine the latest common ancestor.
            let (latest_common_ancestor, ledger_reverted) =
                // Case 2(b) - This ledger is not a fork of the peer, it is on the same canon chain.
                if !is_fork {
                    // Continue to sync from the latest block height of this ledger, if the peer is honest.
                    match first_deviating_locator.is_none() {
                        true => (maximum_common_ancestor, false),
                        false => (latest_block_height, false),
                    }
                }
                // Case 2(c) - This ledger is on a fork of the peer.
                else {
                    // Case 2(c)(a) - If the common ancestor is within the fork range of this ledger,
                    // proceed to switch to the fork.
                    if latest_block_height.saturating_sub(maximum_common_ancestor) <= E::MAXIMUM_FORK_DEPTH
                    {
                        info!("Found a longer chain from {} starting at block {}", peer_ip, maximum_common_ancestor);
                        // If the latest block is the same as the maximum common ancestor, do not revert.
                        if latest_block_height != maximum_common_ancestor && !self.revert_to_block_height(maximum_common_ancestor).await {
                            return;
                        }
                        (maximum_common_ancestor, true)
                    }
                    // Case 2(c)(b) - If the common ancestor is NOT within `MAXIMUM_FORK_DEPTH`.
                    else
                    {
                        // Ensure that the first deviating locator exists.
                        let first_deviating_locator = match first_deviating_locator {
                            Some(locator) => locator,
                            None => return,
                        };

                        // Case 2(c)(b)(a) - Check if the real common ancestor is NOT within `MAXIMUM_FORK_DEPTH`.
                        // If this peer is outside of the fork range of this ledger, proceed to disconnect from the peer.
                        if latest_block_height.saturating_sub(*first_deviating_locator) >= E::MAXIMUM_FORK_DEPTH {
                            debug!("Peer {} is outside of the fork range of this ledger, disconnecting", peer_ip);
                            // Send a `Disconnect` message to the peer.
                            let request = PeersRequest::MessageSend(peer_ip, Message::Disconnect);
                            if let Err(error) = self.peers_router.send(request).await {
                                warn!("[Disconnect] {}", error);
                            }
                            return;
                        }
                        // Case 2(c)(b)(b) - You don't know if your real common ancestor is within `MAXIMUM_FORK_DEPTH`.
                        // Revert to the common ancestor anyways.
                        else {
                            info!("Found a potentially longer chain from {} starting at block {}", peer_ip, maximum_common_ancestor);
                            match self.revert_to_block_height(maximum_common_ancestor).await {
                                true => (maximum_common_ancestor, true),
                                false => return
                            }
                        }
                    }
                };

            // TODO (howardwu): Ensure the start <= end.
            // Determine the start and end block heights to request.
            let number_of_block_requests = std::cmp::min(maximum_block_height - latest_common_ancestor, E::MAXIMUM_BLOCK_REQUEST);
            let start_block_height = latest_common_ancestor + 1;
            let end_block_height = start_block_height + number_of_block_requests - 1;
            debug!("Requesting blocks {} to {} from {}", start_block_height, end_block_height, peer_ip);

            // Send a `BlockRequest` message to the peer.
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
                        // If the ledger was reverted, include the expected new block hash for the fork.
                        match ledger_reverted {
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

            // TODO (howardwu): TEMPORARY - Evaluate the merits of this experiment after seeing the results.
            // If the node is a sync node and the node is currently syncing,
            // reduce the number of connections down to the minimum threshold,
            // to improve the speed with which the node syncs back to tip.
            if E::NODE_TYPE == NodeType::Sync && self.status.is_syncing() && self.number_of_block_requests().await > 0 {
                debug!("Temporarily reducing the number of connected peers to sync");

                // Lock peers_state and block_requests for further processing.
                let peers_state = self.peers_state.read().await;
                let block_requests = self.block_requests.read().await;

                // Determine the peers to disconnect from.
                // Attention - We are reducing this to the `MINIMUM_NUMBER_OF_PEERS`, *not* `MAXIMUM_NUMBER_OF_PEERS`.
                let num_excess_peers = peers_state.len().saturating_sub(E::MINIMUM_NUMBER_OF_PEERS);
                let peer_ips_to_disconnect = peers_state
                    .iter()
                    .filter(|(&peer_ip, _)| {
                        let peer_str = peer_ip.to_string();
                        !E::SYNC_NODES.contains(&peer_str.as_str())
                            && !E::BEACON_NODES.contains(&peer_str.as_str())
                            && !block_requests.contains_key(&peer_ip)
                    })
                    .take(num_excess_peers)
                    .map(|(&ip, _)| ip)
                    .collect::<Vec<SocketAddr>>();

                // Release the lock over peers_state and block_requests.
                drop(peers_state);
                drop(block_requests);

                trace!("Found {} peers to temporarily disconnect", peer_ips_to_disconnect.len());

                // Proceed to send disconnect requests to these peers.
                for peer_ip in peer_ips_to_disconnect {
                    info!("Disconnecting from {} (disconnecting to sync)", peer_ip);
                    // Remove all entries of the peer from the ledger.
                    self.remove_peer(&peer_ip).await;
                    // Update the status of the ledger.
                    self.update_status().await;
                    // Route a `PeerRestricted` to the peers.
                    if let Err(error) = self.peers_router.send(PeersRequest::PeerRestricted(peer_ip)).await {
                        warn!("[PeerRestricted] {}", error);
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
        match locked_block_requests.insert((block_height, block_hash).into(), Utc::now().timestamp()) {
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
        let now = Utc::now().timestamp();
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
            Some(failures) => failures.push((failure, Utc::now().timestamp())),
            None => error!("Missing failure entry for {}", peer_ip),
        };
    }

    ///
    /// Removes failures that have expired.
    ///
    async fn remove_expired_failures(&self) {
        // Clear all failures that have lived longer than `E::FAILURE_EXPIRY_TIME_IN_SECS`.
        let now = Utc::now().timestamp();
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
            if let Err(error) = self.ledger_router.send(LedgerRequest::Disconnect(peer_ip)).await {
                warn!("Failed to send disconnect message to failing peer {}: {}", peer_ip, error);
            }
        }
    }
}
