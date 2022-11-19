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

use indexmap::IndexMap;
use parking_lot::RwLock;
use std::{net::SocketAddr, sync::Arc};

#[derive(Clone, Debug)]
pub(crate) struct Resolver {
    /// The map of the listener address to (ambiguous) peer address.
    from_listener: Arc<RwLock<IndexMap<SocketAddr, SocketAddr>>>,
    /// The map of the (ambiguous) peer address to listener address.
    to_listener: Arc<RwLock<IndexMap<SocketAddr, SocketAddr>>>,
}

impl Default for Resolver {
    /// Initializes a new instance of the resolver.
    fn default() -> Self {
        Self::new()
    }
}

impl Resolver {
    /// Initializes a new instance of the resolver.
    pub fn new() -> Self {
        Self { from_listener: Default::default(), to_listener: Default::default() }
    }

    /// Returns the listener address for the given (ambiguous) peer address, if it exists.
    pub fn get_listener(&self, peer_addr: &SocketAddr) -> Option<SocketAddr> {
        self.to_listener.read().get(peer_addr).copied()
    }

    /// Returns the (ambiguous) peer address for the given listener address, if it exists.
    pub fn get_ambiguous(&self, peer_ip: &SocketAddr) -> Option<SocketAddr> {
        self.from_listener.read().get(peer_ip).copied()
    }

    /// Inserts a bidirectional mapping of the listener address and the (ambiguous) peer address.
    pub fn insert_peer(&self, listener_ip: SocketAddr, peer_addr: SocketAddr) {
        self.from_listener.write().insert(listener_ip, peer_addr);
        self.to_listener.write().insert(peer_addr, listener_ip);
    }

    /// Removes the bidirectional mapping of the listener address and the (ambiguous) peer address.
    pub fn remove_peer(&self, listener_ip: &SocketAddr) {
        if let Some(peer_addr) = self.from_listener.write().remove(listener_ip) {
            self.to_listener.write().remove(&peer_addr);
        }
    }
}
