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
use snarkos_ledger::{storage::Storage, BlockLocators, LedgerState, Metadata, MAXIMUM_LINEAR_BLOCK_LOCATORS};
use snarkvm::dpc::prelude::*;

use anyhow::Result;
use rand::thread_rng;
use std::{
    collections::{BTreeMap, HashMap, HashSet},
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
const MAXIMUM_UNCONFIRMED_BLOCKS: u32 = 1024;

/// Shorthand for the parent half of the `Ledger` message channel.
pub(crate) type LedgerRouter<N, E> = mpsc::Sender<LedgerRequest<N, E>>;
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
    Heartbeat,
    /// Mine := (local_ip, miner_address, ledger_router)
    Mine(SocketAddr, Address<N>, LedgerRouter<N, E>),
    /// Ping := (peer_ip)
    Ping(SocketAddr),
    /// Pong := (peer_ip, block_locators)
    Pong(SocketAddr, BlockLocators<N>),
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
pub struct Ledger<N: Network, E: Environment> {
    /// The status of the ledger.
    status: Arc<AtomicU8>,
    /// The canonical chain of block hashes.
    canon: LedgerState<N>,
    /// A map of previous block hashes to unconfirmed blocks.
    unconfirmed_blocks: CircularMap<N::BlockHash, Block<N>, { MAXIMUM_UNCONFIRMED_BLOCKS }>,
    /// The pool of unconfirmed transactions.
    memory_pool: MemoryPool<N>,
    /// A terminator bit for the miner.
    terminator: Arc<AtomicBool>,

    /// The map of each peer to their ledger state := (common_ancestor, latest_block_height, block_locators).
    peers_state: HashMap<SocketAddr, Option<(u32, u32, BlockLocators<N>)>>,
    /// The map of each peer to their block requests.
    block_requests: HashMap<SocketAddr, HashSet<(u32, Option<N::BlockHash>)>>,
    /// The latest block height requested from a peer.
    latest_block_request: u32,
    /// The timestamp of the last successful block increment.
    last_block_increment_timestamp: Instant,
    /// The map of each peer to their failure messages.
    failures: HashMap<SocketAddr, Vec<String>>,
    _phantom: PhantomData<E>,
}

impl<N: Network, E: Environment> Ledger<N, E> {
    /// Initializes a new instance of the ledger.
    pub fn open<S: Storage, P: AsRef<Path>>(path: P) -> Result<Self> {
        let canon = LedgerState::open::<S, P>(path, false)?;
        let latest_block_request = canon.latest_block_height();
        let last_block_increment_timestamp = Instant::now();
        Ok(Self {
            status: Arc::new(AtomicU8::new(Status::Peering as u8)),
            canon,
            unconfirmed_blocks: Default::default(),
            memory_pool: MemoryPool::new(),
            terminator: Arc::new(AtomicBool::new(false)),

            peers_state: Default::default(),
            block_requests: Default::default(),
            latest_block_request,
            last_block_increment_timestamp,
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

    /// Returns the latest block.
    pub fn latest_block(&self) -> &Block<N> {
        self.canon.latest_block()
    }

    /// Returns the latest block height.
    pub fn latest_block_height(&self) -> u32 {
        self.canon.latest_block_height()
    }

    /// Returns the latest block hash.
    pub fn latest_block_hash(&self) -> N::BlockHash {
        self.canon.latest_block_hash()
    }

    /// Returns the latest block timestamp.
    pub fn latest_block_timestamp(&self) -> i64 {
        self.canon.latest_block_timestamp()
    }

    /// Returns the latest block difficulty target.
    pub fn latest_block_difficulty_target(&self) -> u64 {
        self.canon.latest_block_difficulty_target()
    }

    /// Returns the latest block header.
    pub fn latest_block_header(&self) -> &BlockHeader<N> {
        self.canon.latest_block_header()
    }

    /// Returns the transactions from the latest block.
    pub fn latest_block_transactions(&self) -> &Transactions<N> {
        self.canon.latest_block_transactions()
    }

    /// Returns the latest block locators.
    pub fn latest_block_locators(&self) -> BlockLocators<N> {
        self.canon.latest_block_locators()
    }

    /// Returns the latest ledger root.
    pub fn latest_ledger_root(&self) -> N::LedgerRoot {
        self.canon.latest_ledger_root()
    }

    /// Returns `true` if the given ledger root exists in storage.
    pub fn contains_ledger_root(&self, ledger_root: &N::LedgerRoot) -> Result<bool> {
        self.canon.contains_ledger_root(ledger_root)
    }

    /// Returns `true` if the given block height exists in storage.
    pub fn contains_block_height(&self, block_height: u32) -> Result<bool> {
        self.canon.contains_block_height(block_height)
    }

    /// Returns `true` if the given block hash exists in storage.
    pub fn contains_block_hash(&self, block_hash: &N::BlockHash) -> Result<bool> {
        self.canon.contains_block_hash(block_hash)
    }

    /// Returns `true` if the given transaction ID exists in storage.
    pub fn contains_transaction(&self, transaction_id: &N::TransactionID) -> Result<bool> {
        self.canon.contains_transaction(transaction_id)
    }

    /// Returns `true` if the given serial number exists in storage.
    pub fn contains_serial_number(&self, serial_number: &N::SerialNumber) -> Result<bool> {
        self.canon.contains_serial_number(serial_number)
    }

    /// Returns `true` if the given commitment exists in storage.
    pub fn contains_commitment(&self, commitment: &N::Commitment) -> Result<bool> {
        self.canon.contains_commitment(commitment)
    }

    /// Returns `true` if the given ciphertext ID exists in storage.
    pub fn contains_ciphertext_id(&self, ciphertext_id: &N::CiphertextID) -> Result<bool> {
        self.canon.contains_ciphertext_id(ciphertext_id)
    }

    /// Returns the record ciphertext for a given ciphertext ID.
    pub fn get_ciphertext(&self, ciphertext_id: &N::CiphertextID) -> Result<RecordCiphertext<N>> {
        self.canon.get_ciphertext(ciphertext_id)
    }

    /// Returns the transition for a given transition ID.
    pub fn get_transition(&self, transition_id: &N::TransitionID) -> Result<Transition<N>> {
        self.canon.get_transition(transition_id)
    }

    /// Returns the transaction for a given transaction ID.
    pub fn get_transaction(&self, transaction_id: &N::TransactionID) -> Result<Transaction<N>> {
        self.canon.get_transaction(transaction_id)
    }

    /// Returns the transaction metadata for a given transaction ID.
    pub fn get_transaction_metadata(&self, transaction_id: &N::TransactionID) -> Result<Metadata<N>> {
        self.canon.get_transaction_metadata(transaction_id)
    }

    /// Returns the block height for the given block hash.
    pub fn get_block_height(&self, block_hash: &N::BlockHash) -> Result<u32> {
        self.canon.get_block_height(block_hash)
    }

    /// Returns the block hash for the given block height.
    pub fn get_block_hash(&self, block_height: u32) -> Result<N::BlockHash> {
        self.canon.get_block_hash(block_height)
    }

    /// Returns the block hashes from the given `start_block_height` to `end_block_height` (inclusive).
    pub fn get_block_hashes(&self, start_block_height: u32, end_block_height: u32) -> Result<Vec<N::BlockHash>> {
        self.canon.get_block_hashes(start_block_height, end_block_height)
    }

    /// Returns the previous block hash for the given block height.
    pub fn get_previous_block_hash(&self, block_height: u32) -> Result<N::BlockHash> {
        self.canon.get_previous_block_hash(block_height)
    }

    /// Returns the block header for the given block height.
    pub fn get_block_header(&self, block_height: u32) -> Result<BlockHeader<N>> {
        self.canon.get_block_header(block_height)
    }

    /// Returns the block headers from the given `start_block_height` to `end_block_height` (inclusive).
    pub fn get_block_headers(&self, start_block_height: u32, end_block_height: u32) -> Result<Vec<BlockHeader<N>>> {
        self.canon.get_block_headers(start_block_height, end_block_height)
    }

    /// Returns the transactions from the block of the given block height.
    pub fn get_block_transactions(&self, block_height: u32) -> Result<Transactions<N>> {
        self.canon.get_block_transactions(block_height)
    }

    /// Returns the block for a given block height.
    pub fn get_block(&self, block_height: u32) -> Result<Block<N>> {
        self.canon.get_block(block_height)
    }

    /// Returns the blocks from the given `start_block_height` to `end_block_height` (inclusive).
    pub fn get_blocks(&self, start_block_height: u32, end_block_height: u32) -> Result<Vec<Block<N>>> {
        self.canon.get_blocks(start_block_height, end_block_height)
    }

    /// Returns the ledger root and ledger inclusion proof for a given record commitment.
    pub fn get_ledger_inclusion_proof(&self, record_commitment: &N::Commitment) -> Result<LedgerProof<N>> {
        self.canon.get_ledger_inclusion_proof(*record_commitment)
    }

    /// Returns the ledger root in the block header of the given block height.
    pub fn get_previous_ledger_root(&self, block_height: u32) -> Result<N::LedgerRoot> {
        self.canon.get_previous_ledger_root(block_height)
    }

    pub fn calculate_weight_from_height(&self, block_height: u32) -> Result<f64> {
        Ok(self
            .get_block_headers(block_height, self.latest_block_height())?
            .iter()
            .fold(0f64, |acc, header| acc + 1f64 / (header.difficulty_target() as f64)))
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
                let blocks = match self.get_blocks(start_block_height, end_block_height) {
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
            LedgerRequest::Heartbeat => {
                // Update the ledger.
                self.update_ledger();
                // Update the status of the ledger.
                self.update_status();
                // Update the block requests.
                self.update_block_requests(peers_router).await;
            }
            LedgerRequest::Mine(local_ip, recipient, ledger_router) => {
                // Process the request to mine the next block.
                self.mine_next_block(local_ip, recipient, ledger_router);
            }
            LedgerRequest::Ping(peer_ip) => {
                // Send a `Pong` message to the peer.
                let request = PeersRequest::MessageSend(peer_ip, Message::Pong(self.latest_block_locators()));
                if let Err(error) = peers_router.send(request).await {
                    warn!("[Pong] {}", error);
                }
            }
            LedgerRequest::Pong(peer_ip, block_locators) => {
                // Ensure the peer has been initialized in the ledger.
                self.initialize_peer(peer_ip);
                // Process the pong.
                self.update_peer(peer_ip, block_locators, peers_router).await;
            }
            LedgerRequest::UnconfirmedBlock(peer_ip, block) => {
                // Ensure the given block is new.
                if let Ok(true) = self.contains_block_hash(&block.hash()) {
                    trace!("Canon chain already contains block {}", block.height());
                } else if self.unconfirmed_blocks.contains_key(&block.previous_block_hash()) {
                    trace!("Memory pool already contains unconfirmed block {}", block.height());
                } else {
                    // Process the unconfirmed block.
                    self.add_block(block.clone());
                    // Propagate the unconfirmed block to the connected peers.
                    let request = PeersRequest::MessagePropagate(peer_ip, Message::UnconfirmedBlock(block));
                    if let Err(error) = peers_router.send(request).await {
                        warn!("[UnconfirmedBlock] {}", error);
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
        let mut block = self.latest_block();
        let unconfirmed_blocks = self.unconfirmed_blocks.clone();
        while let Some(unconfirmed_block) = unconfirmed_blocks.get(&block.hash()) {
            // Update the block iterator.
            block = unconfirmed_block;
            // Attempt to add the unconfirmed block.
            self.add_block(block.clone());
            // Upon success, remove the unconfirmed block, as it is now confirmed.
            self.unconfirmed_blocks.remove(&block.hash());
        }

        // If the timestamp of the last block increment has surpassed the preset limit,
        // the ledger is likely syncing from invalid state, and should rollback by one block.
        if self.is_syncing() && self.last_block_increment_timestamp.elapsed() > Duration::from_secs(E::MAXIMUM_RADIO_SILENCE_IN_SECS) {
            trace!("Ledger state has become stale, clearing queue and rolling back");
            self.unconfirmed_blocks = Default::default();
            self.memory_pool = MemoryPool::new();
            self.block_requests = Default::default();
            self.remove_last_blocks(1);
            self.last_block_increment_timestamp = Instant::now();
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
        else if self.peers_state.len() < E::MINIMUM_NUMBER_OF_PEERS {
            return;
        }
        // If `terminator` is `true`, it should not be mining.
        else if self.terminator.load(Ordering::SeqCst) {
            return;
        }
        // If the status is `Ready`, mine the next block.
        else if self.status() == Status::Ready {
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
                            error!("Failed to broadcast mined block: {}", error);
                        }
                    }
                    Err(error) => error!("Failed to mine the next block: {}", error),
                }
            });
        }
    }

    ///
    /// Adds the given block:
    ///     1) as the next block in the ledger if the block height increments by one, or
    ///     2) to the memory pool for later use.
    ///
    fn add_block(&mut self, block: Block<N>) {
        // Ensure the given block is new.
        if let Ok(true) = self.contains_block_hash(&block.hash()) {
            trace!("Canon chain already contains block {}", block.height());
        } else if block.height() == self.latest_block_height() + 1 && block.previous_block_hash() == self.latest_block_hash() {
            match self.canon.add_next_block(&block) {
                Ok(()) => {
                    info!("Ledger advanced to block {}", self.latest_block_height());

                    // Set the terminator bit to `true` to ensure the miner updates state.
                    self.terminator.store(true, Ordering::SeqCst);

                    // Update the timestamp of the last block increment.
                    self.last_block_increment_timestamp = Instant::now();

                    // On success, filter the memory pool of its transactions and the block if it exists.
                    let transactions = block.transactions();
                    self.memory_pool.remove_transactions(transactions);

                    let block_hash = block.hash();
                    if self.memory_pool.contains_block_hash(&block_hash) {
                        self.memory_pool.remove_block(&block_hash);
                    }
                }
                Err(error) => {
                    debug!("Setting latest block request to block {}", self.latest_block_height());
                    self.latest_block_request = self.latest_block_height();
                    warn!("{}", error);
                }
            }
        } else {
            // Ensure the unconfirmed block is well-formed.
            match block.is_valid() {
                true => {
                    // Ensure the unconfirmed block does not already exist in the memory pool.
                    match !self.unconfirmed_blocks.contains_key(&block.previous_block_hash()) {
                        true => {
                            // Set the terminator bit to `true` to ensure the miner updates state.
                            self.terminator.store(true, Ordering::SeqCst);

                            // Add the block to the unconfirmed blocks.
                            trace!("Adding unconfirmed block {} to memory pool", block.height());
                            self.unconfirmed_blocks.insert(block.previous_block_hash(), block);
                        }
                        false => trace!("Unconfirmed block {} already exists in the memory pool", block.height()),
                    }
                }
                false => warn!("Unconfirmed block {} is invalid", block.height()),
            }
        }
    }

    ///
    /// Adds the given unconfirmed transaction to the memory pool.
    ///
    async fn add_unconfirmed_transaction(&mut self, peer_ip: SocketAddr, transaction: Transaction<N>, peers_router: &PeersRouter<N, E>) {
        // Process the unconfirmed transaction.
        trace!("Received unconfirmed transaction {} from {}", transaction.transaction_id(), peer_ip);
        // Ensure the unconfirmed transaction is new.
        if let Ok(false) = self.contains_transaction(&transaction.transaction_id()) {
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
    /// Removes the latest `num_blocks` from storage, returning the successfully removed blocks.
    ///
    fn remove_last_blocks(&mut self, num_blocks: u32) -> Vec<Block<N>> {
        match self.canon.remove_last_blocks(num_blocks) {
            Ok(removed_blocks) => {
                let latest_block_height = self.latest_block_height();
                info!("Ledger rolled back to block {}", latest_block_height);
                self.latest_block_request = latest_block_height;
                removed_blocks
            }
            Err(error) => {
                error!("Failed to roll ledger back: {}", error);
                vec![]
            }
        }
    }

    ///
    /// Adds an entry for the given peer IP to every data structure in `State`.
    ///
    fn initialize_peer(&mut self, peer_ip: SocketAddr) {
        if !self.peers_state.contains_key(&peer_ip) {
            self.peers_state.insert(peer_ip, None);
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
    async fn update_peer(&mut self, peer_ip: SocketAddr, block_locators: BlockLocators<N>, peers_router: &PeersRouter<N, E>) {
        // Ensure the list of block locators is not empty.
        if block_locators.len() == 0 {
            self.add_failure(peer_ip, "Received a sync response with no block locators".to_string());
        } else {
            // Sort the block locators into a map.
            // TODO (howardwu): Clean this up by unifying them into one design.
            let original_locators = block_locators.clone();
            let block_locators: BTreeMap<u32, (N::BlockHash, Option<BlockHeader<N>>)> =
                block_locators.iter().cloned().map(|(a, b, c)| (a, (b, c))).collect();

            // Ensure the peer provided the genesis block locator, and that the genesis block hash is correct.
            match block_locators.get(&0) {
                Some((genesis_block_hash, _)) => {
                    if *genesis_block_hash != N::genesis_block().hash() {
                        warn!("Incorrect genesis block locator from {}", peer_ip);
                        self.add_failure(peer_ip, "Incorrect genesis block locator".to_string());
                        return;
                    }
                }
                None => {
                    warn!("Missing genesis block locator from {}", peer_ip);
                    self.add_failure(peer_ip, "Missing genesis block locator".to_string());
                    return;
                }
            };

            // Determine the common ancestor block height between this ledger and the peer.
            let mut common_ancestor = 0;
            // Determine the latest block height of the peer.
            let mut latest_block_height_of_peer = 0;

            // Verify the integrity of the block hashes sent by the peer.
            for (block_height, (block_hash, _)) in &block_locators {
                // Ensure the block hash corresponds with the block height, if the block hash exists in this ledger.
                if let Ok(expected_block_height) = self.get_block_height(block_hash) {
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
            trace!(
                "Peer {} is at block {} (common_ancestor = {})",
                peer_ip,
                latest_block_height_of_peer,
                common_ancestor,
            );

            // If this ledger is within the fork range of the peer,
            // and the common ancestor is outside of the fork range,
            // and the peer has a higher block height, this peer is malicious.
            // Proceed to disconnect immediately.
            if latest_block_height_of_peer.saturating_sub(self.latest_block_height()) <= MAXIMUM_LINEAR_BLOCK_LOCATORS
                && latest_block_height_of_peer.saturating_sub(common_ancestor) > MAXIMUM_LINEAR_BLOCK_LOCATORS
            {
                warn!("Peer {} is malicious, aborting", common_ancestor);
                // Route the `Disconnect` request.
                let request = PeersRequest::MessageSend(peer_ip, Message::Disconnect);
                if let Err(error) = peers_router.send(request).await {
                    warn!("[Disconnect] {}", error);
                }
            } else {
                match self.peers_state.get_mut(&peer_ip) {
                    Some(status) => *status = Some((common_ancestor, latest_block_height_of_peer, original_locators)),
                    None => self.add_failure(peer_ip, format!("Missing ledger state for {}", peer_ip)),
                };
            }
        }
    }

    ///
    /// Proceeds to send block requests to a connected peer, if the ledger is out of date.
    ///
    async fn update_block_requests(&mut self, peers_router: &PeersRouter<N, E>) {
        // Ensure the ledger is not awaiting responses from outstanding block requests.
        if self.number_of_block_requests() > 0 {
            return;
        }

        // Iterate through the peers to check if this node needs to catch up, and determine a peer to sync with.
        let mut maximal_peer = None;
        let mut maximum_common_ancestor = 0;
        let mut maximum_block_height = self.latest_block_request;
        let mut maximum_block_locators = Default::default();
        for (peer_ip, ledger_state) in self.peers_state.iter() {
            if let Some((common_ancestor, block_height, block_locators)) = ledger_state {
                if *block_height > maximum_block_height {
                    maximal_peer = Some(*peer_ip);
                    maximum_common_ancestor = *common_ancestor;
                    maximum_block_height = *block_height;
                    maximum_block_locators = block_locators.clone();
                }
            }
        }

        // Proceed to add block requests if the maximum block height is higher than the latest.
        if let Some(peer_ip) = maximal_peer {
            {
                // TODO (howardwu): Clean this up, by unifying the call with the same logic in `update_peer`.
                // Determine the common ancestor block height between this ledger and the peer.
                let mut maximum_common_ancestor = maximum_common_ancestor;
                // Verify the integrity of the block hashes sent by the peer.
                for (block_height, block_hash, _) in &*maximum_block_locators {
                    // Ensure the block hash corresponds with the block height, if the block hash exists in this ledger.
                    if let Ok(expected_block_height) = self.get_block_height(block_hash) {
                        if expected_block_height != *block_height {
                            let error = format!("Invalid block height {} for block hash {}", expected_block_height, block_hash);
                            trace!("{}", error);
                            self.add_failure(peer_ip, error);
                            return;
                        } else {
                            // Update the common ancestor, as this block hash exists in this ledger.
                            if expected_block_height > maximum_common_ancestor {
                                maximum_common_ancestor = expected_block_height
                            }
                        }
                    }
                }

                // If this ledger is within the fork range of the peer,
                // and the common ancestor is within the fork range,
                // and the peer has a heavier chain, proceed to switch to the fork.
                let latest_block_height = self.latest_block_height();
                let canon_weight = match self.calculate_weight_from_height(maximum_common_ancestor) {
                    Ok(weight) => weight,
                    Err(error) => {
                        error!("Failed to calculate canon chain weight: {}", error);
                        return;
                    }
                };

                let maximum_weight = {
                    let mut maximum_weight = 0f64;
                    for (block_height, _, block_header) in maximum_block_locators.iter() {
                        if *block_height >= maximum_common_ancestor {
                            if let Some(header) = block_header {
                                maximum_weight += 1f64 / (header.difficulty_target() as f64);
                            }
                        }
                    }

                    maximum_weight
                };

                if maximum_weight > canon_weight
                    && maximum_block_height.saturating_sub(latest_block_height) <= MAXIMUM_LINEAR_BLOCK_LOCATORS
                    && maximum_block_height.saturating_sub(maximum_common_ancestor) > 0
                    && maximum_block_height.saturating_sub(maximum_common_ancestor) <= MAXIMUM_LINEAR_BLOCK_LOCATORS
                    && latest_block_height.saturating_sub(maximum_common_ancestor) > 0
                {
                    info!("Found a heavier fork, rolling ledger back to block {}", maximum_common_ancestor);

                    // Set the terminator bit to `true` to ensure it does not mine.
                    self.terminator.store(true, Ordering::SeqCst);

                    // TODO (howardwu): Change the remove_last_blocks method to take the target block height, instead of the length.
                    let num_blocks = latest_block_height.saturating_sub(maximum_common_ancestor);
                    self.remove_last_blocks(num_blocks);
                }
            }

            // Determine the specific blocks to sync with the peer.
            let num_blocks = std::cmp::min(maximum_block_height - self.latest_block_request, E::MAXIMUM_BLOCK_REQUEST);
            let start_block_height = self.latest_block_request + 1;
            let end_block_height = start_block_height + num_blocks - 1;

            // Send a `BlockRequest` message to the peer.
            debug!("Request blocks {} to {} from {}", start_block_height, end_block_height, peer_ip);
            let request = PeersRequest::MessageSend(peer_ip, Message::BlockRequest(start_block_height, end_block_height));
            if let Err(error) = peers_router.send(request).await {
                warn!("[BlockRequest] {}", error);
                return;
            }

            // Log each block request to ensure the peer responds with all requested blocks.
            for block_height in start_block_height..=end_block_height {
                // Add the block request to the ledger.
                self.add_block_request(peer_ip, block_height, None);
            }
            // Update the latest block height requested from a peer.
            self.latest_block_request = end_block_height;
        }
    }

    ///
    /// Returns the number of outstanding block requests.
    ///
    fn number_of_block_requests(&self) -> usize {
        self.block_requests.values().map(|r| r.len()).fold(0usize, |a, b| a + b)
    }

    ///
    /// Adds a block request for the given block height to the specified peer.
    ///
    fn add_block_request(&mut self, peer_ip: SocketAddr, block_height: u32, block_hash: Option<N::BlockHash>) {
        // Ensure the block request does not already exist.
        if !self.contains_block_request(peer_ip, block_height, block_hash) {
            match self.block_requests.get_mut(&peer_ip) {
                Some(requests) => match requests.insert((block_height, block_hash)) {
                    true => debug!("Requesting block {} from {}", block_height, peer_ip),
                    false => self.add_failure(peer_ip, format!("Duplicate block request for {}", peer_ip)),
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
            Some(requests) => requests.contains(&(block_height, block_hash)) || requests.contains(&(block_height, None)),
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
                let is_success = requests.remove(&(block_height, Some(block_hash))) || requests.remove(&(block_height, None));
                match is_success {
                    true => return true,
                    false => self.add_failure(peer_ip, format!("Non-existent block request from {}", peer_ip)),
                }
            }
            false
        }
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
