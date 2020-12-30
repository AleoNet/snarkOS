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
    errors::network::HandshakeError,
    external::{
        message_types::{GetPeers, Verack, Version},
        Channel,
        Handshake,
        HandshakeState,
        Message,
    },
};

use std::{collections::HashMap, net::SocketAddr};
use tokio::net::TcpStream;

/// Stores the address and latest state of peers we are handshaking with.
#[derive(Clone, Debug, Default)]
pub struct Handshakes {
    handshakes: HashMap<SocketAddr, Handshake>,
}

impl Handshakes {
    /// Construct a new store of connected peer `Handshakes`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new handshake with a peer and send a handshake request to them.
    /// If the request is sent successfully, the handshake is stored and returned.
    pub async fn send_request(&mut self, version: &Version) -> Result<(), HandshakeError> {
        let handshake = Handshake::send_new(version).await?;

        self.handshakes.insert(version.address_receiver, handshake);
        info!("Request handshake with: {:?}", version.address_receiver);

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
        peer_address: SocketAddr,
        reader: TcpStream,
    ) -> Result<(Handshake, SocketAddr, Option<Version>), HandshakeError> {
        let channel = Channel::new_read_only(reader)?;

        // Read the first message or error
        let (name, bytes) = channel.read().await?;

        // Create and insert a new handshake when the channel contains a version message.
        if Version::name() == name {
            let remote_version = Version::deserialize(bytes)?;

            // Peer address and specified port from the version message
            let remote_address = SocketAddr::new(peer_address.ip(), remote_version.address_sender.port());
            let local_address = remote_version.address_receiver;

            let local_version = Version::new(version, height, remote_address, local_address);
            let handshake = Handshake::receive_new(channel, &local_version, &remote_version).await?;

            self.handshakes.insert(remote_address, handshake.clone());

            Ok((handshake, local_address, Some(local_version)))
        }
        // Establish the channel when the channel contains a verack message.
        else if Verack::name() == name {
            let verack = Verack::deserialize(bytes)?;

            let remote_address = verack.address_sender;
            let local_address = verack.address_receiver;

            match self.get_mut(&remote_address) {
                Some(handshake) => {
                    handshake.accept(verack).await?;
                    handshake.update_reader(channel);
                    info!("New handshake with: {:?}", remote_address);

                    // Get our new peer's peer_list
                    handshake.channel.write(&GetPeers).await?;

                    Ok((handshake.clone(), local_address, None))
                }
                None => Err(HandshakeError::HandshakeMissing(remote_address)),
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
        match self.handshakes.get(&address) {
            Some(stored_handshake) => Some(stored_handshake.get_state()),
            None => None,
        }
    }

    /// Returns a reference to the handshake at a peer address.
    pub fn get(&self, address: &SocketAddr) -> Option<&Handshake> {
        self.handshakes.get(&address)
    }

    /// Returns a mutable reference to the handshake at a peer address.
    fn get_mut(&mut self, address: &SocketAddr) -> Option<&mut Handshake> {
        self.handshakes.get_mut(&address)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::external::Message;
    use snarkos_testing::network::random_socket_address;

    use serial_test::serial;
    use tokio::net::TcpListener;

    #[tokio::test]
    #[serial]
    async fn test_handshakes() {
        let local_address = random_socket_address();
        let remote_address = random_socket_address();

        // 1. Bind to remote address
        let mut remote_listener = TcpListener::bind(remote_address).await.unwrap();

        tokio::spawn(async move {
            let mut local_listener = TcpListener::bind(local_address).await.unwrap();

            // 2. Local node sends handshake request

            let local_version = Version::new(1u64, 0u32, remote_address, local_address);

            let mut handshake = Handshakes::new();
            handshake.send_request(&local_version).await.unwrap();

            // 5. Check local node handshake state

            let (reader, _socket) = local_listener.accept().await.unwrap();
            let channel = Channel::new_read_only(reader).unwrap();

            assert_eq!(HandshakeState::Waiting, handshake.get_state(remote_address).unwrap());

            // 6. Local node accepts handshake response

            let (_name, bytes) = channel.read().await.unwrap();
            let verack = Verack::deserialize(bytes).unwrap();

            handshake.accept_response(remote_address, verack).await.unwrap();

            assert_eq!(HandshakeState::Accepted, handshake.get_state(remote_address).unwrap());

            // 7. Local node receives handshake request

            let (_name, bytes) = channel.read().await.unwrap();
            let remote_version = Version::deserialize(bytes).unwrap();

            // 8. Local node sends handshake response

            handshake.receive_request(remote_version, remote_address).await.unwrap();
        });

        // 3. Remote node accepts Local node connection

        let (reader, _socket) = remote_listener.accept().await.unwrap();

        // 4. Remote node sends handshake response, handshake request

        let mut handshakes = Handshakes::new();
        let (handshake, _, _) = handshakes.receive_any(1u64, 0u32, local_address, reader).await.unwrap();

        assert_eq!(HandshakeState::Waiting, handshakes.get_state(local_address).unwrap());

        // 9. Local node accepts handshake response

        let (_name, bytes) = handshake.channel.read().await.unwrap();
        let verack = Verack::deserialize(bytes).unwrap();

        handshakes.accept_response(local_address, verack).await.unwrap();

        assert_eq!(HandshakeState::Accepted, handshakes.get_state(local_address).unwrap())
    }
}
