use crate::{
    message::types::{Verack, Version},
    Channel,
    Handshake,
    HandshakeState,
};
use snarkos_errors::network::HandshakeError;
use std::{collections::HashMap, net::SocketAddr, sync::Arc};

#[derive(Clone, Debug)]
pub struct Handshakes {
    pub addresses: HashMap<SocketAddr, Handshake>,
}

impl Handshakes {
    pub fn new() -> Self {
        Self {
            addresses: HashMap::default(),
        }
    }

    /// Send a handshake request to a peer. Store the result upon success.
    pub async fn send_request(
        &mut self,
        channel: Arc<Channel>,
        version: u64,
        height: u32,
        address_sender: SocketAddr,
    ) -> Result<(), HandshakeError> {
        self.addresses.insert(
            channel.address,
            Handshake::send_new(channel, version, height, address_sender).await?,
        );
        Ok(())
    }

    /// Send a handshake response to a handshake request from a peer.
    /// If the peer is new, send our own handshake request to the peer, store this result upon success.
    pub async fn send_response_request(
        &mut self,
        message: Version,
        new_peer: bool,
        channel: Arc<Channel>,
        version: u64,
        height: u32,
        address_sender: SocketAddr,
    ) -> Result<(), HandshakeError> {
        let peer_address = channel.address;
        match Handshake::receive_new(message, new_peer, channel, version, height, address_sender).await? {
            Some(handshake) => {
                self.addresses.insert(peer_address, handshake);
                Ok(())
            }
            None => {
                if new_peer {
                    return Err(HandshakeError::PeerDisconnect(peer_address));
                }
                Ok(())
            }
        }
    }

    /// Accept a handshake response from a peer that has received our handshake request.
    /// Update the stored handshake status upon success.
    pub async fn accept_response(&mut self, address: SocketAddr, message: Verack) -> Result<(), HandshakeError> {
        match self.addresses.get_mut(&address) {
            Some(stored_handshake) => stored_handshake.accept(message).await,
            None => Err(HandshakeError::HandshakeMissing(address)),
        }
    }

    /// Gets the state of a handshake with a peer
    pub fn get_state(&self, address: SocketAddr) -> Option<HandshakeState> {
        match self.addresses.get(&address) {
            Some(stored_handshake) => Some(stored_handshake.state.clone()),
            None => None,
        }
    }

    pub fn insert(&mut self, address: SocketAddr, handshake: Handshake) -> Option<Handshake> {
        self.addresses.insert(address, handshake)
    }

    pub fn remove(&mut self, address: &SocketAddr) -> Option<Handshake> {
        self.addresses.remove(address)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        test_data::{get_next_channel, random_socket_address},
        Message,
    };
    use serial_test::serial;
    use tokio::net::TcpListener;

    #[tokio::test]
    #[serial]
    async fn test_handshakes() {
        let version = 1u64;
        let height = 0u32;
        let server_address = random_socket_address();
        let peer_address = random_socket_address();

        // 1. Bind to server address

        let mut server_listener = TcpListener::bind(server_address).await.unwrap();

        tokio::spawn(async move {
            // 2. Peer connects to Server address

            let channel = Arc::new(Channel::connect(server_address).await.unwrap());

            // 4. Peer sends peer_handshake request

            let mut peer_handshakes = Handshakes::new();

            peer_handshakes
                .send_request(channel.clone(), 1u64, 0u32, peer_address)
                .await
                .unwrap();

            assert_eq!(
                HandshakeState::Waiting,
                peer_handshakes.get_state(channel.address).unwrap()
            );

            // 7. Peer accepts peer_handshake response

            let (_name, bytes) = channel.read().await.unwrap();
            let message = Verack::deserialize(bytes).unwrap();

            peer_handshakes.accept_response(channel.address, message).await.unwrap();

            assert_eq!(
                HandshakeState::Accepted,
                peer_handshakes.get_state(channel.address).unwrap()
            );

            // 8. Peer receives server_handshake request

            let (_name, bytes) = channel.read().await.unwrap();
            let message = Version::deserialize(bytes).unwrap();

            // 9. Peer sends server_handshake response

            peer_handshakes
                .send_response_request(message, false, channel.clone(), 1u64, 0u32, peer_address)
                .await
                .unwrap();
        });

        // 3. Server accepts Peer connection

        let channel = get_next_channel(&mut server_listener).await;

        // 5. Server receives peer_handshake request

        let (_name, bytes) = channel.read().await.unwrap();
        let message = Version::deserialize(bytes).unwrap();

        // 6. Server sends peer_handshake request, server_handshake response

        let mut server_handshakes = Handshakes::new();
        server_handshakes
            .send_response_request(message, true, channel.clone(), version, height, server_address)
            .await
            .unwrap();

        assert_eq!(
            HandshakeState::Waiting,
            server_handshakes.get_state(channel.address).unwrap()
        );

        // 10. Server accepts server_handshake response

        let (_name, bytes) = channel.read().await.unwrap();
        let message = Verack::deserialize(bytes).unwrap();

        server_handshakes
            .accept_response(channel.address, message)
            .await
            .unwrap();

        assert_eq!(
            HandshakeState::Accepted,
            server_handshakes.get_state(channel.address).unwrap()
        )
    }
}
