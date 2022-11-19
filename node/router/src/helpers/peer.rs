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

use snarkos_node_messages::{NodeType, RawStatus};
use snarkos_node_tcp::ConnectionSide;

use parking_lot::RwLock;
use std::{
    net::SocketAddr,
    sync::{atomic::AtomicU32, Arc},
    time::Instant,
};

/// The state for each connected peer.
#[derive(Clone, Debug)]
pub struct Peer {
    /// The connection side of the peer.
    #[allow(dead_code)]
    side: ConnectionSide,
    /// The IP address of the peer, with the port set to the listener port.
    listening_addr: SocketAddr,
    /// The timestamp of the last message received from this peer.
    last_seen: Arc<RwLock<Instant>>,
    /// The message version of the peer.
    version: u32,
    /// The node type of the peer.
    node_type: NodeType,
    /// The node type of the peer.
    status: RawStatus,
    /// The block height of the peer.
    #[allow(dead_code)]
    block_height: Arc<AtomicU32>,
}

impl Peer {
    /// Initializes a new instance of `Peer`.
    pub fn new(
        side: ConnectionSide,
        listening_addr: SocketAddr,
        version: u32,
        node_type: NodeType,
        status: RawStatus,
    ) -> Self {
        Self {
            side,
            listening_addr,
            last_seen: Arc::new(RwLock::new(Instant::now())),
            version,
            node_type,
            status,
            block_height: Default::default(),
        }
    }

    /// Returns the IP address of the peer, with the port set to the listener port.
    pub fn ip(&self) -> SocketAddr {
        self.listening_addr
    }

    /// Returns the last seen timestamp of the peer.
    pub fn last_seen(&self) -> Instant {
        *self.last_seen.read()
    }

    /// Returns the node type.
    pub fn node_type(&self) -> NodeType {
        self.node_type
    }

    /// Returns `true` if the peer is a beacon.
    pub fn is_beacon(&self) -> bool {
        self.node_type.is_beacon()
    }

    /// Returns `true` if the peer is a validator.
    pub fn is_validator(&self) -> bool {
        self.node_type.is_validator()
    }

    /// Returns `true` if the peer is a prover.
    pub fn is_prover(&self) -> bool {
        self.node_type.is_prover()
    }

    /// Returns `true` if the peer is a client.
    pub fn is_client(&self) -> bool {
        self.node_type.is_client()
    }
}

impl Peer {
    /// Updates the last seen timestamp of the peer.
    pub fn set_last_seen(&self, last_seen: Instant) {
        *self.last_seen.write() = last_seen;
    }

    /// Updates the version.
    pub fn set_version(&mut self, version: u32) {
        self.version = version
    }

    /// Updates the node type.
    pub fn set_node_type(&mut self, node_type: NodeType) {
        self.node_type = node_type
    }

    /// Updates the status.
    pub fn set_status(&mut self, status: RawStatus) {
        self.status = status
    }
}
