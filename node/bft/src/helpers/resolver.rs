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

use snarkvm::prelude::{Address, Network};

use parking_lot::RwLock;
use std::{collections::HashMap, net::SocketAddr};

#[derive(Debug)]
pub struct Resolver<N: Network> {
    /// The map of the listener address to (ambiguous) peer address.
    from_listener: RwLock<HashMap<SocketAddr, SocketAddr>>,
    /// The map of the (ambiguous) peer address to listener address.
    to_listener: RwLock<HashMap<SocketAddr, SocketAddr>>,
    /// A map of `peer IP` to `address`.
    peer_addresses: RwLock<HashMap<SocketAddr, Address<N>>>,
    /// A map of `address` to `peer IP`.
    address_peers: RwLock<HashMap<Address<N>, SocketAddr>>,
}

impl<N: Network> Default for Resolver<N> {
    /// Initializes a new instance of the resolver.
    fn default() -> Self {
        Self::new()
    }
}

impl<N: Network> Resolver<N> {
    /// Initializes a new instance of the resolver.
    pub fn new() -> Self {
        Self {
            from_listener: Default::default(),
            to_listener: Default::default(),
            peer_addresses: Default::default(),
            address_peers: Default::default(),
        }
    }
}

impl<N: Network> Resolver<N> {
    /// Returns the listener address for the given (ambiguous) peer address, if it exists.
    pub fn get_listener(&self, peer_addr: SocketAddr) -> Option<SocketAddr> {
        self.to_listener.read().get(&peer_addr).copied()
    }

    /// Returns the (ambiguous) peer address for the given listener address, if it exists.
    pub fn get_ambiguous(&self, peer_ip: SocketAddr) -> Option<SocketAddr> {
        self.from_listener.read().get(&peer_ip).copied()
    }

    /// Returns the address for the given peer IP.
    pub fn get_address(&self, peer_ip: SocketAddr) -> Option<Address<N>> {
        self.peer_addresses.read().get(&peer_ip).copied()
    }

    /// Returns the peer IP for the given address.
    pub fn get_peer_ip_for_address(&self, address: Address<N>) -> Option<SocketAddr> {
        self.address_peers.read().get(&address).copied()
    }

    /// Inserts a bidirectional mapping of the listener address and the (ambiguous) peer address,
    /// alongside a bidirectional mapping of the listener address and the Aleo address.
    pub fn insert_peer(&self, listener_ip: SocketAddr, peer_addr: SocketAddr, address: Address<N>) {
        self.from_listener.write().insert(listener_ip, peer_addr);
        self.to_listener.write().insert(peer_addr, listener_ip);
        self.peer_addresses.write().insert(listener_ip, address);
        self.address_peers.write().insert(address, listener_ip);
    }

    /// Removes the bidirectional mapping of the listener address and the (ambiguous) peer address,
    /// alongside the bidirectional mapping of the listener address and the Aleo address.
    pub fn remove_peer(&self, listener_ip: SocketAddr) {
        if let Some(peer_addr) = self.from_listener.write().remove(&listener_ip) {
            self.to_listener.write().remove(&peer_addr);
        }
        if let Some(address) = self.peer_addresses.write().remove(&listener_ip) {
            self.address_peers.write().remove(&address);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use snarkvm::{prelude::Rng, utilities::TestRng};

    type CurrentNetwork = snarkvm::prelude::MainnetV0;

    #[test]
    fn test_resolver() {
        let resolver = Resolver::<CurrentNetwork>::new();
        let listener_ip = SocketAddr::from(([127, 0, 0, 1], 1234));
        let peer_addr = SocketAddr::from(([127, 0, 0, 1], 4321));
        let mut rng = TestRng::default();
        let address = Address::<CurrentNetwork>::new(rng.gen());

        assert!(resolver.get_listener(peer_addr).is_none());
        assert!(resolver.get_address(listener_ip).is_none());
        assert!(resolver.get_ambiguous(listener_ip).is_none());
        assert!(resolver.get_peer_ip_for_address(address).is_none());

        resolver.insert_peer(listener_ip, peer_addr, address);

        assert_eq!(resolver.get_listener(peer_addr).unwrap(), listener_ip);
        assert_eq!(resolver.get_address(listener_ip).unwrap(), address);
        assert_eq!(resolver.get_ambiguous(listener_ip).unwrap(), peer_addr);
        assert_eq!(resolver.get_peer_ip_for_address(address).unwrap(), listener_ip);

        resolver.remove_peer(listener_ip);

        assert!(resolver.get_listener(peer_addr).is_none());
        assert!(resolver.get_address(listener_ip).is_none());
        assert!(resolver.get_ambiguous(listener_ip).is_none());
        assert!(resolver.get_peer_ip_for_address(address).is_none());
    }
}
