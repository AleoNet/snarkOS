use crate::{
    message::types::{GetPeers, Verack, Version},
    Channel,
    Handshake,
    HandshakeState,
    Message,
};
use snarkos_errors::network::HandshakeError;

use std::{collections::HashMap, net::SocketAddr};
use tokio::net::TcpStream;

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
        version: u64,
        height: u32,
        address_sender: SocketAddr,
        address_receiver: SocketAddr,
    ) -> Result<Handshake, HandshakeError> {
        let handshake = Handshake::send_new(version, height, address_sender, address_receiver).await?;

        self.addresses.insert(address_receiver, handshake.clone());
        info!("Request handshake with: {:?}", address_receiver);

        Ok(handshake)
    }

    /// Receive a handshake request from a new peer. Send response and store the result upon success.
    pub async fn receive_request_new(
        &mut self,
        version: u64,
        height: u32,
        address_sender: SocketAddr,
        _address_receiver: SocketAddr,
        reader: TcpStream,
    ) -> Result<Handshake, HandshakeError> {
        let channel = Channel::new_read_only(reader)?;

        // Read the first message or error
        let (name, bytes) = channel.read().await?;

        if Version::name() == name {
            let peer_message = Version::deserialize(bytes)?;
            let peer_address = peer_message.address_sender;

            let handshake = Handshake::receive_new(version, height, channel, peer_message, address_sender).await?;

            self.addresses.insert(peer_address, handshake.clone());

            Ok(handshake)
        } else if Verack::name() == name {
            let peer_message = Verack::deserialize(bytes)?;
            let peer_address = peer_message.address_sender;

            match self.get_mut(&peer_address) {
                Some(handshake) => {
                    handshake.accept(peer_message).await?;
                    handshake.update_reader(channel);
                    info!("New handshake with: {:?}", peer_address);

                    // Get our new peer's peer_list
                    handshake.channel.write(&GetPeers).await?;

                    Ok(handshake.clone())
                }
                None => Err(HandshakeError::HandshakeMissing(peer_address)),
            }
        } else {
            Err(HandshakeError::InvalidMessage(name.to_string()))
        }
    }

    /// Receive a handshake request from an existing peer. Send response.
    pub async fn receive_request(
        &mut self,
        message: Version,
        address_receiver: SocketAddr,
    ) -> Result<(), HandshakeError> {
        match self.get_mut(&address_receiver) {
            Some(stored_handshake) => {
                stored_handshake.receive(message).await?;
                Ok(())
            }
            None => Err(HandshakeError::HandshakeMissing(address_receiver)),
        }
    }

    /// Accept a handshake response from a peer that has received our handshake request.
    /// Update the stored handshake status upon success.
    pub async fn accept_response(&mut self, address: SocketAddr, message: Verack) -> Result<(), HandshakeError> {
        match self.get_mut(&address) {
            Some(stored_handshake) => {
                info!("New handshake with: {:?}", address);

                stored_handshake.accept(message).await
            }
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

    fn get_mut(&mut self, address: &SocketAddr) -> Option<&mut Handshake> {
        self.addresses.get_mut(&address)
    }

    pub fn remove(&mut self, address: &SocketAddr) -> Option<Handshake> {
        self.addresses.remove(address)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{test_data::random_socket_address, Message};
    use serial_test::serial;
    use tokio::net::TcpListener;

    #[tokio::test]
    #[serial]
    async fn test_handshakes() {
        let server_address = random_socket_address();
        let peer_address = random_socket_address();

        // 1. Bind to peer address
        let mut peer_listener = TcpListener::bind(peer_address).await.unwrap();

        tokio::spawn(async move {
            let mut server_listener = TcpListener::bind(server_address).await.unwrap();

            // 2. Server sends server_handshake request

            let mut server_handshakes = Handshakes::new();

            let mut server_hand = server_handshakes
                .send_request(1u64, 0u32, server_address, peer_address)
                .await
                .unwrap();

            // 5. Check server handshake state

            let (reader, _socket) = server_listener.accept().await.unwrap();
            let read_channel = Channel::new_read_only(reader).unwrap();
            server_hand.update_reader(read_channel);

            assert_eq!(
                HandshakeState::Waiting,
                server_handshakes.get_state(peer_address).unwrap()
            );

            // 6. Server accepts server_handshake response

            let (_name, bytes) = server_hand.channel.read().await.unwrap();
            let message = Verack::deserialize(bytes).unwrap();

            server_handshakes.accept_response(peer_address, message).await.unwrap();

            assert_eq!(
                HandshakeState::Accepted,
                server_handshakes.get_state(peer_address).unwrap()
            );

            // 7. Server receives peer_handshake request

            let (_name, bytes) = server_hand.channel.read().await.unwrap();
            let message = Version::deserialize(bytes).unwrap();

            // 8. Server sends peer_handshake response

            server_handshakes.receive_request(message, peer_address).await.unwrap();
        });

        // 3. Peer accepts Server connection

        let (reader, _socket) = peer_listener.accept().await.unwrap();

        // 4. Peer sends server_handshake response, peer_handshake request

        let mut peer_handshakes = Handshakes::new();
        let peer_hand = peer_handshakes
            .receive_request_new(1u64, 0u32, peer_address, server_address, reader)
            .await
            .unwrap();

        assert_eq!(
            HandshakeState::Waiting,
            peer_handshakes.get_state(server_address).unwrap()
        );

        // 9. Server accepts server_handshake response

        let (_name, bytes) = peer_hand.channel.read().await.unwrap();
        let message = Verack::deserialize(bytes).unwrap();

        peer_handshakes.accept_response(server_address, message).await.unwrap();

        assert_eq!(
            HandshakeState::Accepted,
            peer_handshakes.get_state(server_address).unwrap()
        )
    }
}
