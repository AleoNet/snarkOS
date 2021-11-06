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

use crate::{Environment, Message, PeersRequest, PeersRouter};
use snarkos_ledger::{storage::Storage, LedgerState, Metadata};
use snarkvm::dpc::prelude::*;

use anyhow::{anyhow, Result};
use rand::thread_rng;
use std::{
    collections::{HashMap, HashSet},
    net::SocketAddr,
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use tokio::{sync::mpsc, task};

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
    /// SyncRequest := (peer_ip)
    SyncRequest(SocketAddr),
    /// SyncResponse := (peer_ip, \[(block height, block_hash)\])
    SyncResponse(SocketAddr, Vec<(u32, N::BlockHash)>),
    /// UnconfirmedBlock := (peer_ip, block)
    UnconfirmedBlock(SocketAddr, Block<N>),
    /// UnconfirmedTransaction := (peer_ip, transaction)
    UnconfirmedTransaction(SocketAddr, Transaction<N>),
}

///
/// A ledger for a specific network on the node server.
///
#[derive(Clone, Debug)]
pub struct Ledger<N: Network> {
    /// The canonical chain of block hashes.
    canon: LedgerState<N>,
    /// A map of previous block hashes to unconfirmed blocks.
    unconfirmed_blocks: HashMap<N::BlockHash, Block<N>>,
    /// The pool of unconfirmed transactions.
    memory_pool: MemoryPool<N>,
    /// A terminator bit for the miner.
    terminator: Arc<AtomicBool>,
    /// A status bit for the miner.
    is_mining: Arc<AtomicBool>,

    /// The map of each peer to their ledger state := (is_fork, common_ancestor, latest_block_height).
    ledger_state: HashMap<SocketAddr, Option<(bool, u32, u32)>>,
    /// The map of each peer to their block requests.
    block_requests: HashMap<SocketAddr, HashSet<u32>>,
    /// The map of each peer to their failure messages.
    failures: HashMap<SocketAddr, Vec<String>>,
    /// A boolean that is `true` when syncing.
    is_syncing: Arc<AtomicBool>,
}

impl<N: Network> Ledger<N> {
    /// Initializes a new instance of the ledger.
    pub fn open<S: Storage, P: AsRef<Path>>(path: P) -> Result<Self> {
        Ok(Self {
            canon: LedgerState::open::<S, P>(path)?,
            unconfirmed_blocks: Default::default(),
            memory_pool: MemoryPool::new(),
            terminator: Arc::new(AtomicBool::new(false)),
            is_mining: Arc::new(AtomicBool::new(false)),

            ledger_state: Default::default(),
            block_requests: Default::default(),
            failures: Default::default(),
            is_syncing: Arc::new(AtomicBool::new(false)),
        })
    }

    /// Returns `true` if the ledger is currently mining.
    pub fn is_mining(&self) -> bool {
        self.is_mining.load(Ordering::SeqCst)
    }

    /// Returns `true` if the ledger is currently syncing.
    pub fn is_syncing(&self) -> bool {
        self.is_syncing.load(Ordering::SeqCst)
    }

    /// Returns the latest block height.
    pub fn latest_block_height(&self) -> u32 {
        self.canon.latest_block_height()
    }

    /// Returns the latest block hash.
    pub fn latest_block_hash(&self) -> N::BlockHash {
        self.canon.latest_block_hash()
    }

    /// Returns the latest ledger root.
    pub fn latest_ledger_root(&self) -> N::LedgerRoot {
        self.canon.latest_ledger_root()
    }

    /// Returns the latest block locators.
    pub fn latest_block_locators(&self) -> &Vec<(u32, N::BlockHash)> {
        self.canon.latest_block_locators()
    }

    /// Returns the latest block timestamp.
    pub fn latest_block_timestamp(&self) -> Result<i64> {
        self.canon.latest_block_timestamp()
    }

    /// Returns the latest block difficulty target.
    pub fn latest_block_difficulty_target(&self) -> Result<u64> {
        self.canon.latest_block_difficulty_target()
    }

    /// Returns the latest block header.
    pub fn latest_block_header(&self) -> Result<BlockHeader<N>> {
        self.canon.latest_block_header()
    }

