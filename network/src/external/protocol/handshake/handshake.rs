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
    errors::HandshakeError,
    external::{
        message_types::{Verack, Version},
        Channel,
    },
};

use std::{net::SocketAddr, sync::Arc};

#[derive(Clone, Debug, PartialEq)]
pub enum HandshakeState {
    Waiting,
    Accepted,
    Rejected,
}

/// Establishes a connection between this node and a peer to send messages.
/// 1. The server sends a Version message to a peer.
/// 2. The peer responds with a Verack message followed by a Version message.
/// 3. The server verifies the Verack and adds the peer to its peer list.
/// 4. The server sees the Version message and responds with a Verack.
/// 5. The peer verifies the Verack and adds the server to its peer list.
///
/// Receiving a Version message means you should send a Verack message.
/// If you receive a Verack message from a peer and accept it, then the handshake is complete.
/// Peers with completed handshakes are added to your connections and your connected peer list.
#[derive(Clone, Debug)]
pub struct Handshake {
    pub channel: Arc<Channel>,
    pub state: HandshakeState,
    pub height: u32,
    pub nonce: u64,
}

impl Handshake {
    /// Send the initial Version message to a peer
    pub async fn send_new(local_version: &Version) -> Result<Self, HandshakeError> {
        // Create temporary write only channel
        let channel = Arc::new(Channel::new_write_only(local_version.address_receiver).await?);

        // Write Version request
        channel.write(local_version).await?;

        Ok(Self {
            channel,
            state: HandshakeState::Waiting,
            height: local_version.height,
            nonce: local_version.nonce,
        })
    }

    /// Receive the initial Version message from a new peer.
    /// Send a Verack message + Version message
    pub async fn receive_new(
        channel: Channel,
        local_version: &Version,
        remote_version: &Version,
    ) -> Result<Handshake, HandshakeError> {
        // Connect to the address specified in the peer_message
        let channel = channel.update_writer(local_version.address_receiver).await?;

        // Write Verack response
        channel
            .write(&Verack::new(
                remote_version.nonce,
                local_version.address_receiver,
                local_version.address_sender,
            ))
            .await?;

        // Write Version request
        channel.write(local_version).await?;

        Ok(Self {
            channel: Arc::new(channel),
            state: HandshakeState::Waiting,
            height: local_version.height,
            nonce: local_version.nonce,
        })
    }

    /// Receive the Version message for an existing peer handshake.
    /// Send a Verack message.
    pub async fn receive(&mut self, version: Version) -> Result<(), HandshakeError> {
        // You are the new sender and your peer is the receiver
        let address_receiver = self.channel.address;
        let address_sender = version.address_receiver;

        self.channel
            .write(&Verack::new(version.nonce, address_receiver, address_sender))
            .await?;
        Ok(())
    }

    /// Accept the Verack from a peer.
    pub async fn accept(&mut self, message: Verack) -> Result<(), HandshakeError> {
        if self.nonce != message.nonce {
            self.state = HandshakeState::Rejected;

            return Err(HandshakeError::InvalidNonce(self.nonce, message.nonce));
        } else if self.state == HandshakeState::Waiting {
            self.state = HandshakeState::Accepted;
        }

        Ok(())
    }

    /// Updates the stored channel address if needed for an existing peer handshake.
    pub fn update_address(&mut self, address: SocketAddr) {
        if self.channel.address != address {
            self.channel = Arc::new(self.channel.update_address(address))
        }
    }

    /// Updates the stored reader stream for an existing peer handshake.
    pub fn update_reader(&mut self, channel: Channel) {
        self.channel = Arc::new(self.channel.update_reader(channel.reader))
    }

    /// Returns current handshake state.
    pub fn get_state(&self) -> HandshakeState {
        self.state.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::external::message::Message;
    use snarkos_testing::network::random_socket_address;

    use serial_test::serial;
    use tokio::net::TcpListener;

    #[tokio::test]
    #[serial]
    async fn test_handshake_full() {
        let local_address = random_socket_address();
        let remote_address = random_socket_address();

        // 1. Bind to remote address

        let mut remote_listener = TcpListener::bind(remote_address).await.unwrap();

        tokio::spawn(async move {
            let mut local_listener = TcpListener::bind(local_address).await.unwrap();

            // 2. Local node connects to remote. Remote node sends handshake Version

            let local_version = Version::new(1u64, 0u32, remote_address, local_address);
            let mut handshake = Handshake::send_new(&local_version).await.unwrap();

            let (reader, _socket) = local_listener.accept().await.unwrap();
            let channel = Channel::new_read_only(reader).unwrap();

            handshake.update_reader(channel);

            // 5. Local node accepts handshake Verack

            let (_name, bytes) = handshake.channel.read().await.unwrap();
            let verack = Verack::deserialize(bytes).unwrap();

            handshake.accept(verack).await.unwrap();

            // 6. Local node receives handshake Version

            let (_name, bytes) = handshake.channel.read().await.unwrap();
            let remote_version = Version::deserialize(bytes).unwrap();

            // 7. Local node sends handshake Verack

            handshake.receive(remote_version).await.unwrap();
        });

        // 3. Remote node accepts Local node connection

        let (reader, _socket) = remote_listener.accept().await.unwrap();
        let channel = Channel::new_read_only(reader).unwrap();
        let (_name, bytes) = channel.read().await.unwrap();

        // 4. Remote node receives handshake Version.
        // Remote node sends handshake Verack, handshake Version

        let local_version = Version::new(1u64, 0u32, local_address, remote_address);
        let remote_version = Version::deserialize(bytes).unwrap();

        let mut handshake = Handshake::receive_new(channel, &local_version, &remote_version)
            .await
            .unwrap();

        // 8. Remote node accepts handshake Verack

        let (_name, bytes) = handshake.channel.read().await.unwrap();
        let verack = Verack::deserialize(bytes).unwrap();

        handshake.accept(verack).await.unwrap();
    }
}
