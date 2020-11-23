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

use crate::{
    external::{Handshakes, Pings},
    internal::{Connections, PeerBook},
};

use std::net::SocketAddr;
use tokio::sync::RwLock;

/// The network context for this node.
/// All variables are public to allow server components to acquire read/write access.
pub struct Context {
    /// The ip address/socket of this node.
    pub local_address: RwLock<SocketAddr>,

    /// Frequency the server requests memory pool transactions.
    pub memory_pool_interval: u8,

    /// Mininmum number of peers to connect to
    pub min_peers: u16,

    /// Maximum number of peers to connect to
    pub max_peers: u16,

    /// If enabled, node will not connect to bootnodes on startup.
    pub is_bootnode: bool,

    /// Hardcoded nodes and user-specified nodes this node should connect to on startup.
    pub bootnodes: Vec<String>,

    /// If enabled, node will operate as a miner
    pub is_miner: bool,

    /// Manages connected, gossiped, and disconnected peers
    pub peer_book: RwLock<PeerBook>,

    /// Handshakes to make connected peers
    pub handshakes: RwLock<Handshakes>,

    /// Connected peer channels for reading/writing messages
    pub connections: RwLock<Connections>,

    /// Ping/pongs with connected peers
    pub pings: RwLock<Pings>,
}

impl Context {
    /// Construct a new network `Context`.
    pub fn new(
        local_address: SocketAddr,
        memory_pool_interval: u8,
        min_peers: u16,
        max_peers: u16,
        is_bootnode: bool,
        bootnodes: Vec<String>,
        is_miner: bool,
    ) -> Self {
        Self {
            local_address: RwLock::new(local_address),
            memory_pool_interval,
            min_peers,
            max_peers,
            is_bootnode,
            bootnodes,
            is_miner,
            connections: RwLock::new(Connections::new()),
            peer_book: RwLock::new(PeerBook::new()),
            handshakes: RwLock::new(Handshakes::new()),
            pings: RwLock::new(Pings::new()),
        }
    }
}
