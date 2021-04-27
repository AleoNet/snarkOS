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

use crate::{Node, State};
use snarkos_consensus::{ConsensusParameters, MemoryPool, MerkleTreeLedger};
use snarkvm_dpc::base_dpc::{
    instantiated::{Components, Tx},
    parameters::PublicParameters,
};
use snarkvm_objects::Storage;

use parking_lot::{Mutex, RwLock};
use std::{
    net::SocketAddr,
    sync::Arc,
    time::{Duration, Instant},
};

/// The sync handler of this node.
pub struct Sync<S: Storage> {
    /// The node this sync is bound to.
    node: Node<S>,
    /// The core sync objects.
    pub consensus: Arc<snarkos_consensus::Consensus<S>>,
    /// If `true`, initializes a mining task on this node.
    is_miner: bool,
    /// The interval between each block sync.
    block_sync_interval: Duration,
    /// The last time a block sync was initiated.
    last_block_sync: RwLock<Option<Instant>>,
    /// The interval between each transaction (memory pool) sync.
    transaction_sync_interval: Duration,
}

impl<S: Storage> Sync<S> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        node: Node<S>,
        consensus: Arc<snarkos_consensus::Consensus<S>>,
        is_miner: bool,
        block_sync_interval: Duration,
        transaction_sync_interval: Duration,
    ) -> Self {
        Self {
            node,
            consensus,
            is_miner,
            block_sync_interval,
            last_block_sync: Default::default(),
            transaction_sync_interval,
        }
    }

    #[inline]
    pub fn node(&self) -> &Node<S> {
        &self.node
    }

    /// Returns a reference to the storage system of this node.
    #[inline]
    pub fn storage(&self) -> &MerkleTreeLedger<S> {
        &self.consensus.ledger
    }

    /// Returns a reference to the memory pool of this node.
    #[inline]
    pub fn memory_pool(&self) -> &Mutex<MemoryPool<Tx>> {
        &self.consensus.memory_pool
    }

    /// Returns a reference to the sync parameters of this node.
    #[inline]
    pub fn consensus_parameters(&self) -> &ConsensusParameters {
        &self.consensus.parameters
    }

    /// Returns a reference to the DPC parameters of this node.
    #[inline]
    pub fn dpc_parameters(&self) -> &PublicParameters<Components> {
        &self.consensus.public_parameters
    }

    /// Returns `true` if this node is a mining node. Otherwise, returns `false`.
    #[inline]
    pub fn is_miner(&self) -> bool {
        self.is_miner
    }

    /// Checks whether the node is currently syncing blocks.
    pub fn is_syncing_blocks(&self) -> bool {
        self.node.state() == State::Syncing
    }

    /// Register that the node is no longer syncing blocks.
    pub fn finished_syncing_blocks(&self) {
        self.node.set_state(State::Idle);
    }

    /// Returns the current block height of the ledger from storage.
    #[inline]
    pub fn current_block_height(&self) -> u32 {
        self.consensus.ledger.get_current_block_height()
    }

    /// Checks whether any previous sync attempt has expired.
    pub fn has_block_sync_expired(&self) -> bool {
        if let Some(ref timestamp) = *self.last_block_sync.read() {
            timestamp.elapsed() > self.block_sync_interval
        } else {
            // this means it's the very first sync attempt
            true
        }
    }

    /// Register that the node attempted to sync blocks with the given peer.
    pub fn register_block_sync_attempt(&self, provider: SocketAddr) {
        trace!("Attempting to sync with {}", provider);
        *self.last_block_sync.write() = Some(Instant::now());
    }

    /// Returns the interval between each transaction (memory pool) sync.
    pub fn transaction_sync_interval(&self) -> Duration {
        self.transaction_sync_interval
    }

    pub fn max_block_size(&self) -> usize {
        self.consensus.parameters.max_block_size
    }
}
