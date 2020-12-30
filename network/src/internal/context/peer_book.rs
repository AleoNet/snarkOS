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

use crate::internal::address_book::AddressBook;
use snarkos_errors::network::ServerError;
use snarkos_storage::Ledger;
use snarkvm_models::{algorithms::LoadableMerkleParameters, objects::Transaction};

use chrono::{DateTime, Utc};
use std::{collections::HashMap, net::SocketAddr};

/// Stores connected, disconnected, and known peers.
#[derive(Clone, Debug, Eq, PartialEq, Default)]
pub struct PeerBook {
    /// Connected peers
    connected: AddressBook,

    /// Disconnected peers
    disconnected: AddressBook,

    /// Gossiped but unconnected peers
    gossiped: AddressBook,
}

impl PeerBook {
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns copy of connected peers.
    pub fn get_connected(&self) -> HashMap<SocketAddr, DateTime<Utc>> {
        self.connected.get_addresses()
    }

    /// Returns copy of gossiped peers.
    pub fn get_gossiped(&self) -> HashMap<SocketAddr, DateTime<Utc>> {
        self.gossiped.get_addresses()
    }

    /// Returns true if address is a connected peer.
    pub fn connected_contains(&self, address: &SocketAddr) -> bool {
        self.connected.contains(address)
    }

    /// Returns true if address is a disconnected peer.
    pub fn disconnected_contains(&self, address: &SocketAddr) -> bool {
        self.disconnected.contains(address)
    }

    /// Returns true if address is a gossiped peer.
    pub fn gossiped_contains(&self, address: &SocketAddr) -> bool {
        self.gossiped.contains(address)
    }

    /// Move a peer from disconnected/gossiped to connected peers.
    pub fn update_connected(&mut self, address: SocketAddr, date: DateTime<Utc>) -> bool {
        self.disconnected.remove(&address);
        self.gossiped.remove(&address);
        self.connected.insert_or_update(address, date)
    }

    /// Move a peer from connected/disconnected to gossiped peers.
    pub fn update_gossiped(&mut self, address: SocketAddr, date: DateTime<Utc>) -> bool {
        self.connected.remove(&address);
        self.disconnected.remove(&address);
        self.gossiped.insert_or_update(address, date)
    }

    /// Move a peer from connected peers to disconnected peers.
    pub fn disconnect_peer(&mut self, address: SocketAddr) -> bool {
        self.connected.remove(&address);
        self.gossiped.remove(&address);
        self.disconnected.insert_or_update(address, Utc::now())
    }

    /// Forget a peer.
    pub fn forget_peer(&mut self, address: SocketAddr) {
        self.connected.remove(&address);
        self.gossiped.remove(&address);
        self.disconnected.remove(&address);
    }

    /// Remove_gossiped peer
    pub fn remove_gossiped(&mut self, address: SocketAddr) -> bool {
        self.gossiped.remove(&address).is_some()
    }

    /// Returns the number of connected peers.
    pub fn connected_total(&self) -> u16 {
        self.connected.length()
    }

    /// Writes connected peers to storage.
    pub fn store<T: Transaction, P: LoadableMerkleParameters>(
        &self,
        storage: &Ledger<T, P>,
    ) -> Result<(), ServerError> {
        Ok(storage.store_to_peer_book(bincode::serialize(&self.get_connected())?)?)
    }
}
