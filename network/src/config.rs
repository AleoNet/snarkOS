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

use arc_swap::ArcSwap;
use std::{
    net::SocketAddr,
    sync::Arc,
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
    pub bootnodes: ArcSwap<Vec<SocketAddr>>,
    /// The initial (non-bootnode) addresses this node should connect to.
    pub initial_peers: ArcSwap<Vec<SocketAddr>>,
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
        bootnodes_addrs: Vec<String>,
        initial_peer_addrs: Vec<String>,
        is_bootnode: bool,
        peer_sync_interval: Duration,
    ) -> Result<Self, NetworkError> {
        // Convert the given bootnodes into socket addresses.
        let mut bootnodes = Vec::with_capacity(bootnodes_addrs.len());
        for bootnode_addr in bootnodes_addrs.iter() {
            if let Ok(bootnode) = bootnode_addr.parse::<SocketAddr>() {
                bootnodes.push(bootnode);
            }
        }

        let mut initial_peers = Vec::with_capacity(initial_peer_addrs.len());
        for initial_peer_addr in initial_peer_addrs {
            if let Ok(addr) = initial_peer_addr.parse::<SocketAddr>() {
                initial_peers.push(addr);
            }
        }

        Ok(Self {
            desired_address,
            minimum_number_of_connected_peers,
            maximum_number_of_connected_peers,
            bootnodes: ArcSwap::new(Arc::new(bootnodes)),
            initial_peers: ArcSwap::new(Arc::new(initial_peers)),
            is_bootnode,
            peer_sync_interval,
        })
    }

    /// Returns the default bootnodes of the network.
    #[inline]
    pub fn bootnodes(&self) -> Arc<Vec<SocketAddr>> {
        self.bootnodes.load_full()
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
