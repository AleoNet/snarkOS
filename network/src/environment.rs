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

use crate::NetworkError;
use snarkos_consensus::{ConsensusParameters, MemoryPool, MerkleTreeLedger};
use snarkvm_dpc::base_dpc::{
    instantiated::{Components, Tx},
    parameters::PublicParameters,
};

use parking_lot::{Mutex, RwLock};
use std::{net::SocketAddr, sync::Arc, time::Duration};

#[derive(Clone)]
pub struct Consensus {
    /// The storage system of this node.
    storage: Arc<RwLock<MerkleTreeLedger>>,
    /// The memory pool of this node.
    memory_pool: Arc<Mutex<MemoryPool<Tx>>>,
    /// The consensus parameters for the associated network ID.
    consensus_parameters: ConsensusParameters,
    /// The DPC parameters for the associated network ID.
    dpc_parameters: PublicParameters<Components>,
    /// If `true`, initializes a mining task on this node.
    is_miner: bool,
    /// The interval between each block sync.
    block_sync_interval: Duration,
    /// The interval between each transaction (memory pool) sync.
    transaction_sync_interval: Duration,
}

impl Consensus {
    pub fn new(
        storage: Arc<RwLock<MerkleTreeLedger>>,
        memory_pool: Arc<Mutex<MemoryPool<Tx>>>,
        consensus_parameters: ConsensusParameters,
        dpc_parameters: PublicParameters<Components>,
        is_miner: bool,
        block_sync_interval: Duration,
        transaction_sync_interval: Duration,
    ) -> Self {
        Self {
            storage,
            memory_pool,
            consensus_parameters,
            dpc_parameters,
            is_miner,
            block_sync_interval,
            transaction_sync_interval,
        }
    }
}

/// A core data structure containing the networking parameters for this node.
#[derive(Clone)]
pub struct Environment {
    /// The objects related to consensus.
    consensus: Option<Consensus>,
    /// The local address of this node.
    local_address: Option<SocketAddr>,
    /// The minimum number of peers required to maintain connections with.
    minimum_number_of_connected_peers: u16,
    /// The maximum number of peers permitted to maintain connections with.
    maximum_number_of_connected_peers: u16,
    /// The default bootnodes of the network.
    bootnodes: Vec<SocketAddr>,
    /// If `true`, initializes this node as a bootnode and forgoes connecting
    /// to the default bootnodes or saved peers in the peer book.
    is_bootnode: bool,
    /// The interval between each peer sync.
    peer_sync_interval: Duration,
}

impl Environment {
    /// Creates a new instance of `Environment`.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        consensus: Option<Consensus>,
        local_address: Option<SocketAddr>,
        minimum_number_of_connected_peers: u16,
        maximum_number_of_connected_peers: u16,
        bootnodes_addresses: Vec<String>,
        is_bootnode: bool,
        peer_sync_interval: Duration,
    ) -> Result<Self, NetworkError> {
        // Convert the given bootnodes into socket addresses.
        let mut bootnodes = Vec::with_capacity(bootnodes_addresses.len());
        for bootnode_address in bootnodes_addresses.iter() {
            if let Ok(bootnode) = bootnode_address.parse::<SocketAddr>() {
                bootnodes.push(bootnode);
            }
        }

        Ok(Self {
            consensus,
            local_address,
            minimum_number_of_connected_peers,
            maximum_number_of_connected_peers,
            bootnodes,
            is_bootnode,
            peer_sync_interval,
        })
    }

    /// Returns a reference to the storage system of this node.
    #[inline]
    pub fn storage(&self) -> &Arc<RwLock<MerkleTreeLedger>> {
        &self.consensus.as_ref().expect("no consensus!").storage
    }

    /// Returns a reference to the memory pool of this node.
    #[inline]
    pub fn memory_pool(&self) -> &Arc<Mutex<MemoryPool<Tx>>> {
        &self.consensus.as_ref().expect("no consensus!").memory_pool
    }

    /// Returns a reference to the consensus parameters of this node.
    #[inline]
    pub fn consensus_parameters(&self) -> &ConsensusParameters {
        &self.consensus.as_ref().expect("no consensus!").consensus_parameters
    }

    /// Returns a reference to the DPC parameters of this node.
    #[inline]
    pub fn dpc_parameters(&self) -> &PublicParameters<Components> {
        &self.consensus.as_ref().expect("no consensus!").dpc_parameters
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

    #[inline]
    #[doc(hide)]
    pub fn has_consensus(&self) -> bool {
        self.consensus.is_some()
    }

    /// Returns `true` if this node is a mining node. Otherwise, returns `false`.
    #[inline]
    pub fn is_miner(&self) -> bool {
        self.consensus.as_ref().expect("no consensus!").is_miner
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

    /// Returns the current block height of the ledger from storage.
    #[inline]
    pub fn current_block_height(&self) -> u32 {
        self.consensus
            .as_ref()
            .expect("no consensus!")
            .storage
            .read()
            .get_current_block_height()
    }

    /// Returns the interval between each peer sync.
    pub fn peer_sync_interval(&self) -> Duration {
        self.peer_sync_interval
    }

    /// Returns the interval between each block sync.
    pub fn block_sync_interval(&self) -> Duration {
        self.consensus.as_ref().expect("no consensus!").block_sync_interval
    }

    /// Returns the interval between each transaction (memory pool) sync.
    pub fn transaction_sync_interval(&self) -> Duration {
        self.consensus
            .as_ref()
            .expect("no consensus!")
            .transaction_sync_interval
    }
}
