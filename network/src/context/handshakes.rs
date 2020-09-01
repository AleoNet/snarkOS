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

use crate::{
    message_types::{GetPeers, Verack, Version},
    Channel,
    Handshake,
    HandshakeState,
    Message,
};
use snarkos_errors::network::HandshakeError;

use std::{collections::HashMap, net::SocketAddr};
use tokio::net::TcpStream;

/// Stores the address and latest state of peers we are handshaking with.
#[derive(Clone, Debug)]
pub struct Handshakes {
    addresses: HashMap<SocketAddr, Handshake>,
}

impl Handshakes {
    /// Construct a new store of connected peer `Handshakes`.
    pub fn new() -> Self {
        Self {
            addresses: HashMap::default(),
        }
    }

    /// Create a new handshake with a peer and send a handshake request to them.
    /// If the request is sent successfully, the handshake is stored and returned.
    pub async fn send_request(
        &mut self,
        version: u64,
        height: u32,
        address_sender: SocketAddr,
        address_receiver: SocketAddr,
    ) -> Result<(), HandshakeError> {
        let handshake = Handshake::send_new(version, height, address_sender, address_receiver).await?;

        self.addresses.insert(address_receiver, handshake);
        info!("Request handshake with: {:?}", address_receiver);

        Ok(())
    }

    /// Receive the first message upon accepting a peer connection.
    /// If the message is a Version:
    ///     1. Create a new handshake.
    ///     2. Send a handshake response.
    ///     3. If the response is sent successfully, store the handshake.
    ///     4. Return the handshake, your address as seen by sender, and the version message.
    /// If the message is a Verack:
    ///     1. Get the existing handshake.
    ///     2. Mark the handshake as accepted.
    ///     3. Send a request for peers.
    ///     4. Return the accepted handshake and your address as seen by sender.
    pub async fn receive_any(
        &mut self,
        version: u64,
        height: u32,
        local_address: SocketAddr,
        peer_address: SocketAddr,
        reader: TcpStream,
    ) -> Result<(Handshake, SocketAddr, Option<Version>), HandshakeError> {
        let channel = Channel::new_read_only(reader)?;

        // Read the first message or error
        let (name, bytes) = channel.read().await?;

        if Version::name() == name {
            let peer_message = Version::deserialize(bytes)?;

            let receiver = peer_message.address_receiver;

            // Peer address and specified port from the version message
            let peer_address = SocketAddr::new(peer_address.ip(), peer_message.address_sender.port());

            let handshake = Handshake::receive_new(
                version,
                height,
                channel,
                peer_message.clone(),
                local_address,
                peer_address,
            )
            .await?;

            self.addresses.insert(peer_address, handshake.clone());

            Ok((handshake, receiver, Some(peer_message)))
        } else if Verack::name() == name {
            let peer_message = Verack::deserialize(bytes)?;
            let peer_address = peer_message.address_sender;
            let receiver = peer_message.address_receiver;

            match self.get_mut(&peer_address) {
                Some(handshake) => {
                    handshake.accept(peer_message).await?;
                    handshake.update_reader(channel);
                    info!("New handshake with: {:?}", peer_address);

                    // Get our new peer's peer_list
                    handshake.channel.write(&GetPeers).await?;

                    Ok((handshake.clone(), receiver, None))
                }
                None => Err(HandshakeError::HandshakeMissing(peer_address)),
            }
        } else {
            Err(HandshakeError::InvalidMessage(name.to_string()))
        }
    }

    /// Receive a handshake request from a connected peer.
    /// Update the handshake channel address if needed.
    /// Send a handshake response.
    pub async fn receive_request(
        &mut self,
        message: Version,
        address_receiver: SocketAddr,
    ) -> Result<(), HandshakeError> {
        match self.get_mut(&address_receiver) {
            Some(stored_handshake) => {
                stored_handshake.update_address(address_receiver);
                stored_handshake.receive(message).await?;

                Ok(())
            }
            None => Err(HandshakeError::HandshakeMissing(address_receiver)),
        }
    }

    /// Accept a handshake response from a connected peer.
    pub async fn accept_response(&mut self, address: SocketAddr, message: Verack) -> Result<(), HandshakeError> {
        match self.get_mut(&address) {
            Some(stored_handshake) => {
                info!("New handshake with: {:?}", address);

                stored_handshake.accept(message).await
            }
            None => Err(HandshakeError::HandshakeMissing(address)),
        }
    }

    /// Returns the state of the handshake at a peer address.
    pub fn get_state(&self, address: SocketAddr) -> Option<HandshakeState> {
        match self.addresses.get(&address) {
            Some(stored_handshake) => Some(stored_handshake.get_state()),
            None => None,
        }
    }

    /// Returns a mutable reference to the handshake at a peer address.
    fn get_mut(&mut self, address: &SocketAddr) -> Option<&mut Handshake> {
        self.addresses.get_mut(&address)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Message;
    use snarkos_testing::network::random_socket_address;

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

            server_handshakes
                .send_request(1u64, 0u32, server_address, peer_address)
                .await
                .unwrap();

            // 5. Check server handshake state

            let (reader, _socket) = server_listener.accept().await.unwrap();
            let read_channel = Channel::new_read_only(reader).unwrap();

            assert_eq!(
                HandshakeState::Waiting,
                server_handshakes.get_state(peer_address).unwrap()
            );

            // 6. Server accepts server_handshake response

            let (_name, bytes) = read_channel.read().await.unwrap();
            let message = Verack::deserialize(bytes).unwrap();

            server_handshakes.accept_response(peer_address, message).await.unwrap();

            assert_eq!(
                HandshakeState::Accepted,
                server_handshakes.get_state(peer_address).unwrap()
            );

            // 7. Server receives peer_handshake request

            let (_name, bytes) = read_channel.read().await.unwrap();
            let message = Version::deserialize(bytes).unwrap();

            // 8. Server sends peer_handshake response

            server_handshakes.receive_request(message, peer_address).await.unwrap();
        });

        // 3. Peer accepts Server connection

        let (reader, _socket) = peer_listener.accept().await.unwrap();

        // 4. Peer sends server_handshake response, peer_handshake request

        let mut peer_handshakes = Handshakes::new();
        let (peer_hand, _, _) = peer_handshakes
            .receive_any(1u64, 0u32, peer_address, server_address, reader)
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
