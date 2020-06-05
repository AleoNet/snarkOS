use crate::{
    message::types::{Ping, Pong},
    Channel,
    PingProtocol,
    PingState,
};
use snarkos_errors::network::PingProtocolError;

use std::{collections::HashMap, net::SocketAddr, sync::Arc};

/// Stores connected peers and the latest state of a ping/pong protocol.
#[derive(Clone, Debug)]
pub struct Pings {
    addresses: HashMap<SocketAddr, PingProtocol>,
}

impl Pings {
    /// Construct new store of connected peer `Pings`.
    pub fn new() -> Self {
        Self {
            addresses: HashMap::default(),
        }
    }

    /// Send a ping request to a peer.
    /// Store the result upon success.
    pub async fn send_ping(&mut self, channel: Arc<Channel>) -> Result<(), PingProtocolError> {
        self.addresses
            .insert(channel.address, PingProtocol::send(channel).await?);
        Ok(())
    }

    /// Send a pong response to a ping request.
    pub async fn send_pong(message: Ping, channel: Arc<Channel>) -> Result<(), PingProtocolError> {
        PingProtocol::receive(message, channel).await
    }

    /// Accept a pong response.
    pub async fn accept_pong(&mut self, peer_address: SocketAddr, message: Pong) -> Result<(), PingProtocolError> {
        match self.addresses.get_mut(&peer_address) {
            Some(stored_ping) => stored_ping.accept(message).await,
            None => Err(PingProtocolError::PingProtocolMissing(peer_address)),
        }
    }

    /// Returns ping state for current peer.
    pub fn get_state(&self, address: SocketAddr) -> Option<PingState> {
        match self.addresses.get(&address) {
            Some(stored_ping) => Some(stored_ping.get_state()),
            None => None,
        }
    }
}
