use crate::{
    message::types::{Ping, Pong},
    Channel,
    PingProtocol,
    PingState,
};
use snarkos_errors::network::PingProtocolError;
use std::{collections::HashMap, net::SocketAddr, sync::Arc};

#[derive(Clone, Debug)]
pub struct Pings {
    pub addresses: HashMap<SocketAddr, PingProtocol>,
}

impl Pings {
    pub fn new() -> Self {
        Self {
            addresses: HashMap::default(),
        }
    }

    /// Send a ping request to a peer. Store the result upon success.
    pub async fn send_ping(&mut self, channel: Arc<Channel>) -> Result<(), PingProtocolError> {
        self.addresses
            .insert(channel.address, PingProtocol::send(channel).await?);
        Ok(())
    }

    /// Send a pong response to a ping request
    pub async fn send_pong(message: Ping, channel: Arc<Channel>) -> Result<(), PingProtocolError> {
        PingProtocol::receive(message, channel).await
    }

    /// Accept a pong response
    pub async fn accept_pong(&mut self, peer_address: SocketAddr, message: Pong) -> Result<(), PingProtocolError> {
        match self.addresses.get_mut(&peer_address) {
            Some(stored_ping) => stored_ping.accept(message).await,
            None => Err(PingProtocolError::PingProtocolMissing(peer_address)),
        }
    }

    /// Returns ping state for current peer
    pub fn get_state(&self, address: SocketAddr) -> Option<PingState> {
        match self.addresses.get(&address) {
            Some(stored_ping) => Some(stored_ping.state.clone()),
            None => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        test_data::{accept_channel, connect_channel, random_socket_address},
        Message,
        PingState,
    };
    use serial_test::serial;
    use tokio::net::TcpListener;

    #[tokio::test]
    #[serial]
    async fn test_pings() {
        let server_address = random_socket_address();
        let peer_address = random_socket_address();

        // 1. Bind to server address

        let mut server_listener = TcpListener::bind(server_address).await.unwrap();

        let (tx, rx) = tokio::sync::oneshot::channel();
        tokio::spawn(async move {
            let mut peer_listener = TcpListener::bind(peer_address).await.unwrap();

            // 2. Peer connects to server address

            let channel = Arc::new(connect_channel(&mut peer_listener, server_address).await);

            // 4. Peer sends ping request

            let mut pings = Pings::new();

            pings.send_ping(channel.clone()).await.unwrap();

            assert_eq!(PingState::Waiting, pings.get_state(server_address).unwrap());

            // 7. Peer receives pong response

            let (_name, bytes) = channel.read().await.unwrap();
            let message = Pong::deserialize(bytes).unwrap();

            pings.accept_pong(channel.address, message).await.unwrap();

            assert_eq!(PingState::Accepted, pings.get_state(server_address).unwrap());
            tx.send(()).unwrap();
        });

        // 3. Server accepts peer connection

        let channel = Arc::new(accept_channel(&mut server_listener, peer_address).await);

        // 5. Server receives ping request

        let (_name, bytes) = channel.read().await.unwrap();
        let message = Ping::deserialize(bytes).unwrap();

        // 6. Server sends pong response

        Pings::send_pong(message, channel).await.unwrap();
        rx.await.unwrap();
    }
}
