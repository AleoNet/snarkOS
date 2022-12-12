// Copyright (C) 2019-2022 Aleo Systems Inc.
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

use snarkos_node_messages::NodeType;
use snarkvm::prelude::{Address, Network};

use parking_lot::RwLock;
use std::{net::SocketAddr, sync::Arc, time::Instant};

/// The state for each connected peer.
#[derive(Clone, Debug)]
pub struct Peer<N: Network> {
    /// The IP address of the peer, with the port set to the listener port.
    peer_ip: SocketAddr,
    /// The Aleo address of the peer.
    address: Address<N>,
    /// The node type of the peer.
    node_type: NodeType,
    /// The message version of the peer.
    version: u32,
    /// The timestamp of the first message received from the peer.
    first_seen: Instant,
    /// The timestamp of the last message received from this peer.
    last_seen: Arc<RwLock<Instant>>,
}

impl<N: Network> Peer<N> {
    /// Initializes a new instance of `Peer`.
    pub fn new(listening_ip: SocketAddr, address: Address<N>, node_type: NodeType, version: u32) -> Self {
        Self {
            peer_ip: listening_ip,
            address,
            node_type,
            version,
            first_seen: Instant::now(),
            last_seen: Arc::new(RwLock::new(Instant::now())),
        }
    }

    /// Returns the IP address of the peer, with the port set to the listener port.
    pub const fn ip(&self) -> SocketAddr {
        self.peer_ip
    }

    /// Returns the Aleo address of the peer.
    pub const fn address(&self) -> Address<N> {
        self.address
    }

    /// Returns the node type.
    pub const fn node_type(&self) -> NodeType {
        self.node_type
    }

    /// Returns `true` if the peer is a beacon.
    pub const fn is_beacon(&self) -> bool {
        self.node_type.is_beacon()
    }

    /// Returns `true` if the peer is a validator.
    pub const fn is_validator(&self) -> bool {
        self.node_type.is_validator()
    }

    /// Returns `true` if the peer is a prover.
    pub const fn is_prover(&self) -> bool {
        self.node_type.is_prover()
    }

    /// Returns `true` if the peer is a client.
    pub const fn is_client(&self) -> bool {
        self.node_type.is_client()
    }

    /// Returns the message version of the peer.
    pub const fn version(&self) -> u32 {
        self.version
    }

    /// Returns the first seen timestamp of the peer.
    pub fn first_seen(&self) -> Instant {
        self.first_seen
    }

    /// Returns the last seen timestamp of the peer.
    pub fn last_seen(&self) -> Instant {
        *self.last_seen.read()
    }
}

impl<N: Network> Peer<N> {
    /// Updates the node type.
    pub fn set_node_type(&mut self, node_type: NodeType) {
        self.node_type = node_type;
    }

    /// Updates the version.
    pub fn set_version(&mut self, version: u32) {
        self.version = version;
    }

    /// Updates the last seen timestamp of the peer.
    pub fn set_last_seen(&self, last_seen: Instant) {
        *self.last_seen.write() = last_seen;
    }
}
