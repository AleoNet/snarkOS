use crate::address_book::AddressBook;
use snarkos_errors::network::ServerError;
use snarkos_storage::BlockStorage;

use chrono::{DateTime, Utc};
use std::{collections::HashMap, net::SocketAddr};

/// Stores connected, disconnected, and known peers.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PeerBook {
    /// Connected peers
    connected: AddressBook,

    /// Disconnected peers
    disconnected: AddressBook,

    /// Gossiped but uncontacted peers
    gossiped: AddressBook,
}

impl PeerBook {
    pub fn new() -> Self {
        Self {
            connected: AddressBook::new(),
            disconnected: AddressBook::new(),
            gossiped: AddressBook::new(),
        }
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
        self.connected.update(address, date)
    }

    /// Move a peer from connected/disconnected to gossiped peers.
    pub fn update_gossiped(&mut self, address: SocketAddr, date: DateTime<Utc>) -> bool {
        self.connected.remove(&address);
        self.disconnected.remove(&address);
        self.gossiped.update(address, date)
    }

    /// Move a peer from connected peers to disconnected peers.
    pub fn disconnect_peer(&mut self, address: SocketAddr) -> bool {
        self.connected.remove(&address);
        self.gossiped.remove(&address);
        self.disconnected.update(address, Utc::now())
    }

    /// Returns the number of connected peers.
    pub fn connected_total(&self) -> u16 {
        self.connected.length()
    }

    /// Writes connected peers to storage.
    pub fn store(&self, storage: &BlockStorage) -> Result<(), ServerError> {
        Ok(storage.store_peer_book(bincode::serialize(&self.get_connected())?)?)
    }
}
