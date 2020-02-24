use crate::address_book::AddressBook;

use std::net::SocketAddr;
//use crate::Handshakes;

/// Log of connected, disconnected, and known peers
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PeerBook {
    //    /// Handshake protocols initiated with remote nodes
    //    pub handshakes: Handshakes,
    /// Connected peers
    pub peers: AddressBook,
    /// Disconnected peers
    pub disconnected: AddressBook,
    /// Gossiped but uncontacted peers
    pub gossiped: AddressBook,
}

impl PeerBook {
    pub fn new() -> Self {
        Self {
            //            handshakes: Handshakes::new(),
            peers: AddressBook::new(),
            disconnected: AddressBook::new(),
            gossiped: AddressBook::new(),
        }
    }

    // Temporary contains helpers while we use primitive Vec for address storage

    pub fn peer_contains(&self, socket_addr: &SocketAddr) -> bool {
        self.peers.addresses.contains_key(socket_addr)
    }

    pub fn disconnected_contains(&self, socket_addr: &SocketAddr) -> bool {
        self.disconnected.addresses.contains_key(socket_addr)
    }

    pub fn gossiped_contains(&self, socket_addr: &SocketAddr) -> bool {
        self.gossiped.addresses.contains_key(socket_addr)
    }

    pub fn disconnect_peer(&mut self, socket_addr: &SocketAddr) -> bool {
        match self.peers.remove(&socket_addr) {
            Some(last_seen) => {
                warn!("Disconnected from peer {:?}", socket_addr);
                self.disconnected.update(socket_addr.clone(), last_seen.clone())
            }
            None => false,
        }
    }
}
