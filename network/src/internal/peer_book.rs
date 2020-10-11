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

use crate::{internal::PeerInfo, NetworkError};
use snarkos_metrics::Metrics;
use snarkos_models::{algorithms::LoadableMerkleParameters, objects::Transaction};
use snarkos_storage::Ledger;

use serde::{Deserialize, Serialize};
use std::{collections::HashMap, net::SocketAddr};

/// An data structure for tracking and indexing the history of
/// all connected and disconnected peers to this node.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PeerBook {
    /// The local address of this node.
    local_address: SocketAddr,
    /// The mapping of connected peers to their metadata.
    connected_peers: HashMap<SocketAddr, PeerInfo>,
    /// The mapping of disconnected peers to their metadata.
    disconnected_peers: HashMap<SocketAddr, PeerInfo>,
}

impl PeerBook {
    /// Creates a new instance of `PeerBook`.
    #[inline]
    pub fn new(local_address: SocketAddr) -> Self {
        Self {
            local_address,
            connected_peers: HashMap::default(),
            disconnected_peers: HashMap::default(),
        }
    }

    // TODO (howardwu): Implement manual serializers and deserializers to prevent forward breakage
    //  when the PeerBook or PeerInfo struct fields change.
    ///
    /// Returns an instance of `PeerBook` from the given storage object.
    ///
    /// This function fetches a serialized peer book from the given storage object,
    /// and attempts to deserialize it as an instance of `PeerBook`.
    ///
    /// If the peer book does not exist in storage or fails to deserialize properly,
    /// returns a `NetworkError`.
    ///
    #[inline]
    pub fn load<T: Transaction, P: LoadableMerkleParameters>(storage: &Ledger<T, P>) -> Result<Self, NetworkError> {
        // Fetch the peer book from storage.
        match storage.get_peer_book() {
            // Attempt to deserialize it as a peer book.
            Ok(serialized_peer_book) => Ok(bincode::deserialize(&serialized_peer_book)?),
            _ => Err(NetworkError::PeerBookFailedToLoad),
        }
    }

    /// Returns the local address of this node.
    #[inline]
    pub fn local_address(&self) -> SocketAddr {
        self.local_address
    }

    /// Updates the local address of this node.
    #[inline]
    pub fn set_local_address(&mut self, address: SocketAddr) {
        // Check that the node does not maintain a connection to itself.
        self.forget_peer(address);
        // Update the local address to the given address.
        self.local_address = address;
    }

    /// Returns `true` if a given address is a connected peer in the `PeerBook`.
    #[inline]
    pub fn is_connected(&self, address: &SocketAddr) -> bool {
        self.connected_peers.contains_key(address)
    }

    /// Returns `true` if a given address is a disconnected peer in the `PeerBook`.
    #[inline]
    pub fn is_disconnected(&self, address: &SocketAddr) -> bool {
        self.disconnected_peers.contains_key(address)
    }

    /// Returns the number of connected peers.
    #[inline]
    pub fn num_connected(&self) -> u16 {
        self.connected_peers.len() as u16
    }

    /// Returns the number of disconnected peers.
    #[inline]
    pub fn num_disconnected(&self) -> u16 {
        self.disconnected_peers.len() as u16
    }

    /// Returns a reference to the connected peers in this peer book.
    #[inline]
    pub fn get_all_connected(&self) -> &HashMap<SocketAddr, PeerInfo> {
        &self.connected_peers
    }

    /// Returns a reference to the disconnected peers in this peer book.
    #[inline]
    pub fn get_all_disconnected(&self) -> &HashMap<SocketAddr, PeerInfo> {
        &self.disconnected_peers
    }

    /// Returns a reference to the peer info of a given address
    /// if it exists in the `PeerBook`.
    #[inline]
    pub fn get_peer_info(&mut self, address: &SocketAddr) -> Option<&PeerInfo> {
        // Issue an error message if the peer is both connected and disconnected
        // in the internal state of the `PeerBook`.
        let is_connected = self.is_connected(address);
        let is_disconnected = self.is_disconnected(address);
        if is_connected && is_disconnected {
            error!("The peer info of {} is corrupted in the peer book.", address);
        }
        // Fetch and return the peer info of the given address if it exists.
        if is_connected {
            self.connected_peers.get(address)
        } else if is_disconnected {
            self.disconnected_peers.get(address)
        } else {
            None
        }
    }

