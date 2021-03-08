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

use crate::Node;
use snarkos_consensus::{ConsensusParameters, MemoryPool, MerkleTreeLedger};
use snarkvm_dpc::base_dpc::{
    instantiated::{Components, Tx},
    parameters::PublicParameters,
};
use snarkvm_objects::Storage;

use parking_lot::{Mutex, RwLock};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

// TODO: Remove the inner Arcs, currently these objects are being cloned individually in the miner.
pub struct Consensus<S: Storage> {
    /// The node this consensus is bound to.
    node: Node<S>,
    /// The storage system of this node.
    storage: Arc<MerkleTreeLedger<S>>,
    /// The memory pool of this node.
    memory_pool: Arc<Mutex<MemoryPool<Tx>>>,
    /// The consensus parameters for the associated network ID.
    consensus_parameters: Arc<ConsensusParameters>,
    /// The DPC parameters for the associated network ID.
    dpc_parameters: Arc<PublicParameters<Components>>,
    /// If `true`, initializes a mining task on this node.
    is_miner: bool,
    /// The interval between each block sync.
    block_sync_interval: Duration,
    /// The last time a block sync was initiated.
    last_block_sync: RwLock<Instant>,
    /// The interval between each transaction (memory pool) sync.
    transaction_sync_interval: Duration,
    /// Is the node currently syncing blocks?
    is_syncing_blocks: AtomicBool,
}

impl<S: Storage> Consensus<S> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        node: Node<S>,
        storage: Arc<MerkleTreeLedger<S>>,
        memory_pool: Arc<Mutex<MemoryPool<Tx>>>,
        consensus_parameters: Arc<ConsensusParameters>,
        dpc_parameters: Arc<PublicParameters<Components>>,
        is_miner: bool,
        block_sync_interval: Duration,
        transaction_sync_interval: Duration,
    ) -> Self {
        Self {
            node,
            storage,
            memory_pool,
            consensus_parameters,
            dpc_parameters,
            is_miner,
            block_sync_interval,
            last_block_sync: RwLock::new(Instant::now()),
            transaction_sync_interval,
            is_syncing_blocks: Default::default(),
        }
    }

    #[inline]
    pub fn node(&self) -> &Node<S> {
        &self.node
    }

    /// Returns a reference to the storage system of this node.
    #[inline]
    pub fn storage(&self) -> &Arc<MerkleTreeLedger<S>> {
        &self.storage
    }

    /// Returns a reference to the memory pool of this node.
    #[inline]
    pub fn memory_pool(&self) -> &Arc<Mutex<MemoryPool<Tx>>> {
        &self.memory_pool
    }

    /// Returns a reference to the consensus parameters of this node.
    #[inline]
    pub fn consensus_parameters(&self) -> &Arc<ConsensusParameters> {
        &self.consensus_parameters
    }

    /// Returns a reference to the DPC parameters of this node.
    #[inline]
    pub fn dpc_parameters(&self) -> &Arc<PublicParameters<Components>> {
        &self.dpc_parameters
    }

    /// Returns `true` if this node is a mining node. Otherwise, returns `false`.
    #[inline]
    pub fn is_miner(&self) -> bool {
        self.is_miner
    }

    /// Checks whether the node is currently syncing blocks.
    pub fn is_syncing_blocks(&self) -> bool {
        self.is_syncing_blocks.load(Ordering::SeqCst)
    }

    /// Register that the node is no longer syncing blocks.
    pub fn finished_syncing_blocks(&self) {
        self.is_syncing_blocks.store(false, Ordering::SeqCst);
    }

    /// Returns the current block height of the ledger from storage.
    #[inline]
    pub fn current_block_height(&self) -> u32 {
        self.storage.get_current_block_height()
    }

    /// Checks whether enough time has elapsed for the node to attempt another block sync.
    pub fn should_sync_blocks(&self) -> bool {
        !self.is_syncing_blocks() && self.last_block_sync.read().elapsed() > self.block_sync_interval
    }

    /// Register that the node attempted to sync blocks.
    pub fn register_block_sync_attempt(&self) {
        *self.last_block_sync.write() = Instant::now();
        self.is_syncing_blocks.store(true, Ordering::SeqCst);
    }

    /// Returns the interval between each transaction (memory pool) sync.
    pub fn transaction_sync_interval(&self) -> Duration {
        self.transaction_sync_interval
    }

    pub fn max_block_size(&self) -> usize {
        self.consensus_parameters.max_block_size
    }
}
