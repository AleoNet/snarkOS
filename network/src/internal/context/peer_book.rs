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

use snarkos_errors::network::ServerError;
use snarkos_metrics::Metrics;
use snarkos_models::{algorithms::LoadableMerkleParameters, objects::Transaction};
use snarkos_storage::Ledger;

use chrono::{DateTime, Utc};
use serde::Serialize;
use std::{collections::HashMap, net::SocketAddr};

pub enum PeerStatus {
    NeverConnected,
    Connected,
    Disconnected,
    Unknown,
}

/// Stores relevant metadata about a peer.
#[derive(Clone, Debug, Serialize)]
pub struct PeerInfo {
    /// The IP address of the peer.
    address: SocketAddr,
    /// The number of times we connected to the peer.
    connected_count: i64,
    /// The number of times we disconnected from the peer.
    disconnected_count: i64,
    /// The last datetime we connected to the peer.
    last_connected: DateTime<Utc>,
    /// The last datetime we disconnected from the peer.
    last_disconnected: DateTime<Utc>,
    /// The last datetime we saw the peer.
    last_seen: DateTime<Utc>,
    /// The first datetime we saw the peer.
    first_seen: DateTime<Utc>,
}

impl PeerInfo {
    /// Creates a new instance of `PeerInfo`.
    #[inline]
    pub fn new(address: SocketAddr) -> Self {
        let now = Utc::now();
        Self {
            address,
            connected_count: 0,
            disconnected_count: 0,
            last_connected: now.clone(),
            last_disconnected: now.clone(),
            last_seen: now.clone(),
            first_seen: now,
        }
    }

    /// Updates the last seen datetime of the peer.
    #[inline]
    pub fn set_last_seen(&mut self) {
        // Only update the last seen datetime if the peer is connected.
        match self.status() {
            PeerStatus::Connected => {
                self.last_seen = Utc::now();
            }
            PeerStatus::Disconnected | PeerStatus::NeverConnected | PeerStatus::Unknown => {
                error!(
                    "Attempted to set the last seen datetime for a disconnected peer ({})",
                    self.address
                );
            }
        }
    }

    /// Updates the connected metrics of the peer.
    #[inline]
    pub fn set_connected(&mut self) {
        // Only update the last connected metrics if the peer is new or currently disconnected.
        match self.status() {
            PeerStatus::NeverConnected | PeerStatus::Disconnected => {
                let now = Utc::now();
                self.last_seen = now.clone();
                self.last_connected = now;
                self.connected_count += 1;
            }
            PeerStatus::Connected | PeerStatus::Unknown => {
                error!(
                    "Attempted to set a connected peer to connected again ({})",
                    self.address
                );
            }
        }
    }

    /// Updates the disconnected metrics of the peer.
    #[inline]
    pub fn set_disconnected(&mut self) {
        // Only update the last disconnected metrics if the peer is connected.
        match self.status() {
            PeerStatus::Connected => {
                let now = Utc::now();
                self.last_seen = now.clone();
                self.last_disconnected = now;
                self.disconnected_count += 1;
            }
            PeerStatus::Disconnected | PeerStatus::NeverConnected | PeerStatus::Unknown => {
                error!(
                    "Attempted to set a disconnected peer to disconnected again ({})",
                    self.address
                );
            }
        }
    }

    /// Returns the status of the peer connection based on the datetime
    /// that the node connected and disconnected to the peer.
    #[inline]
    pub fn status(&self) -> PeerStatus {
        // If `first_seen`, `last_connected`, and `last_disconnected` are all equal,
        // it means we are unnecessarily refreshing.
        if (self.first_seen == self.last_connected) && (self.last_connected == self.last_disconnected) {
            PeerStatus::NeverConnected
        }
        // If `first_seen` is earlier than `last_connected`,
        // `last_connected` is later than `last_disconnected`,
        // `last_connected` is later than the current time,
        // and `last_connected` is close to the current time,
        // it means we are connected to this peer.
        else if (self.first_seen < self.last_connected) && (self.last_connected > self.last_disconnected) {
            PeerStatus::Connected
        }
        // If `first_seen` is earlier than `last_connected`,
        // `last_connected` is earlier than `last_disconnected`,
        // `last_disconnected` is later than the current time,
        // and `last_disconnected` is close to the current time,
        // it means we are disconnected from this peer.
        else if (self.first_seen < self.last_connected) && (self.last_connected < self.last_disconnected) {
            PeerStatus::Disconnected
        }
        // If the above cases did not address our needs,
        // it is likely that either:
        //
        // 1. There is a bug in how the `PeerBook` is refreshing `PeerInfo`.
        // 2. A malicious peer or system is fudging `PeerInfo` state.
        // 3. The `PeerBook` is incorrectly triggering a refresh for an unknown reason.
        else {
            error!("The peer info of {} is refreshing incorrectly or corrupt", self.address);
            PeerStatus::Unknown
        }
    }

    /// Returns the IP address of the peer.
    #[inline]
    pub fn address(&self) -> &SocketAddr {
        &self.address
    }

    /// Returns the number of times we connected with the peer.
    #[inline]
    pub fn connected_count(&self) -> i64 {
        self.connected_count
    }

    /// Returns the number of times we disconnected with the peer.
    #[inline]
    pub fn disconnected_count(&self) -> i64 {
        self.disconnected_count
    }

    /// Returns the last datetime we connected with the peer.
    #[inline]
    pub fn last_connected(&self) -> &DateTime<Utc> {
        &self.last_connected
    }

    /// Returns the last datetime we disconnected with the peer.
    #[inline]
    pub fn last_disconnected(&self) -> &DateTime<Utc> {
        &self.last_disconnected
    }

    /// Returns the last datetime we saw the peer.
    #[inline]
    pub fn last_seen(&self) -> &DateTime<Utc> {
        &self.last_seen
    }

    /// Returns the first datetime we saw the peer.
    #[inline]
    pub fn first_seen(&self) -> &DateTime<Utc> {
        &self.first_seen
    }
}

/// Stores the existence of a peer and the date they were last seen.
#[derive(Debug)]
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
    pub fn num_connected(&self) -> u16 {
        self.connected_peers.len() as u16
    }

    /// Returns the number of disconnected peers.
    pub fn num_disconnected(&self) -> u16 {
        self.disconnected_peers.len() as u16
    }

    /// Returns a reference to the connected peers in the `PeerBook`.
    #[inline]
    pub fn get_connected(&self) -> &HashMap<SocketAddr, PeerInfo> {
        &self.connected_peers
    }

    /// Returns a reference to the disconnected peers in the `PeerBook`.
    #[inline]
    pub fn get_disconnected(&self) -> &HashMap<SocketAddr, PeerInfo> {
        &self.disconnected_peers
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
    pub fn found_peer(&mut self, address: &SocketAddr) -> bool {
        if self.is_connected(address) || self.is_disconnected(address) {
            // Case 1: The peer is already-known.
            // self.set_last_seen();
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
    pub fn forget_peer(&mut self, address: SocketAddr) -> bool {
        if !self.connected_peers.contains_key(&address) {
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
    pub fn store<T: Transaction, P: LoadableMerkleParameters>(
        &self,
        storage: &Ledger<T, P>,
    ) -> Result<(), ServerError> {
        Ok(storage.store_to_peer_book(bincode::serialize(&(self.get_connected().clone()))?)?)
    }
}