    /// Returns a mutable reference to the peer info of a given address
    /// if it exists in the `PeerBook`.
    #[inline]
    pub fn get_peer_info_mut(&mut self, address: &SocketAddr) -> Option<&mut PeerInfo> {
        // Issue an error message if the peer is both connected and disconnected
        // in the internal state of the `PeerBook`.
        let is_connected = self.is_connected(address);
        let is_disconnected = self.is_disconnected(address);
        if is_connected && is_disconnected {
            error!("The peer info of {} is corrupted in the peer book.", address);
        }
        // Fetch and return the peer info of the given address if it exists.
        if is_connected {
            self.connected_peers.get_mut(address)
        } else if is_disconnected {
            self.disconnected_peers.get_mut(address)
        } else {
            None
        }
    }

    /// Add the given address to the connected peers in the `PeerBook`.
    /// Returns `true` on success. Otherwise, returns `false`.
    #[inline]
    pub fn connected_peer(&mut self, address: &SocketAddr) -> bool {
        // Check that the address is not the local address of this node.
        if self.local_address() == *address {
            return false;
        }
        // Remove the address from the disconnected peers, if it exists.
        let mut peer_info = match self.disconnected_peers.remove(&address) {
            // Case 1: A previously-known peer.
            Some(peer_info) => peer_info,
            // Case 2: A newly-discovered peer.
            _ => PeerInfo::new(*address),
        };
        // Update the peer info to connected.
        peer_info.set_connected();
        // Add the address into the connected peers.
        let success = self.connected_peers.insert(*address, peer_info).is_none();
        // Only increment the connected_peer metric if the peer was not connected already.
        connected_peers_inc!(success)
    }

    /// Remove the given address from the connected peers in this peer book.
    /// Returns `true` on success. Otherwise, returns `false`.
    #[inline]
    pub fn disconnected_peer(&mut self, address: &SocketAddr) -> bool {
        // Check that the address is not the local address of this node.
        if self.local_address() == *address {
            return false;
        }
        // Remove the address from the connected peers, if it exists.
        if let Some(mut peer_info) = self.connected_peers.remove(&address) {
            // Case 1: A presently-connected peer.

            // Update the peer info to disconnected.
            peer_info.set_disconnected();
            // Add the address into the disconnected peers.
            let success = self.disconnected_peers.insert(*address, peer_info).is_none();
            // Only decrement the connected_peer metric if the peer was not disconnected already.
            connected_peers_dec!(success)
        } else {
            // Case 2: A newly-discovered peer.

            // Add the address into the disconnected peers.
            self.found_peer(address);
            false
        }
    }

    /// Add the given address to the disconnected peers in this peer book.
    /// Returns `true` on success. Otherwise, returns `false`.
    #[inline]
    pub fn found_peer(&mut self, address: &SocketAddr) -> bool {
        if self.local_address() == *address {
            // Case 1: The peer is our node.
            return false;
        } else if self.is_connected(address) || self.is_disconnected(address) {
            // Case 2: The peer is already-known.

            // Update the last seen datetime as we have just seen the peer.
            if let Some(ref mut peer_info) = self.get_peer_info_mut(address) {
                peer_info.set_last_seen();
            }
            false
        } else {
            // Case 3: The peer is newly-discovered.
            self.disconnected_peers
                .insert(*address, PeerInfo::new(*address))
                .is_none()
        }
    }

    ///
    /// Removes the given address from the `PeerBook`.
    ///
    /// If the given address is a currently connected peer in the `PeerBook`,
    /// the connected peer will be disconnected from this node.
    ///
    #[inline]
    pub fn forget_peer(&mut self, address: SocketAddr) {
        // Remove the address from the connected peers, if it exists.
        if let Some(mut peer_info) = self.connected_peers.remove(&address) {
            // Set the peer info of this address to disconnected.
            peer_info.set_disconnected();
            // Decrement the connected_peer metric as the peer was not yet disconnected.
            connected_peers_dec!()
        }

        // Remove the address from the disconnected peers in case it exists.
        self.disconnected_peers.remove(&address);
    }
}
