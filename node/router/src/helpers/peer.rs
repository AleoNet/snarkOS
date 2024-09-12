// Copyright 2024 Aleo Network Foundation
// This file is part of the snarkOS library.

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at:

// http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::messages::{ChallengeRequest, NodeType};
use snarkvm::prelude::{Address, Network};

use std::{net::SocketAddr, time::Instant};

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
    last_seen: Instant,
}

impl<N: Network> Peer<N> {
    /// Initializes a new instance of `Peer`.
    pub fn new(listening_ip: SocketAddr, challenge_request: &ChallengeRequest<N>) -> Self {
        Self {
            peer_ip: listening_ip,
            address: challenge_request.address,
            node_type: challenge_request.node_type,
            version: challenge_request.version,
            first_seen: Instant::now(),
            last_seen: Instant::now(),
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
        self.last_seen
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
    pub fn set_last_seen(&mut self, last_seen: Instant) {
        self.last_seen = last_seen;
    }
}
