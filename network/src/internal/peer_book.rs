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

use crate::internal::PeerInfo;
use snarkos_errors::network::ServerError;
use snarkos_metrics::Metrics;
use snarkos_models::{algorithms::LoadableMerkleParameters, objects::Transaction};
use snarkos_storage::Ledger;

use serde::Serialize;
use std::{collections::HashMap, net::SocketAddr};

/// An data structure for tracking and indexing the history of
/// all connected and disconnected peers to the node.
#[derive(Debug, Serialize)]
pub struct PeerBook {
    /// A mapping of connected peers.
    connected_peers: HashMap<SocketAddr, PeerInfo>,
    /// A mapping of disconnected peers.
    disconnected_peers: HashMap<SocketAddr, PeerInfo>,
}

impl PeerBook {
    /// Construct a new `PeerBook`.
    #[inline]
    pub fn new() -> Self {
        Self {
            connected_peers: HashMap::default(),
            disconnected_peers: HashMap::default(),
        }
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

    /// Returns a reference to the connected peers in the `PeerBook`.
    #[inline]
    pub fn get_all_connected(&self) -> &HashMap<SocketAddr, PeerInfo> {
        &self.connected_peers
    }

    /// Returns a reference to the disconnected peers in the `PeerBook`.
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

    /// Returns `true` if a given address is a connected peer in the `PeerBook`.
    #[inline]
    pub fn is_connected(&self, address: &SocketAddr) -> bool {
        self.connected_peers.contains_key(address)
    }

    /// Returns `true` if a given address is a disconnected peer in the `PeerBook`.
    #[inline]
    pub fn is_disconnected(&self, address: &SocketAddr) -> bool {
        !self.is_connected(address)
    }

    /// Add the given address to the connected peers in the `PeerBook`.
    /// Returns `true` on success. Otherwise, returns `false`.
    #[inline]
    pub fn connected_peer(&mut self, address: &SocketAddr) -> bool {
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

    /// Remove the given address from the connected peers in the `PeerBook`.
    /// Returns `true` on success. Otherwise, returns `false`.
    #[inline]
    pub fn disconnected_peer(&mut self, address: &SocketAddr) -> bool {
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

    /// Add the given address to the disconnected peers in the `PeerBook`.
    /// Returns `true` on success. Otherwise, returns `false`.
    #[inline]
    pub fn found_peer(&mut self, address: &SocketAddr) -> bool {
        if self.is_connected(address) || self.is_disconnected(address) {
            // Case 1: The peer is already-known.

            // Update the last seen datetime as we have just seen the peer.
            if let Some(ref mut peer_info) = self.get_peer_info_mut(address) {
                peer_info.set_last_seen();
            }
            false
        } else {
            // Case 2: The peer is newly-discovered.
            self.disconnected_peers
                .insert(*address, PeerInfo::new(*address))
                .is_none()
        }
    }

    /// Remove the given address from the `PeerBook`.
    /// Returns `true` on success. Otherwise, returns `false`.
    ///
    /// Note that the given address may only be removed from the `PeerBook`
    /// if the peer is not connected to the node.
    #[inline]
    pub fn forget_peer(&mut self, address: SocketAddr) -> bool {
        if !self.is_connected(&address) {
            // We can forget the peer if we are not connected to them.

            // Do not use the result of the `HashMap::remove`.
            // If we know the peer, return `true`.
            // And if we do not know the peer, still return `true`.
            self.disconnected_peers.remove(&address);
            true
        } else {
            // We cannot forget the peer if we are connected to them.
            false
        }
    }

    /// Serializes and writes the `PeerBook` to storage.
    #[inline]
    pub fn store<T: Transaction, P: LoadableMerkleParameters>(
        &self,
        storage: &Ledger<T, P>,
    ) -> Result<(), ServerError> {
        Ok(storage.store_to_peer_book(bincode::serialize(&self.clone())?)?)
    }
}
