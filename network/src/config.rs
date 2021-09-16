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
use serde::{Deserialize, Serialize};
use std::{
    net::SocketAddr,
    sync::Arc,
    time::Duration,
    {self},
};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum NodeType {
    /// The default "regular" node.
    Client,
    /// Crawls the network to track the known network.
    Crawler,
    /// Used for peer discovery, they are the entry point into the network.
    Beacon,
    /// Used for initial sync after joining the network.
    ///
    /// Sync provider addresses are shared by the beacons.
    SyncProvider,
}

/// A core data structure containing the pre-configured parameters for the node.
pub struct Config {
    /// The desired numeric ID of the node.
    pub node_id: Option<u64>,
    pub node_type: NodeType,
    /// The pre-configured desired address of this node.
    pub desired_address: SocketAddr,
    /// The minimum number of peers required to maintain connections with.
    minimum_number_of_connected_peers: u16,
    /// The maximum number of peers permitted to maintain connections with.
    maximum_number_of_connected_peers: u16,
    /// The default peer discovery nodes of the network.
    pub beacons: ArcSwap<Vec<SocketAddr>>,
    /// The default sync provider nodes of the network.
    sync_providers: ArcSwap<Vec<SocketAddr>>,
    /// The interval between each peer sync.
    peer_sync_interval: Duration,
}

impl Config {
    /// Creates a new instance of `Environment`.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        node_id: Option<u64>,
        node_type: NodeType,
        desired_address: SocketAddr,
        minimum_number_of_connected_peers: u16,
        maximum_number_of_connected_peers: u16,
        beacon_addresses: Vec<String>,
        sync_provider_addresses: Vec<String>,
        peer_sync_interval: Duration,
    ) -> Result<Self, NetworkError> {
        // Convert the given seeded nodes into socket addresses.
        let beacons: Vec<SocketAddr> = beacon_addresses
            .into_iter()
            .filter_map(|addr| addr.parse().ok())
            .collect();

        let sync_providers: Vec<SocketAddr> = sync_provider_addresses
            .into_iter()
            .filter_map(|addr| addr.parse().ok())
            .collect();

        Ok(Self {
            node_id,
            node_type,
            desired_address,
            minimum_number_of_connected_peers,
            maximum_number_of_connected_peers,
            beacons: ArcSwap::new(Arc::new(beacons)),
            sync_providers: ArcSwap::new(Arc::new(sync_providers)),
            peer_sync_interval,
        })
    }

    /// Returns the default peer discovery nodes of the network.
    #[inline]
    pub fn beacons(&self) -> Arc<Vec<SocketAddr>> {
        self.beacons.load_full()
    }

    /// Returns the default sync provider nodes of the network.
    #[inline]
    pub fn sync_providers(&self) -> Arc<Vec<SocketAddr>> {
        self.sync_providers.load_full()
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
