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

use chrono::{DateTime, Utc};
use std::{collections::HashMap, net::SocketAddr, ops::Deref};

/// Stores the datetime of the last updated time.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LastUpdated(pub DateTime<Utc>);

impl Deref for LastUpdated {
    type Target = DateTime<Utc>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Store the datetime of the first seen instance of the peer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FirstSeen(pub DateTime<Utc>);

impl Deref for FirstSeen {
    type Target = DateTime<Utc>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub enum Trustworthiness {
    Gossiped,
    Connected,
}

/// Stores relevant metadata about a peer.
#[derive(Debug)]
pub struct PeerInfo {
    /// The IP address of the peer.
    address: SocketAddr,
    /// g
    trust: PeerRelationship,
    /// The last datetime we connected with the peer.
    last_updated: LastUpdated,
    /// The first datetime we discovered the peer.
    first_seen: FirstSeen,
}

impl PeerInfo {
    /// Returns the IP address of the peer.
    fn address(self) -> SocketAddr {
        self.address
    }

    /// Returns the last datetime we connected with the peer.
    fn last_updated(self) -> DateTime<Utc> {
        *self.last_updated
    }

    /// Returns the first datetime we discovered the peer.
    fn first_seen(self) -> DateTime<Utc> {
        *self.first_seen
    }
}

/// Stores the existence of a peer and the date they were last seen.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct AddressBook {
    /// A mapping of connected peers.
    connected_peers: HashMap<SocketAddr, PeerInfo>,
    /// A mapping of disconnected peers.
    disconnected_peers: HashMap<SocketAddr, PeerInfo>,
}

impl AddressBook {
    /// Construct a new `PeerBook`.
    pub fn new() -> Self {
        AddressBook(HashMap::default())
    }

    /// Returns a reference to the connected peers in the `PeerBook`.
    pub fn get_connected(&self) -> &HashMap<SocketAddr, PeerInfo> {
        &self.connected_peers
    }

    /// Returns a reference to the disconnected peers in the `PeerBook`.
    pub fn get_disconnected(&self) -> &HashMap<SocketAddr, PeerInfo> {
        self.disconnected_peers.get_addresses()
    }

    /// Returns `true` if a given address is a connected peer in the `PeerBook`.
    pub fn is_connected(&self, address: &SocketAddr) -> bool {
        self.connected_peers.contains_key(address)
    }

    /// Insert or update a new date for an address.
    /// Returns `true` if the address is new and inserted.
    /// Returns `false` if the address already exists.
    ///
    /// If the address already exists in the address book,
    /// the datetime will be updated to reflect the latest datetime.
    pub fn insert_or_update(&mut self, address: SocketAddr, date: DateTime<Utc>) -> bool {
        match self.0.get(&address) {
            Some(stored_date) => {
                if stored_date < &date {
                    self.0.insert(address, date);
                }
                false
            }
            None => self.0.insert(address, date).is_none(),
        }
    }

    /// Checks if a given address exists in the `PeerBook`.
    /// Returns `true` if it exists. Otherwise, returns `false`.
    pub fn contains(&self, address: &SocketAddr) -> bool {
        self.0.contains_key(address)
    }

    /// Removes a given address from the `PeerBook`.
    /// Returns `true` if a given address existed and was removed.
    /// Otherwise, returns `false`.
    pub fn remove(&mut self, address: &SocketAddr) -> bool {
        // `HashMap::remove` returns `Some(_)`
        // if it successfully removed a (key, value) pair.
        self.0.remove(address).is_some()
    }

    /// Returns the number of peers.
    pub fn length(&self) -> u16 {
        self.0.len() as u16
    }

    /// Returns copy of addresses
    pub fn get_addresses(&self) -> HashMap<SocketAddr, DateTime<Utc>> {
        self.0.clone()
    }
}
