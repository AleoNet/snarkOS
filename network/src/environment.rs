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

use crate::{Consensus, NetworkError};
use snarkos_consensus::{ConsensusParameters, MemoryPool, MerkleTreeLedger};
use snarkvm_dpc::base_dpc::{
    instantiated::{Components, Tx},
    parameters::PublicParameters,
};

use parking_lot::{Mutex, RwLock};
use std::{
    net::SocketAddr,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

/// A core data structure containing the networking parameters for this node.
#[derive(Clone)]
pub struct Environment {
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
            local_address,
            minimum_number_of_connected_peers,
            maximum_number_of_connected_peers,
            bootnodes,
            is_bootnode,
            peer_sync_interval,
        })
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

    /// Returns the interval between each peer sync.
    pub fn peer_sync_interval(&self) -> Duration {
        self.peer_sync_interval
    }
}
