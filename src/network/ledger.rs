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
use snarkos_ledger::{storage::Storage, LedgerState};
use snarkvm::dpc::prelude::*;

use anyhow::{anyhow, Result};
use rand::thread_rng;
use std::{
    collections::HashMap,
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
    /// Heartbeat := ()
    Heartbeat,
    /// Mine := (local_ip, miner_address, peers_router, ledger_router)
    Mine(SocketAddr, Address<N>, PeersRouter<N, E>, LedgerRouter<N, E>),
    /// ForkRequest := (peer_ip, block hashes and heights, peers_router)
    ForkRequest(SocketAddr, Vec<(u32, N::BlockHash)>, PeersRouter<N, E>),
    // /// SyncRequest := (peer_ip, block_height, peers_router)
    // SyncRequest(SocketAddr, u32, PeersRouter<N, E>),
    /// SyncResponse := (block)
    SyncResponse(Block<N>),
    /// UnconfirmedBlock := (peer_ip, block, peers_router)
    UnconfirmedBlock(SocketAddr, Block<N>, PeersRouter<N, E>),
    /// UnconfirmedTransaction := (peer_ip, transaction, peers_router)
    UnconfirmedTransaction(SocketAddr, Transaction<N>, PeersRouter<N, E>),
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
        })
    }

    /// Returns `true` if the ledger is currently mining.
    pub fn is_mining(&self) -> bool {
        self.is_mining.load(Ordering::SeqCst)
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

    /// Returns the blocks from `start_block_height` to `end_block_height`.
    pub fn get_blocks(&self, start_block_height: u32, end_block_height: u32) -> Result<Vec<Block<N>>> {
        self.canon.get_blocks(start_block_height, end_block_height)
    }

    /// Returns the the ledger's current block locator hashes.
    pub fn get_block_locator_hashes(&self) -> Result<Vec<(u32, N::BlockHash)>> {
        const NUM_LOCATOR_HASHES: u32 = 64;

        let block_height = self.latest_block_height();

        // The number of locator hashes left to obtain; accounts for the genesis block.
        let mut num_locator_hashes = std::cmp::min(NUM_LOCATOR_HASHES - 1, block_height);

        // The output list of block locator hashes.
        let mut block_locator_hashes = Vec::with_capacity(num_locator_hashes as usize);

        // The index of the current block for which a locator hash is obtained.
        let mut hash_index = block_height;

        // The number of top blocks to provide locator hashes for.
        let num_top_blocks = std::cmp::min(10, num_locator_hashes);

        for _ in 0..num_top_blocks {
            block_locator_hashes.push((hash_index, self.get_block_hash(hash_index)?));
            hash_index -= 1; // safe; num_top_blocks is never higher than the height
        }

        num_locator_hashes -= num_top_blocks;
        if num_locator_hashes == 0 {
            block_locator_hashes.push((0, self.get_block_hash(0)?));
            return Ok(block_locator_hashes);
        }

        // Calculate the average distance between block hashes based on the desired number of locator hashes.
        let mut proportional_step = hash_index / num_locator_hashes;

        // Provide hashes of blocks with indices descending quadratically while the quadratic step distance is
        // lower or close to the proportional step distance.
        let num_quadratic_steps = (proportional_step as f32).log2() as u32;

        // The remaining hashes should have a proportional index distance between them.
        let num_proportional_steps = num_locator_hashes - num_quadratic_steps;

        // Obtain a few hashes increasing the distance quadratically.
        let mut quadratic_step = 2; // the size of the first quadratic step
        for _ in 0..num_quadratic_steps {
            block_locator_hashes.push((hash_index, self.get_block_hash(hash_index)?));
            hash_index = hash_index.saturating_sub(quadratic_step);
            quadratic_step *= 2;
        }

        // Update the size of the proportional step so that the hashes of the remaining blocks have the same distance
        // between one another.
        proportional_step = hash_index / num_proportional_steps;

        // Tweak: in order to avoid "jumping" by too many indices with the last step,
        // increase the value of each step by 1 if the last step is too large. This
        // can result in the final number of locator hashes being a bit lower, but
        // it's preferable to having a large gap between values.
        if hash_index - proportional_step * num_proportional_steps > 2 * proportional_step {
            proportional_step += 1;
        }

        // Obtain the rest of hashes with a proportional distance between them.
        for _ in 0..num_proportional_steps {
            block_locator_hashes.push((hash_index, self.get_block_hash(hash_index)?));
            if hash_index == 0 {
                return Ok(block_locator_hashes);
            }
            hash_index = hash_index.saturating_sub(proportional_step);
        }

        block_locator_hashes.push((0, self.get_block_hash(0)?));

        Ok(block_locator_hashes)
    }

    ///
    /// Performs the given `request` to the ledger.
    /// All requests must go through this `update`, so that a unified view is preserved.
    ///
    pub(super) async fn update<E: Environment>(&mut self, request: LedgerRequest<N, E>) -> Result<()> {
        match request {
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
                Ok(())
            }
            LedgerRequest::Mine(local_ip, recipient, peers_router, ledger_router) => {
                if let Err(error) = self.mine_next_block(local_ip, recipient, peers_router, ledger_router) {
                    error!("Failed to mine the next block: {}", error)
                }
                Ok(())
            }
            LedgerRequest::ForkRequest(peer_ip, block_headers, peers_router) => {
                self.handle_fork_request(peer_ip, block_headers, peers_router).await
            }
            // LedgerRequest::SyncRequest(peer_ip, block_height, peers_router) => {
            //     let request = match self.get_block(block_height) {
            //         Ok(block) => PeersRequest::MessageSend(peer_ip, Message::SyncResponse(block.height(), block)),
            //         Err(error) => PeersRequest::Failure(peer_ip, format!("{}", error)),
            //     };
            //     peers_router.send(request).await?;
            //     Ok(())
            // }
            LedgerRequest::SyncResponse(block) => self.add_block::<E>(&block),
            LedgerRequest::UnconfirmedBlock(peer_ip, block, peers_router) => self.add_unconfirmed_block(peer_ip, block, peers_router).await,
            LedgerRequest::UnconfirmedTransaction(peer_ip, transaction, peers_router) => {
                self.add_unconfirmed_transaction(peer_ip, transaction, peers_router).await
            }
        }
    }

    /// Mines a new block and adds it to the canon blocks.
    fn mine_next_block<E: Environment>(
        &mut self,
        local_ip: SocketAddr,
        recipient: Address<N>,
        peers_router: PeersRouter<N, E>,
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
            let request = LedgerRequest::UnconfirmedBlock(local_ip, block, peers_router);
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
                    // TODO (howardwu): Filter the memory pool, removing any now confirmed transctions.
                    self.memory_pool.clear_transactions();
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
        // Process the unconfirmed block.
        if self.add_block::<E>(&block).is_ok() {
            // Upon success, propagate the unconfirmed block to the connected peers.
            let request = PeersRequest::MessagePropagate(peer_ip, Message::UnconfirmedBlock(block.height(), block));
            peers_router.send(request).await?;
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
        debug!("Adding unconfirmed transaction {} to memory pool", transaction.transaction_id());
        // Ensure the unconfirmed transaction is new.
        if !self.contains_transaction(&transaction.transaction_id())? {
            // Attempt to add the unconfirmed transaction to the memory pool.
            match self.memory_pool.add_transaction(&transaction) {
                Ok(()) => {
                    // Upon success, propagate the unconfirmed transaction to the connected peers.
                    let request = PeersRequest::MessagePropagate(peer_ip, Message::UnconfirmedTransaction(transaction));
                    peers_router.send(request).await?;
                }
                Err(error) => error!("{}", error),
            }
        }
        Ok(())
    }

    ///
    /// Handles the fork request.
    /// A fork request contains a sequence of block hashes that gives a insight to the
    /// peers block state.
    ///
    async fn handle_fork_request<E: Environment>(
        &mut self,
        peer_ip: SocketAddr,
        block_hashes: Vec<(u32, N::BlockHash)>,
        peers_router: PeersRouter<N, E>,
    ) -> Result<()> {
        if block_hashes.len() > 0 {
            // The most recent block height shared with the peer.
            let mut latest_shared_block_height = 0;

            // The latest block height the peer has shared.
            let mut latest_peer_block_height = 0;

            // Scan the block hashes sent by the peer.
            for (block_height, block_hash) in block_hashes {
                if let Ok(expected_block_height) = self.canon.get_block_height(&block_hash) {
                    // Check if the declared height for the block hash is valid.
                    if expected_block_height != block_height {
                        let error = format!("Invalid block height {} for block hash {}", expected_block_height, block_hash);
                        trace!("{}", error);
                        return Err(anyhow!(error));
                    } else {
                        // Update the latest shared block height.
                        if expected_block_height > latest_shared_block_height {
                            latest_shared_block_height = expected_block_height
                        }

                        // Update the latest peer block height.
                        if expected_block_height > latest_peer_block_height {
                            latest_peer_block_height = expected_block_height
                        }
                    }
                }
            }

            // TODO (raychu86): Consider if we just want to make this 1 directional. i.e. don't request blocks,
            //   just send if it's relevant.

            // The peer is ahead of you.
            if latest_peer_block_height > self.latest_block_height() {
                // Request new blocks from peer.

                // Check that a fork would be under the fork threshold.
                if latest_peer_block_height - latest_shared_block_height < E::FORK_THRESHOLD as u32 {
                    // TODO (raychu86): Request blocks from latest_shared_block_height to latest_peer_block_height.
                } else {
                    let error = format!(
                        "Fork of size {} is larger than the fork threshold {}",
                        latest_peer_block_height - latest_shared_block_height,
                        E::FORK_THRESHOLD
                    );
                    trace!("{}", error);
                    return Err(anyhow!(error));
                }
            } else if latest_peer_block_height < self.latest_block_height() {
                // You are ahead of your peer.
                info!("");

                // TODO (raychu86): Give peer information about new blocks.
                let request = PeersRequest::MessageSend(peer_ip, Message::ForkResponse);
                peers_router.send(request).await?;
            }
        }

        Ok(())
    }
}
