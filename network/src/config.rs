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

use parking_lot::RwLock;
use std::{
    net::SocketAddr,
    time::Duration,
    {self},
};

/// A core data structure containing the pre-configured parameters for the node.
pub struct Config {
    /// The pre-configured desired address of this node.
    pub desired_address: SocketAddr,
    /// The minimum number of peers required to maintain connections with.
    minimum_number_of_connected_peers: u16,
    /// The maximum number of peers permitted to maintain connections with.
    maximum_number_of_connected_peers: u16,
    /// The default bootnodes of the network.
    pub bootnodes: RwLock<Vec<SocketAddr>>,
    /// If `true`, initializes this node as a bootnode and forgoes connecting
    /// to the default bootnodes or saved peers in the peer book.
    is_bootnode: bool,
    /// The interval between each peer sync.
    peer_sync_interval: Duration,
}

impl Config {
    /// Creates a new instance of `Environment`.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        desired_address: SocketAddr,
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
            desired_address,
            minimum_number_of_connected_peers,
            maximum_number_of_connected_peers,
            bootnodes: RwLock::new(bootnodes),
            is_bootnode,
            peer_sync_interval,
        })
    }

    /// Returns the default bootnodes of the network.
    #[inline]
    pub fn bootnodes(&self) -> Vec<SocketAddr> {
        self.bootnodes.read().clone()
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
