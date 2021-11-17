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

use crate::{helpers::CircularMap, Environment, Message, NodeType, PeersRequest, PeersRouter};
use snarkos_ledger::{storage::Storage, BlockLocators, LedgerState};
use snarkvm::dpc::prelude::*;

use anyhow::Result;
use chrono::Utc;
use parking_lot::{Mutex, RwLock};
use rand::thread_rng;
use std::{
    collections::HashMap,
    marker::PhantomData,
    net::SocketAddr,
    path::Path,
    sync::{
        atomic::{AtomicBool, AtomicU8, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
use tokio::{sync::mpsc, task};

/// The maximum number of unconfirmed blocks that can be held by the ledger.
const MAXIMUM_UNCONFIRMED_BLOCKS: u32 = 50;

/// Shorthand for the parent half of the `Ledger` message channel.
pub(crate) type LedgerRouter<N, E> = mpsc::Sender<LedgerRequest<N, E>>;
#[allow(unused)]
/// Shorthand for the child half of the `Ledger` message channel.
type LedgerHandler<N, E> = mpsc::Receiver<LedgerRequest<N, E>>;

///
/// An enum of requests that the `Ledger` struct processes.
///
#[derive(Debug)]
pub enum LedgerRequest<N: Network, E: Environment> {
    /// BlockRequest := (peer_ip, start_block_height, end_block_height (inclusive))
    BlockRequest(SocketAddr, u32, u32),
    /// BlockResponse := (peer_ip, block)
    BlockResponse(SocketAddr, Block<N>),
    /// Disconnect := (peer_ip)
    Disconnect(SocketAddr),
    /// Heartbeat := ()
    Heartbeat(LedgerRouter<N, E>),
    /// Mine := (local_ip, miner_address, ledger_router)
    Mine(SocketAddr, Address<N>, LedgerRouter<N, E>),
    /// Ping := (peer_ip, block_height, block_hash)
    Ping(SocketAddr, u32, N::BlockHash),
    /// Pong := (peer_ip, is_fork, block_locators)
    Pong(SocketAddr, Option<bool>, BlockLocators<N>),
    /// SendPing := (peer_ip)
    SendPing(SocketAddr),
    /// UnconfirmedBlock := (peer_ip, block)
    UnconfirmedBlock(SocketAddr, Block<N>),
    /// UnconfirmedTransaction := (peer_ip, transaction)
    UnconfirmedTransaction(SocketAddr, Transaction<N>),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Status {
    /// The ledger is ready to handle requests.
    Ready = 0,
    /// The ledger is mining the next block.
    Mining,
    /// The ledger is connecting to the minimum number of required peers.
    Peering,
    /// The ledger is syncing blocks with a connected peer.
    Syncing,
    /// The ledger is terminating and shutting down.
    ShuttingDown,
}

///
/// A ledger for a specific network on the node server.
///
#[derive(Clone, Debug)]
#[allow(clippy::type_complexity)]
pub struct Ledger<N: Network, E: Environment> {
    /// The canonical chain of block hashes.
    canon: LedgerState<N>,
    /// A map of previous block hashes to unconfirmed blocks.
    unconfirmed_blocks: CircularMap<N::BlockHash, Block<N>, { MAXIMUM_UNCONFIRMED_BLOCKS }>,
    /// The pool of unconfirmed transactions.
    memory_pool: MemoryPool<N>,

    /// The status of the ledger.
    status: Arc<AtomicU8>,
    /// A terminator bit for the miner.
    terminator: Arc<AtomicBool>,
    /// The map of each peer to their ledger state := (is_fork, latest_block_height, block_locators).
    peers_state: HashMap<SocketAddr, Option<(Option<bool>, u32, BlockLocators<N>)>>,
    /// The map of each peer to their block requests := HashMap<(block_height, block_hash), timestamp>
    block_requests: HashMap<SocketAddr, HashMap<(u32, Option<N::BlockHash>), i64>>,
    /// A lock to ensure methods that need to be mutually-exclusive are enforced.
    /// In this context, `add_block` and `update_block_requests` must be mutually-exclusive.
    block_requests_lock: Arc<Mutex<bool>>,
    /// The timestamp of the last successful block update.
    last_block_update_timestamp: Instant,
    /// The map of each peer to their failure messages := (failure_message, timestamp).
    failures: HashMap<SocketAddr, Vec<(String, i64)>>,
    _phantom: PhantomData<E>,
}

impl<N: Network, E: Environment> Ledger<N, E> {
    /// Initializes a new instance of the ledger.
    pub fn open<S: Storage, P: AsRef<Path>>(path: P) -> Result<Self> {
        let canon = LedgerState::open::<S, P>(path, false, None)?;
        let last_block_update_timestamp = Instant::now();
        Ok(Self {
            canon,
            unconfirmed_blocks: Default::default(),
            memory_pool: MemoryPool::new(),

            status: Arc::new(AtomicU8::new(Status::Peering as u8)),
            terminator: Arc::new(AtomicBool::new(false)),
            peers_state: Default::default(),
            block_requests: Default::default(),
            block_requests_lock: Arc::new(Mutex::new(true)),
            last_block_update_timestamp,
            failures: Default::default(),
            _phantom: PhantomData,
        })
    }

    /// Returns the status of the ledger.
    pub fn status(&self) -> Status {
        match self.status.load(Ordering::SeqCst) {
            0 => Status::Ready,
            1 => Status::Mining,
            2 => Status::Peering,
            3 => Status::Syncing,
            4 => Status::ShuttingDown,
            _ => unreachable!("Invalid status code"),
        }
    }

    /// Returns `true` if the ledger is currently mining.
    pub fn is_mining(&self) -> bool {
        self.status() == Status::Mining
    }

    /// Returns `true` if the ledger is currently peering.
    pub fn is_peering(&self) -> bool {
        self.status() == Status::Peering
    }

    /// Returns `true` if the ledger is currently syncing.
    pub fn is_syncing(&self) -> bool {
        self.status() == Status::Syncing
    }

    /// Returns the latest block object.
    pub fn latest_block(&self) -> Arc<RwLock<Block<N>>> {
        self.canon.latest_block_object()
    }

    /// Returns the latest block height.
    pub fn latest_block_height(&self) -> u32 {
        self.canon.latest_block_height()
    }

    /// Returns the latest block hash.
    pub fn latest_block_hash(&self) -> N::BlockHash {
        self.canon.latest_block_hash()
    }

    ///
    /// Performs the given `request` to the ledger.
    /// All requests must go through this `update`, so that a unified view is preserved.
    ///
    pub(super) async fn update(&mut self, request: LedgerRequest<N, E>, peers_router: &PeersRouter<N, E>) {
        match request {
            LedgerRequest::BlockRequest(peer_ip, start_block_height, end_block_height) => {
                // Ensure the request is within the accepted limits.
                let number_of_blocks = end_block_height.saturating_sub(start_block_height);
                if number_of_blocks > E::MAXIMUM_BLOCK_REQUEST {
                    let failure = format!("Attempted to request {} blocks", number_of_blocks);
                    warn!("{}", failure);
                    self.add_failure(peer_ip, failure);
                    return;
                }
                // Retrieve the requested blocks.
                let blocks = match self.canon.get_blocks(start_block_height, end_block_height) {
                    Ok(blocks) => blocks,
                    Err(error) => {
                        error!("{}", error);
                        self.add_failure(peer_ip, format!("{}", error));
                        return;
                    }
                };
                // Send a `BlockResponse` message for each block to the peer.
                for block in blocks {
                    let request = PeersRequest::MessageSend(peer_ip, Message::BlockResponse(block));
                    if let Err(error) = peers_router.send(request).await {
                        warn!("[BlockResponse] {}", error);
                    }
                }
            }
            LedgerRequest::BlockResponse(peer_ip, block) => {
                // Remove the block request from the ledger.
                if self.remove_block_request(peer_ip, block.height(), block.hash()) {
                    // On success, process the block response.
                    self.add_block(block);
                    // Check if syncing with this peer is complete.
                    if let Some(requests) = self.block_requests.get(&peer_ip) {
                        if requests.is_empty() {
                            trace!("All block requests with {} have been processed", peer_ip);
                            self.update_block_requests(peers_router).await;
                        }
                    }
                }
            }
            LedgerRequest::Disconnect(peer_ip) => {
                info!("Disconnecting from {}", peer_ip);
                // Remove all entries of the peer from the ledger.
                self.remove_peer(&peer_ip);
                // Update the status of the ledger.
                self.update_status();
                // Route a `PeerDisconnected` to the peers.
                if let Err(error) = peers_router.send(PeersRequest::PeerDisconnected(peer_ip)).await {
                    warn!("[Disconnect] {}", error);
                }
            }
            LedgerRequest::Heartbeat(ledger_router) => {
                // Update the ledger.
                self.update_ledger();
                // Update the status of the ledger.
                self.update_status();
                // Remove expired block requests.
                self.remove_expired_block_requests();
                // Remove expired failures.
                self.remove_expired_failures();
                // Disconnect from peers with frequent failures.
                self.disconnect_from_failing_peers(&ledger_router).await;
                // Update the block requests.
                self.update_block_requests(peers_router).await;
            }
            LedgerRequest::Mine(local_ip, recipient, ledger_router) => {
                // Process the request to mine the next block.
                self.mine_next_block(local_ip, recipient, ledger_router);
            }
            LedgerRequest::Ping(peer_ip, block_height, block_hash) => {
                // Determine if the peer is on a fork (or unknown).
                let is_fork: Option<bool> = match self.canon.get_block_hash(block_height) {
                    Ok(expected_block_hash) => Some(expected_block_hash != block_hash),
                    Err(_) => None,
                };
                // Send a `Pong` message to the peer.
                let request = PeersRequest::MessageSend(peer_ip, Message::Pong(is_fork, self.canon.latest_block_locators()));
                if let Err(error) = peers_router.send(request).await {
                    warn!("[Pong] {}", error);
                }
            }
            LedgerRequest::Pong(peer_ip, is_fork, block_locators) => {
                // Ensure the peer has been initialized in the ledger.
                self.initialize_peer(peer_ip);
                // Process the pong.
                self.update_peer(peer_ip, is_fork, block_locators).await;

                // Sleep for the preset time before sending a `Ping` request.
                tokio::time::sleep(Duration::from_secs(E::PING_SLEEP_IN_SECS)).await;
                // Send a `Ping` request to the peer.
                let message = Message::Ping(E::MESSAGE_VERSION, self.latest_block_height(), self.latest_block_hash());
                let request = PeersRequest::MessageSend(peer_ip, message);
                if let Err(error) = peers_router.send(request).await {
                    warn!("[Ping] {}", error);
                }
            }
            LedgerRequest::SendPing(peer_ip) => {
                // Send a `Ping` request to the peer.
                let message = Message::Ping(E::MESSAGE_VERSION, self.latest_block_height(), self.latest_block_hash());
                let request = PeersRequest::MessageSend(peer_ip, message);
                if let Err(error) = peers_router.send(request).await {
                    warn!("[Ping] {}", error);
                }
            }
            LedgerRequest::UnconfirmedBlock(peer_ip, block) => {
                // Ensure the given block is new.
                if let Ok(true) = self.canon.contains_block_hash(&block.hash()) {
                    trace!("Canon chain already contains block {}", block.height());
                } else if self.unconfirmed_blocks.contains_key(&block.previous_block_hash()) {
                    trace!("Memory pool already contains unconfirmed block {}", block.height());
                } else {
                    // Ensure the unconfirmed block is at least within 10 blocks of the latest block height.
                    if block.height() + 10 > self.latest_block_height() {
                        // Process the unconfirmed block.
                        self.add_block(block.clone());
                        // Propagate the unconfirmed block to the connected peers.
                        let request = PeersRequest::MessagePropagate(peer_ip, Message::UnconfirmedBlock(block));
                        if let Err(error) = peers_router.send(request).await {
                            warn!("[UnconfirmedBlock] {}", error);
                        }
                    }
                }
            }
            LedgerRequest::UnconfirmedTransaction(peer_ip, transaction) => {
                // Process the unconfirmed transaction.
                self.add_unconfirmed_transaction(peer_ip, transaction, peers_router).await
            }
        }
    }

    ///
    /// Attempt to fast-forward the ledger with unconfirmed blocks.
    ///
    fn update_ledger(&mut self) {
        // Check for candidate blocks to fast forward the ledger.
        let mut block = &self.canon.latest_block();
        let unconfirmed_blocks = self.unconfirmed_blocks.clone();
        while let Some(unconfirmed_block) = unconfirmed_blocks.get(&block.hash()) {
            // Update the block iterator.
            block = unconfirmed_block;

            // Ensure the block height is not part of a block request in a fork.
            let mut is_forked_block = false;
            for requests in self.block_requests.values() {
                for (block_height, block_hash) in requests.keys() {
                    // If the block is part of a fork, then don't attempt to add it again.
                    if block_height == &block.height() && block_hash.is_some() {
                        is_forked_block = true;
                        break;
                    }
                }
            }

            // If the block is on a fork, remove the unconfirmed block, and break the loop.
            if is_forked_block {
                self.unconfirmed_blocks.remove(&block.hash());
                break;
            }
            // Attempt to add the unconfirmed block.
            else {
                match self.add_block(block.clone()) {
                    // Upon success, remove the unconfirmed block, as it is now confirmed.
                    true => self.unconfirmed_blocks.remove(&block.hash()),
                    false => break,
                }
            }
        }

        // If the timestamp of the last block increment has surpassed the preset limit,
        // the ledger is likely syncing from invalid state, and should revert by one block.
        if self.is_syncing() && self.last_block_update_timestamp.elapsed() > Duration::from_secs(E::RADIO_SILENCE_IN_SECS) {
            trace!("Ledger state has become stale, clearing queue and reverting by one block");
            self.unconfirmed_blocks = Default::default();
            self.memory_pool = MemoryPool::new();
            self.block_requests.values_mut().for_each(|requests| *requests = Default::default());
            self.revert_to_block_height(self.latest_block_height().saturating_sub(1));
        }
    }

    ///
    /// Updates the status of the ledger.
    ///
    fn update_status(&mut self) {
        // Retrieve the status variable.
        let mut status = self.status();

        // If the node is shutting down, skip the update.
        if status == Status::ShuttingDown {
            trace!("Ledger is shutting down");
            // Set the terminator bit to `true` to ensure it stops mining.
            self.terminator.store(true, Ordering::SeqCst);
            return;
        }
        // If there is an insufficient number of connected peers, set the status to `Peering`.
        else if self.peers_state.len() < E::MINIMUM_NUMBER_OF_PEERS {
            status = Status::Peering;
        }
        // If the ledger is out of date, set the status to `Syncing`.
        else {
            // Update the status to `Ready` or `Mining`.
            status = match status {
                Status::Mining => Status::Mining,
                _ => Status::Ready,
            };

            // Retrieve the latest block height of this node.
            let latest_block_height = self.latest_block_height();
            // Iterate through the connected peers, to determine if the ledger state is out of date.
            for (_, ledger_state) in self.peers_state.iter() {
                if let Some((_, block_height, _)) = ledger_state {
                    if *block_height > latest_block_height {
                        // Sync if this ledger has fallen behind by 3 or more blocks.
                        if block_height - latest_block_height > 2 {
                            // Set the status to `Syncing`.
                            status = Status::Syncing;
                            break;
                        }
                    }
                }
            }
        }

        // If the node is `Peering` or `Syncing`, it should not be mining (yet).
        if status == Status::Peering || status == Status::Syncing {
            // Set the terminator bit to `true` to ensure it does not mine.
            self.terminator.store(true, Ordering::SeqCst);
        } else {
            // Set the terminator bit to `false` to ensure it is allowed to mine.
            self.terminator.store(false, Ordering::SeqCst);
        }

        // Update the ledger to the determined status.
        self.status.store(status as u8, Ordering::SeqCst);
    }

    ///
    /// Mines a new block and adds it to the canon blocks.
    ///
    fn mine_next_block(&self, local_ip: SocketAddr, recipient: Address<N>, ledger_router: LedgerRouter<N, E>) {
        // If the node type is not a miner, it should not be mining.
        if E::NODE_TYPE != NodeType::Miner {
            return;
        }
        // If there is an insufficient number of connected peers, it should not be mining.
        if self.peers_state.len() < E::MINIMUM_NUMBER_OF_PEERS {
            return;
        }
        // If `terminator` is `true`, it should not be mining.
        if self.terminator.load(Ordering::SeqCst) {
            return;
        }
        // If the status is `Ready`, mine the next block.
        if self.status() == Status::Ready {
            // Set the status to `Mining`.
            self.status.store(Status::Mining as u8, Ordering::SeqCst);

            // Prepare the unconfirmed transactions, terminator, and status.
            let canon = self.canon.clone(); // This is safe as we only *read* LedgerState.
            let unconfirmed_transactions = self.memory_pool.transactions();
            let terminator = self.terminator.clone();
            let status = self.status.clone();

            task::spawn(async move {
                // Mine the next block.
                let result = canon.mine_next_block(recipient, &unconfirmed_transactions, &terminator, &mut thread_rng());

                // Set the status to `Ready`.
                status.store(Status::Ready as u8, Ordering::SeqCst);

                match result {
                    Ok(block) => {
                        trace!("Miner has found the next block");
                        // Broadcast the next block.
                        let request = LedgerRequest::UnconfirmedBlock(local_ip, block);
                        if let Err(error) = ledger_router.send(request).await {
                            warn!("Failed to broadcast mined block: {}", error);
                        }
                    }
                    Err(error) => trace!("{}", error),
                }
            });
        }
    }

    ///
    /// Adds the given block:
    ///     1) as the next block in the ledger if the block height increments by one, or
    ///     2) to the memory pool for later use.
    ///
    /// Returns `true` if the given block is successfully added to the *canon* chain.
    ///
    fn add_block(&mut self, block: Block<N>) -> bool {
        // Acquire the lock for block requests.
        let _ = self.block_requests_lock.lock();

        // Ensure the given block is new.
        if let Ok(true) = self.canon.contains_block_hash(&block.hash()) {
            trace!("Canon chain already contains block {}", block.height());
        } else if block.height() == self.latest_block_height() + 1 && block.previous_block_hash() == self.latest_block_hash() {
            match self.canon.add_next_block(&block) {
                Ok(()) => {
                    info!("Ledger advanced to block {}", self.latest_block_height());

                    // Update the timestamp of the last block increment.
                    self.last_block_update_timestamp = Instant::now();
                    // Set the terminator bit to `true` to ensure the miner updates state.
                    self.terminator.store(true, Ordering::SeqCst);
                    // On success, filter the memory pool of its transactions, if they exist.
                    self.memory_pool.remove_transactions(block.transactions());
                    // On success, filter the unconfirmed blocks of this block, if it exists.
                    if self.unconfirmed_blocks.contains_key(&block.hash()) {
                        self.unconfirmed_blocks.remove(&block.hash());
                    }

                    return true;
                }
                Err(error) => warn!("{}", error),
            }
        } else {
            // Ensure the unconfirmed block is well-formed.
            match block.is_valid() {
                true => {
                    // Ensure the unconfirmed block does not already exist in the memory pool.
                    match !self.unconfirmed_blocks.contains_key(&block.previous_block_hash()) {
                        true => {
                            trace!("Adding unconfirmed block {} to memory pool", block.height());

                            // Add the block to the unconfirmed blocks.
                            self.unconfirmed_blocks.insert(block.previous_block_hash(), block);
                        }
                        false => trace!("Unconfirmed block {} already exists in the memory pool", block.height()),
                    }
                }
                false => warn!("Unconfirmed block {} is invalid", block.height()),
            }
        }
        false
    }

    ///
    /// Adds the given unconfirmed transaction to the memory pool.
    ///
    async fn add_unconfirmed_transaction(&mut self, peer_ip: SocketAddr, transaction: Transaction<N>, peers_router: &PeersRouter<N, E>) {
        // Process the unconfirmed transaction.
        trace!("Received unconfirmed transaction {} from {}", transaction.transaction_id(), peer_ip);
        // Ensure the unconfirmed transaction is new.
        if let Ok(false) = self.canon.contains_transaction(&transaction.transaction_id()) {
            debug!("Adding unconfirmed transaction {} to memory pool", transaction.transaction_id());
            // Attempt to add the unconfirmed transaction to the memory pool.
            match self.memory_pool.add_transaction(&transaction) {
                Ok(()) => {
                    // Upon success, propagate the unconfirmed transaction to the connected peers.
                    let request = PeersRequest::MessagePropagate(peer_ip, Message::UnconfirmedTransaction(transaction));
                    if let Err(error) = peers_router.send(request).await {
                        warn!("[UnconfirmedTransaction] {}", error);
                    }
                }
                Err(error) => error!("{}", error),
            }
        }
    }

    ///
    /// Reverts the ledger state back to height `block_height`, returning `true` on success.
    ///
    fn revert_to_block_height(&mut self, block_height: u32) -> bool {
        match self.canon.revert_to_block_height(block_height) {
            Ok(removed_blocks) => {
                info!("Ledger successfully reverted to block {}", self.latest_block_height());

                // Update the last block update timestamp.
                self.last_block_update_timestamp = Instant::now();
                // Set the terminator bit to `true` to ensure the miner resets state.
                self.terminator.store(true, Ordering::SeqCst);

                // Ensure the removed blocks are not in the unconfirmed blocks.
                for removed_block in removed_blocks {
                    if self.unconfirmed_blocks.contains_key(&removed_block.hash()) {
                        self.unconfirmed_blocks.remove(&removed_block.hash());
                    }
                }
                true
            }
            Err(error) => {
                error!("Failed to revert the ledger to block {}: {}", block_height, error);

                // Set the terminator bit to `true` to ensure the miner resets state.
                self.terminator.store(true, Ordering::SeqCst);
                // Reset the unconfirmed blocks.
                self.unconfirmed_blocks = Default::default();

                false
            }
        }
    }

    ///
    /// Adds an entry for the given peer IP to every data structure in `State`.
    ///
    fn initialize_peer(&mut self, peer_ip: SocketAddr) {
        self.peers_state.entry(peer_ip).or_insert(None);
        self.block_requests.entry(peer_ip).or_insert_with(Default::default);
        self.failures.entry(peer_ip).or_insert_with(Default::default);
    }

    ///
    /// Removes the entry for the given peer IP from every data structure in `State`.
    ///
    fn remove_peer(&mut self, peer_ip: &SocketAddr) {
        if self.peers_state.contains_key(peer_ip) {
            self.peers_state.remove(peer_ip);
        }
        if self.block_requests.contains_key(peer_ip) {
            self.block_requests.remove(peer_ip);
        }
        if self.failures.contains_key(peer_ip) {
            self.failures.remove(peer_ip);
        }
    }

    ///
    /// Updates the state of the given peer.
    ///
    async fn update_peer(&mut self, peer_ip: SocketAddr, is_fork: Option<bool>, block_locators: BlockLocators<N>) {
        // Ensure the list of block locators is not empty.
        if block_locators.is_empty() {
            self.add_failure(peer_ip, "Received a sync response with no block locators".to_string());
        } else {
            // Ensure the peer provided well-formed block locators.
            match self.canon.check_block_locators(&block_locators) {
                Ok(is_valid) => {
                    if !is_valid {
                        warn!("Invalid block locators from {}", peer_ip);
                        self.add_failure(peer_ip, "Invalid block locators".to_string());
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
                if *block_height > latest_block_height_of_peer {
                    latest_block_height_of_peer = *block_height;
                }
            }

            // trace!("STATUS {:?} {} {}", self.status(), self.latest_block_height(), self.number_of_block_requests());
            debug!(
                "Peer {} is at block {} (common_ancestor = {})",
                peer_ip, latest_block_height_of_peer, common_ancestor,
            );

            match self.peers_state.get_mut(&peer_ip) {
                Some(status) => *status = Some((is_fork, latest_block_height_of_peer, block_locators)),
                None => self.add_failure(peer_ip, format!("Missing ledger state for {}", peer_ip)),
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
    ///             Case 2(c)(a) - Common ancestor is within `ALEO_MAXIMUM_FORK_DEPTH`:
    ///                  - Revert to common ancestor, and send block requests to sync.
    ///             Case 2(c)(b) - Common ancestor is NOT within `ALEO_MAXIMUM_FORK_DEPTH`:
    ///                  Case 2(c)(b)(a) - You can calculate that you are outside of the `ALEO_MAXIMUM_FORK_DEPTH`:
    ///                      - Disconnect from peer.
    ///                  Case 2(c)(b)(b) - You don't know if you are within the `ALEO_MAXIMUM_FORK_DEPTH`:
    ///                      - Revert to most common ancestor and send block requests to sync.
    ///
    async fn update_block_requests(&mut self, peers_router: &PeersRouter<N, E>) {
        // Ensure the ledger is not awaiting responses from outstanding block requests.
        if self.number_of_block_requests() > 0 {
            return;
        }

        // Acquire the lock for block requests.
        let _ = self.block_requests_lock.lock();

        // Iterate through the peers to check if this node needs to catch up, and determine a peer to sync with.
        // Prioritize the sync nodes before regular peers.
        let mut maximal_peer = None;
        let mut maximal_peer_is_fork = None;
        let mut maximum_block_height = self.latest_block_height();
        let mut maximum_block_locators = Default::default();

        // Determine if the peers state has any sync nodes.
        let sync_nodes: Vec<SocketAddr> = E::SYNC_NODES.iter().map(|ip| ip.parse().unwrap()).collect();
        let mut peers_contains_sync_node = false;
        for ip in self.peers_state.keys() {
            peers_contains_sync_node |= sync_nodes.contains(ip);
        }

        // Check if any of the peers are ahead and have a larger block height.
        for (peer_ip, ledger_state) in self.peers_state.iter() {
            // Only update the maximal peer if there are no sync nodes or the peer is a sync node.
            if !peers_contains_sync_node || sync_nodes.contains(peer_ip) {
                if let Some((is_fork, block_height, block_locators)) = ledger_state {
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

        // Case 1 - Ensure the peer has a higher block height than this ledger.
        let latest_block_height = self.latest_block_height();
        if latest_block_height >= maximum_block_height {
            return;
        }

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
                        self.add_failure(peer_ip, error);
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
            let latest_block_height = self.latest_block_height();
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
                    if latest_block_height.saturating_sub(maximum_common_ancestor) <= N::ALEO_MAXIMUM_FORK_DEPTH
                    {
                        info!("Found a longer chain from {} starting at block {}", peer_ip, maximum_common_ancestor);
                        // If the latest block is the same as the maximum common ancestor, do not revert.
                        if latest_block_height != maximum_common_ancestor && !self.revert_to_block_height(maximum_common_ancestor) {
                            return;
                        }
                        (maximum_common_ancestor, true)
                    }
                    // Case 2(c)(b) - If the common ancestor is NOT within `ALEO_MAXIMUM_FORK_DEPTH`.
                    else
                    {
                        // Ensure that the first deviating locator exists.
                        let first_deviating_locator = match first_deviating_locator {
                            Some(locator) => locator,
                            None => return,
                        };

                        // Case 2(c)(b)(a) - Check if the real common ancestor is NOT within `ALEO_MAXIMUM_FORK_DEPTH`.
                        // If this peer is outside of the fork range of this ledger, proceed to disconnect from the peer.
                        if latest_block_height.saturating_sub(*first_deviating_locator) >= N::ALEO_MAXIMUM_FORK_DEPTH {
                            debug!("Peer {} is outside of the fork range of this ledger, disconnecting", peer_ip);
                            // Send a `Disconnect` message to the peer.
                            let request = PeersRequest::MessageSend(peer_ip, Message::Disconnect);
                            if let Err(error) = peers_router.send(request).await {
                                warn!("[Disconnect] {}", error);
                            }
                            return;
                        }
                        // Case 2(c)(b)(b) - You don't know if your real common ancestor is within `ALEO_MAXIMUM_FORK_DEPTH`.
                        // Revert to the common ancestor anyways.
                        else {
                            info!("Found a potentially longer chain from {} starting at block {}", peer_ip, maximum_common_ancestor);
                            match self.revert_to_block_height(maximum_common_ancestor) {
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
            debug!("Request blocks {} to {} from {}", start_block_height, end_block_height, peer_ip);

            // Send a `BlockRequest` message to the peer.
            let request = PeersRequest::MessageSend(peer_ip, Message::BlockRequest(start_block_height, end_block_height));
            if let Err(error) = peers_router.send(request).await {
                warn!("[BlockRequest] {}", error);
                return;
            }

            // Log each block request to ensure the peer responds with all requested blocks.
            for block_height in start_block_height..=end_block_height {
                // If the ledger was reverted, include the expected new block hash for the fork.
                match ledger_reverted {
                    true => self.add_block_request(peer_ip, block_height, maximum_block_locators.get_block_hash(block_height)),
                    false => self.add_block_request(peer_ip, block_height, None),
                };
            }
        }
    }

    ///
    /// Returns the number of outstanding block requests.
    ///
    fn number_of_block_requests(&self) -> usize {
        self.block_requests.values().map(|r| r.len()).sum()
    }

    ///
    /// Adds a block request for the given block height to the specified peer.
    ///
    fn add_block_request(&mut self, peer_ip: SocketAddr, block_height: u32, block_hash: Option<N::BlockHash>) {
        // Ensure the block request does not already exist.
        if !self.contains_block_request(peer_ip, block_height, block_hash) {
            match self.block_requests.get_mut(&peer_ip) {
                Some(requests) => match requests.insert((block_height, block_hash), Utc::now().timestamp()) {
                    None => debug!("Requesting block {} from {}", block_height, peer_ip),
                    Some(_old_request) => self.add_failure(peer_ip, format!("Duplicate block request for {}", peer_ip)),
                },
                None => self.add_failure(peer_ip, format!("Missing block requests for {}", peer_ip)),
            };
        }
    }

    ///
    /// Returns `true` if the block request for the given block height to the specified peer exists.
    ///
    fn contains_block_request(&self, peer_ip: SocketAddr, block_height: u32, block_hash: Option<N::BlockHash>) -> bool {
        match self.block_requests.get(&peer_ip) {
            Some(requests) => requests.contains_key(&(block_height, block_hash)) || requests.contains_key(&(block_height, None)),
            None => false,
        }
    }

    ///
    /// Removes a block request for the given block height to the specified peer.
    /// On success, returns `true`, otherwise returns `false`.
    ///
    fn remove_block_request(&mut self, peer_ip: SocketAddr, block_height: u32, block_hash: N::BlockHash) -> bool {
        // Ensure the block height corresponds to a requested block.
        if !self.contains_block_request(peer_ip, block_height, Some(block_hash)) {
            self.add_failure(peer_ip, "Received an invalid block response".to_string());
            false
        } else {
            if let Some(requests) = self.block_requests.get_mut(&peer_ip) {
                let is_success =
                    requests.remove(&(block_height, Some(block_hash))).is_some() || requests.remove(&(block_height, None)).is_some();
                match is_success {
                    true => return true,
                    false => self.add_failure(peer_ip, format!("Non-existent block request from {}", peer_ip)),
                }
            }
            false
        }
    }

    ///
    /// Removes block requests that have expired.
    ///
    fn remove_expired_block_requests(&mut self) {
        // Clear all block requests that have lived longer than `E::RADIO_SILENCE_IN_SECS`.
        let now = Utc::now().timestamp();
        self.block_requests.iter_mut().for_each(|(_peer, block_requests)| {
            block_requests.retain(|_, time_of_request| now.saturating_sub(*time_of_request) < E::RADIO_SILENCE_IN_SECS as i64)
        });
    }

    ///
    /// Adds the given failure message to the specified peer IP.
    ///
    fn add_failure(&mut self, peer_ip: SocketAddr, failure: String) {
        trace!("Adding failure for {}: {}", peer_ip, failure);
        match self.failures.get_mut(&peer_ip) {
            Some(failures) => failures.push((failure, Utc::now().timestamp())),
            None => error!("Missing failure entry for {}", peer_ip),
        };
    }

    ///
    /// Removes failures that have expired.
    ///
    fn remove_expired_failures(&mut self) {
        // Clear all failures that have lived longer than `E::FAILURE_EXPIRY_TIME_IN_SECS`.
        let now = Utc::now().timestamp();
        self.failures.iter_mut().for_each(|(_, failures)| {
            failures.retain(|(_, time_of_fail)| now.saturating_sub(*time_of_fail) < E::FAILURE_EXPIRY_TIME_IN_SECS as i64)
        });
    }

    ///
    /// Disconnects from connected peers who exhibit frequent failures.
    ///
    async fn disconnect_from_failing_peers(&self, ledger_router: &LedgerRouter<N, E>) {
        let peers_to_disconnect = self
            .failures
            .iter()
            .filter(|(_, failures)| failures.len() > E::MAXIMUM_NUMBER_OF_FAILURES)
            .map(|(peer_ip, _)| peer_ip);
        for peer_ip in peers_to_disconnect {
            if let Err(error) = ledger_router.send(LedgerRequest::Disconnect(*peer_ip)).await {
                warn!("Failed to send disconnect message to failing peer {}: {}", peer_ip, error);
            }
        }
    }
}