    /// Returns the transactions from the latest block.
    pub fn latest_block_transactions(&self) -> Result<Transactions<N>> {
        self.canon.latest_block_transactions()
    }

    /// Returns the latest block.
    pub fn latest_block(&self) -> Result<Block<N>> {
        self.canon.latest_block()
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

    /// Returns the previous block hash for the given block height.
    pub fn get_previous_block_hash(&self, block_height: u32) -> Result<N::BlockHash> {
        self.canon.get_previous_block_hash(block_height)
    }

    /// Returns the block header for the given block height.
    pub fn get_block_header(&self, block_height: u32) -> Result<BlockHeader<N>> {
        self.canon.get_block_header(block_height)
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

    /// Returns the block hashes from the given `start_block_height` to `end_block_height` (inclusive).
    pub fn get_block_hashes(&self, start_block_height: u32, end_block_height: u32) -> Result<Vec<N::BlockHash>> {
        self.canon.get_block_hashes(start_block_height, end_block_height)
    }

    /// Returns the ledger root in the block header of the given block height.
    pub fn get_previous_ledger_root(&self, block_height: u32) -> Result<N::LedgerRoot> {
        self.canon.get_previous_ledger_root(block_height)
    }

    /// Returns the block locators of the current ledger, from the given block height.
    pub fn get_block_locators(&self, block_height: u32) -> Result<Vec<(u32, N::BlockHash)>> {
        self.canon.get_block_locators(block_height)
    }

    /// Removes the latest `num_blocks` from storage, returning the removed blocks on success.
    pub fn remove_last_blocks(&mut self, num_blocks: u32) -> Result<Vec<Block<N>>> {
        self.canon.remove_last_blocks(num_blocks)
    }

    ///
    /// Performs the given `request` to the ledger.
    /// All requests must go through this `update`, so that a unified view is preserved.
    ///
    pub(super) async fn update<E: Environment>(&mut self, request: LedgerRequest<N, E>, peers_router: PeersRouter<N, E>) -> Result<()> {
        match request {
            LedgerRequest::BlockRequest(peer_ip, start_block_height, end_block_height) => {
                // Ensure the request is within the tolerated limit.
                match end_block_height - start_block_height <= E::MAXIMUM_BLOCK_REQUEST {
                    true => match self.get_blocks(start_block_height, end_block_height) {
                        Ok(blocks) => {
                            for block in blocks {
                                let request = PeersRequest::MessageSend(peer_ip, Message::BlockResponse(block));
                                if let Err(error) = peers_router.send(request).await {
                                    warn!("[BlockResponse] {}", error);
                                }
                            }
                        }
                        Err(error) => {
                            error!("{}", error);
                            self.add_failure(peer_ip, format!("{}", error));
                        }
                    },
                    false => {
                        // Record the failed request from the peer.
                        let num_blocks = end_block_height - start_block_height;
                        let failure = format!("Attempted to request {} blocks", num_blocks);
                        warn!("{}", failure);
                        self.add_failure(peer_ip, failure);
                    }
                }
                Ok(())
            }
            LedgerRequest::BlockResponse(peer_ip, block) => {
                // Ensure the block height corresponds to the requested block.
                if !self.contains_block_request(peer_ip, block.height()) {
                    self.add_failure(peer_ip, "Received block response for an unrequested block".to_string());
                }
                // Process the sync response.
                else {
                    // Remove the block request from the state manager.
                    self.remove_block_request(peer_ip, block.height());
                    // Ensure the block height corresponds to the requested block.
                    if block.height() != block.height() {
                        self.add_failure(peer_ip, "Block height does not match".to_string());
                    }
                    // Process the block response.
                    else {
                        if let Err(error) = self.add_block::<E>(&block) {
                            warn!("Failed to add a block to the ledger: {}", error);
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
                Ok(())
            }
            LedgerRequest::Disconnect(peer_ip) => {
                // Remove all entries of the peer from the state manager.
                self.remove_peer(&peer_ip);
                // Process the disconnect.
                info!("Disconnecting from {}", peer_ip);
                // Route a `PeerDisconnected` to the peers.
                if let Err(error) = peers_router.send(PeersRequest::PeerDisconnected(peer_ip)).await {
                    warn!("[Disconnect] {}", error);
                }
                Ok(())
            }
            LedgerRequest::Heartbeat => {
                // Check for candidate blocks to fast forward the ledger.
                let mut block = &self.latest_block()?;
                let unconfirmed_blocks = self.unconfirmed_blocks.clone();
                while let Some(unconfirmed_block) = unconfirmed_blocks.get(&block.block_hash()) {
                    // Update the block iterator.
                    block = unconfirmed_block;
                    // Attempt to add the unconfirmed block.
                    self.add_block::<E>(block)?;
                    // Upon success, remove the unconfirmed block, as it is now confirmed.
                    self.unconfirmed_blocks.remove(&block.block_hash());
                }

                // Send a sync request to each connected peer.
                let request = PeersRequest::MessageBroadcast(Message::SyncRequest);
                // Send a `SyncRequest` message to the peer.
                if let Err(error) = peers_router.send(request).await {
                    warn!("[SyncRequest] {}", error);
                }

                // Ensure the state manager is not already syncing.
                if self.is_syncing() {
                    return Ok(());
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
                    let latest_block_height = self.latest_block_height();
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

                Ok(())
            }
            LedgerRequest::Mine(local_ip, recipient, ledger_router) => {
                if let Err(error) = self.mine_next_block(local_ip, recipient, ledger_router) {
                    error!("Failed to mine the next block: {}", error)
                }
                Ok(())
            }
            LedgerRequest::SyncRequest(peer_ip) => {
                // Ensure the peer has been initialized in the ledger.
                self.initialize_peer(peer_ip);
                // Skip the sync request if this node is already syncing.
                if self.is_syncing() {
                    return Ok(());
                }
                // Process the sync request.
                else {
                    // Send a sync response to the peer.
                    let block_locators = self.latest_block_locators().clone();
                    let request = PeersRequest::MessageSend(peer_ip, Message::SyncResponse(block_locators));
                    peers_router.send(request).await?;
                    Ok(())
                }
            }
            LedgerRequest::SyncResponse(peer_ip, block_locators) => {
                // Ensure the list of block locators is not empty.
                if block_locators.len() == 0 {
                    self.add_failure(peer_ip, "Received a sync response with no block locators".to_string());
                    return Ok(());
                }
                // Process the sync response.
                else {
                    // Determine the common ancestor block height between this ledger and the peer.
                    let mut common_ancestor = 0;
                    // Determine the latest block height of the peer.
                    let mut latest_block_height_of_peer = 0;

                    // Verify the integrity of the block hashes sent by the peer.
                    for (block_height, block_hash) in block_locators {
                        // Ensure the block hash corresponds with the block height, if the block hash exists in this ledger.
                        if let Ok(expected_block_height) = self.get_block_height(&block_hash) {
                            if expected_block_height != block_height {
                                let error = format!("Invalid block height {} for block hash {}", expected_block_height, block_hash);
                                trace!("{}", error);
                                self.add_failure(peer_ip, error);
                                return Ok(());
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

                    // // Ensure any potential fork is within the maximum fork depth.
                    // if latest_block_height_of_peer - common_ancestor + 1 > E::MAXIMUM_FORK_DEPTH {
                    //     self.add_failure(peer_ip, "Received a sync response that exceeds the maximum fork depth".to_string());
                    //     return Ok(());
                    // }

                    // TODO (howardwu): If the distance of (latest_block_height_of_peer - common_ancestor) is less than 10,
                    //  manually check the fork status 1 by 1, as a slow response from the peer could make it look like a fork, based on this simple logic.
                    // Determine if the peer is a fork.
                    let is_fork = common_ancestor < self.latest_block_height() && latest_block_height_of_peer > self.latest_block_height();

                    // // Construct a HashMap of the block locators.
                    // let block_locators: HashMap<u32, N::BlockHash> = block_locators.iter().cloned().collect();
                    // let (start_block_height, end_block_height) = match (block_locators.keys().min(), block_locators.keys().max()) {
                    //     (Some(min), Some(max)) => (*min, *max),
                    //     _ => {
                    //         error!("Failed to find the starting and ending block height in a sync response");
                    //         return;
                    //     }
                    // };

                    trace!(
                        "{} is at block {} (common_ancestor = {})",
                        peer_ip,
                        latest_block_height_of_peer,
                        common_ancestor,
                    );

                    // trace!(
                    //     "{} is at block {} (is_fork = {}, common_ancestor = {})",
                    //     peer_ip,
                    //     latest_block_height_of_peer,
                    //     is_fork,
                    //     common_ancestor,
                    // );

                    // Update the ledger state of the peer.
                    self.update_ledger_state(peer_ip, (is_fork, common_ancestor, latest_block_height_of_peer));

                    return Ok(());
                }
            }
            LedgerRequest::UnconfirmedBlock(peer_ip, block) => {
                // Ensure the peer has been initialized in the ledger.
                self.initialize_peer(peer_ip);
                // Process the unconfirmed block.
                self.add_unconfirmed_block(peer_ip, block, peers_router.clone()).await
            }
            LedgerRequest::UnconfirmedTransaction(peer_ip, transaction) => {
                // Ensure the peer has been initialized in the ledger.
                self.initialize_peer(peer_ip);
                // Process the unconfirmed transaction.
                self.add_unconfirmed_transaction(peer_ip, transaction, peers_router.clone()).await
            }
        }
    }

    /// Mines a new block and adds it to the canon blocks.
    fn mine_next_block<E: Environment>(
        &self,
        local_ip: SocketAddr,
        recipient: Address<N>,
        ledger_router: LedgerRouter<N, E>,
    ) -> Result<()> {
        // Ensure the ledger is not already mining.
        match self.is_mining.load(Ordering::SeqCst) {
            true => return Ok(()),
            false => self.is_mining.store(true, Ordering::SeqCst),
        }

        // Prepare the new block.
        let previous_block_hash = self.latest_block_hash();
        let block_height = self.latest_block_height() + 1;

        // Compute the block difficulty target.
        let previous_timestamp = self.latest_block_timestamp()?;
        let previous_difficulty_target = self.latest_block_difficulty_target()?;
        let block_timestamp = chrono::Utc::now().timestamp();
        let difficulty_target = Blocks::<N>::compute_difficulty_target(previous_timestamp, previous_difficulty_target, block_timestamp);

        // Construct the ledger root and unconfirmed transactions.
        let ledger_root = self.canon.latest_ledger_root();
        let unconfirmed_transactions = self.memory_pool.transactions();
        let terminator = self.terminator.clone();
        let is_mining = self.is_mining.clone();

        task::spawn(async move {
            // Craft a coinbase transaction.
            let amount = Block::<N>::block_reward(block_height);
            let coinbase_transaction = match Transaction::<N>::new_coinbase(recipient, amount, &mut thread_rng()) {
                Ok(coinbase) => coinbase,
                Err(error) => {
                    error!("{}", error);
                    return;
                }
            };

            // Construct the new block transactions.
            let transactions = match Transactions::from(&[vec![coinbase_transaction], unconfirmed_transactions].concat()) {
                Ok(transactions) => transactions,
                Err(error) => {
                    error!("{}", error);
                    return;
                }
            };

            // Mine the next block.
            let block = match Block::mine(
                previous_block_hash,
                block_height,
                block_timestamp,
                difficulty_target,
                ledger_root,
                transactions,
                &terminator,
                &mut thread_rng(),
            ) {
                Ok(block) => {
                    // Set the mining status to off.
                    is_mining.store(false, Ordering::SeqCst);
                    block
                }
                Err(error) => {
                    error!("Failed to mine the next block: {}", error);
                    return;
                }
            };

            // Broadcast the next block.
            let request = LedgerRequest::UnconfirmedBlock(local_ip, block);
            if let Err(error) = ledger_router.send(request).await {
                error!("Failed to broadcast mined block: {}", error);
            }
        });

        Ok(())
    }

    ///
    /// Adds the given block:
    ///     1) as the next block in the ledger if the block height increments by one, or
    ///     2) to the memory pool for later use.
    ///
    fn add_block<E: Environment>(&mut self, block: &Block<N>) -> Result<()> {
        // Ensure the given block is new.
        if self.contains_block_hash(&block.block_hash())? {
            let error = format!("Canon chain already contains block {}", block.height());
            trace!("{}", error);
            Err(anyhow!("{}", error))
        } else if block.height() == self.latest_block_height() + 1 && block.previous_block_hash() == self.latest_block_hash() {
            match self.canon.add_next_block(block) {
                Ok(()) => {
                    info!("Ledger advanced to block {}", self.latest_block_height());
                    // On success, filter the memory pool of its transactions and the block if it exists.
                    let transactions = block.transactions();
                    self.memory_pool.remove_transactions(transactions);

                    let block_hash = block.block_hash();
                    if self.memory_pool.contains_block_hash(&block_hash) {
                        self.memory_pool.remove_block(&block_hash);
                    }

                    // TODO (howardwu) - Set the terminator bit to true.
                    Ok(())
                }
                Err(error) => {
                    warn!("{}", error);
                    Err(anyhow!("{}", error))
                }
            }
        } else {
            // Ensure the unconfirmed block is well-formed.
            match block.is_valid() {
                true => {
                    // Ensure the unconfirmed block does not already exist in the memory pool.
                    match !self.unconfirmed_blocks.contains_key(&block.previous_block_hash()) {
                        true => {
                            // Add the block to the unconfirmed blocks.
                            trace!("Adding unconfirmed block {} to memory pool", block.height());
                            self.unconfirmed_blocks.insert(block.previous_block_hash(), block.clone());
                            Ok(())
                        }
                        false => {
                            let error = format!("Unconfirmed block {} already exists in the memory pool", block.height());
                            trace!("{}", error);
                            Err(anyhow!(error))
                        }
                    }
                }
                false => {
                    let error = format!("Unconfirmed block {} is invalid", block.height());
                    warn!("{}", error);
                    Err(anyhow!(error))
                }
            }
        }
    }

    ///
    /// Adds the given unconfirmed block:
    ///     1) as the next block in the ledger if the block height increments by one, or
    ///     2) to the memory pool for later use.
    ///
    async fn add_unconfirmed_block<E: Environment>(
        &mut self,
        peer_ip: SocketAddr,
        block: Block<N>,
        peers_router: PeersRouter<N, E>,
    ) -> Result<()> {
        trace!("Received unconfirmed block {} from {}", block.height(), peer_ip);
        // Process the unconfirmed block.
        if self.add_block::<E>(&block).is_ok() {
            // Upon success, propagate the unconfirmed block to the connected peers.
            let request = PeersRequest::MessagePropagate(peer_ip, Message::UnconfirmedBlock(block));
            if let Err(error) = peers_router.send(request).await {
                warn!("[UnconfirmedBlock] {}", error);
            }
        }
        Ok(())
    }

    ///
    /// Adds the given unconfirmed transaction to the memory pool.
    ///
    async fn add_unconfirmed_transaction<E: Environment>(
        &mut self,
        peer_ip: SocketAddr,
        transaction: Transaction<N>,
        peers_router: PeersRouter<N, E>,
    ) -> Result<()> {
        // Process the unconfirmed transaction.
        trace!("Received unconfirmed transaction {} from {}", transaction.transaction_id(), peer_ip);
        // Ensure the unconfirmed transaction is new.
        if !self.contains_transaction(&transaction.transaction_id())? {
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
        Ok(())
    }

    ///
    /// Adds an entry for the given peer IP to every data structure in `State`.
    ///
    fn initialize_peer(&mut self, peer_ip: SocketAddr) {
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

    // ///
    // /// Processes a fork request, which contains a sequence of block hashes that
    // /// gives a insight to the ledger state of the peer.
    // ///
    // async fn process_sync_request<E: Environment>(
    //     &mut self,
    //     peer_ip: SocketAddr,
    //     block_hashes: Vec<(u32, N::BlockHash)>,
    //     peers_router: PeersRouter<N, E>,
    // ) -> Result<()> {
    //     // debug!("Handling fork request from peer {}", peer_ip);
    //     // if block_hashes.len() > 0 {
    //     //     // Find the last common block between this ledger and the peer, before forking.
    //     //     let mut fork_point_block_height = 0;
    //     //     // Find the latest block height of the peer.
    //     //     let mut latest_block_height_of_peer = 0;
    //     //
    //     //     // Verify the integrity of the block hashes sent by the peer.
    //     //     for (candidate_block_height, block_hash) in block_hashes {
    //     //         // Ensure the block hash corresponds with the block height, if the block hash exists in this ledger.
    //     //         if let Ok(expected_block_height) = self.canon.get_block_height(&block_hash) {
    //     //             if expected_block_height != candidate_block_height {
    //     //                 let error = format!("Invalid block height {} for block hash {}", expected_block_height, block_hash);
    //     //                 trace!("{}", error);
    //     //                 return Err(anyhow!(error));
    //     //             } else {
    //     //                 // Update the fork point block height, as this block hash exists in this ledger.
    //     //                 if expected_block_height > fork_point_block_height {
    //     //                     fork_point_block_height = expected_block_height
    //     //                 }
    //     //             }
    //     //         }
    //     //
    //     //         // Update the latest block height of the peer.
    //     //         if candidate_block_height > latest_block_height_of_peer {
    //     //             latest_block_height_of_peer = candidate_block_height
    //     //         }
    //     //     }
    //     //
    //     //     // TODO (raychu86): FORK - Consider if we just want to make this 1 directional. i.e. don't request blocks,
    //     //     //   just send if it's relevant.
    //     //
    //     //     // The peer is ahead of you.
    //     //     if latest_block_height_of_peer > self.latest_block_height() {
    //     //         debug!(
    //     //             "Peer {} is ahead of you. Current block height: {}. Peer block height: {}",
    //     //             peer_ip,
    //     //             self.latest_block_height(),
    //     //             latest_block_height_of_peer
    //     //         );
    //     //
    //     //         // Request new blocks from peer.
    //     //     } else if latest_block_height_of_peer < self.latest_block_height() {
    //     //         // You are ahead of your peer.
    //     //         debug!(
    //     //             "Sending fork response to peer {} for block heights between {} and {}",
    //     //             peer_ip,
    //     //             fork_point_block_height,
    //     //             self.latest_block_height()
    //     //         );
    //     //
    //     //         let request =
    //     //             PeersRequest::MessageSend(peer_ip, Message::SyncResponse(fork_point_block_height, self.latest_block_height()));
    //     //         peers_router.send(request).await?;
    //     //     }
    //     // }
    //     //
    //     // Ok(())
    // }

    // ///
    // /// Handles the fork request.
    // /// A fork request contains a sequence of block hashes that gives a insight to the
    // /// peers block state.
    // ///
    // async fn process_fork_response<E: Environment>(
    //     &mut self,
    //     peer_ip: SocketAddr,
    //     fork_point_block_height: u32,
    //     target_block_height: u32,
    // ) -> Result<()> {
    //     debug!("Handling fork response from peer {}", peer_ip);
    //
    //     if target_block_height > self.latest_block_height() {
    //         if target_block_height - fork_point_block_height <= E::FORK_THRESHOLD as u32 {
    //             // Remove latest blocks until the fork_point_block_height.
    //             while self.latest_block_height() > fork_point_block_height {
    //                 if let Err(error) = self.canon.remove_last_block() {
    //                     trace!("{}", error);
    //                     return Err(anyhow!("{}", error));
    //                 }
    //             }
    //
    //             // Regenerate the ledger tree.
    //             if let Err(error) = self.canon.regenerate_ledger_tree() {
    //                 trace!("{}", error);
    //                 return Err(anyhow!("{}", error));
    //             }
    //         } else {
    //             let error = format!(
    //                 "Fork size {} larger than fork threshold {}",
    //                 target_block_height - fork_point_block_height,
    //                 E::FORK_THRESHOLD
    //             );
    //             trace!("{}", error);
    //             return Err(anyhow!("{}", error));
    //         }
    //     } else {
    //         let error = format!(
    //             "Fork target height {} is not greater than current block height {}",
    //             target_block_height,
    //             self.latest_block_height()
    //         );
    //         trace!("{}", error);
    //         return Err(anyhow!("{}", error));
    //     }
    //
    //     Ok(())
    // }
}
