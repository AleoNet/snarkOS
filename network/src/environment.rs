// Copyright (C) 2019-2020 Aleo Systems Inc.
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

use crate::NetworkError;
use snarkos_consensus::{ConsensusParameters, MemoryPool, MerkleTreeLedger};
use snarkvm_dpc::base_dpc::{
    instantiated::{Components, Tx},
    parameters::PublicParameters,
};
use snarkvm_objects::Network;

use parking_lot::{Mutex, RwLock};
use std::{net::SocketAddr, sync::Arc};

/// A core data structure containing the networking parameters for this node.
#[derive(Clone)]
pub struct Environment {
    /// TODO (howardwu): Rearchitect the ledger to be thread safe with shared ownership.
    /// The storage system of this node.
    storage: Arc<RwLock<MerkleTreeLedger>>,
    /// The memory pool of this node.
    memory_pool: Arc<Mutex<MemoryPool<Tx>>>,
    /// The consensus parameters for the associated network ID.
    consensus_parameters: Arc<ConsensusParameters>,
    /// The DPC parameters for the associated network ID.
    dpc_parameters: Arc<PublicParameters<Components>>,
    /// The network ID of this node.
    network_id: Network,

    /// The local address of this node.
    local_address: Option<SocketAddr>,

    /// The minimum number of peers required to maintain connections with.
    minimum_number_of_connected_peers: u16,
    /// The maximum number of peers permitted to maintain connections with.
    maximum_number_of_connected_peers: u16,

    /// TODO (howardwu): Rename CONNECTION_FREQUENCY to this.
    /// The number of milliseconds this node waits to perform a periodic sync with its peers.
    sync_interval: u64,
    /// TODO (howardwu): this is not in seconds. deprecate this and rearchitect it.
    /// The number of seconds this node waits to request memory pool transactions from its peers.
    memory_pool_interval: u8,

    /// The default bootnodes of the network.
    bootnodes: Vec<SocketAddr>,
    /// If `true`, initializes this node as a bootnode and forgoes connecting
    /// to the default bootnodes or saved peers in the peer book.
    is_bootnode: bool,
    /// If `true`, initializes a mining task on this node.
    is_miner: bool,
}

impl Environment {
    /// Creates a new instance of `Environment`.
    #[inline]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        storage: Arc<RwLock<MerkleTreeLedger>>,
        memory_pool: Arc<Mutex<MemoryPool<Tx>>>,
        consensus_parameters: Arc<ConsensusParameters>,
        dpc_parameters: Arc<PublicParameters<Components>>,

        local_address: Option<SocketAddr>,

        minimum_number_of_connected_peers: u16,
        maximum_number_of_connected_peers: u16,
        sync_interval: u64,
        memory_pool_interval: u8,

        bootnodes_addresses: Vec<String>,
        is_bootnode: bool,
        is_miner: bool,
    ) -> Result<Self, NetworkError> {
        // Check that the minimum and maximum number of peers is valid.
        if minimum_number_of_connected_peers == 0 || maximum_number_of_connected_peers == 0 {
            return Err(NetworkError::PeerCountInvalid);
        }

        // Check that the sync interval is a reasonable number of seconds.
        if !(2..=300).contains(&sync_interval) {
            return Err(NetworkError::SyncIntervalInvalid);
        }

        // TODO (howardwu): Check the memory pool interval.

        // Convert the given bootnodes into socket addresses.
        let mut bootnodes = Vec::with_capacity(bootnodes_addresses.len());
        for bootnode_address in bootnodes_addresses.iter() {
            if let Ok(bootnode) = bootnode_address.parse::<SocketAddr>() {
                bootnodes.push(bootnode);
            }
        }

        // Derive the network ID.
        let network_id = consensus_parameters.network_id;

        Ok(Self {
            storage,
            memory_pool,
            consensus_parameters,
            dpc_parameters,
            network_id,

            local_address,

            minimum_number_of_connected_peers,
            maximum_number_of_connected_peers,
            sync_interval,
            memory_pool_interval,

            bootnodes,
            is_bootnode,
            is_miner,
        })
    }

    /// Returns a reference to the storage system of this node.
    #[inline]
    pub fn storage(&self) -> &Arc<RwLock<MerkleTreeLedger>> {
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

    /// Returns the local address of the node.
    #[inline]
    pub fn local_address(&self) -> Option<SocketAddr> {
        self.local_address
    }

    /// Sets the local address of the node to the given value.
    #[inline]
    pub fn set_local_address(&mut self, addr: SocketAddr) {
        self.local_address = Some(addr);
    }

    /// Returns a reference to the default bootnodes of the network.
    #[inline]
    pub fn bootnodes(&self) -> &Vec<SocketAddr> {
        &self.bootnodes
    }

    /// Returns `true` if this node is a bootnode. Otherwise, returns `false`.
    #[inline]
    pub fn is_bootnode(&self) -> bool {
        self.is_bootnode
    }

    /// Returns `true` if this node is a mining node. Otherwise, returns `false`.
    #[inline]
    pub fn is_miner(&self) -> bool {
        self.is_miner
    }

    /// Returns the minimum number of peers this node maintains a connection with.
    #[inline]
    pub fn minimum_number_of_connected_peers(&self) -> u16 {
        self.minimum_number_of_connected_peers
    }

    /// Returns the maximum number of peers this node maintains a connection with.
    #[inline]
    pub fn maximum_number_of_connected_peers(&self) -> u16 {
        self.maximum_number_of_connected_peers
    }

    /// Returns the sync interval of this node.
    #[inline]
    pub fn sync_interval(&self) -> u64 {
        self.sync_interval
    }

    /// Returns the memory pool interval of this node.
    #[inline]
    pub fn memory_pool_interval(&self) -> u8 {
        self.memory_pool_interval
    }

    /// Returns the current block height of the ledger from storage.
    #[inline]
    pub async fn current_block_height(&self) -> u32 {
        self.storage.read().get_current_block_height()
    }
}
