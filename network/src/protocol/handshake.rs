use crate::message::{
    types::{Verack, Version},
    Channel,
};
use snarkos_errors::network::HandshakeError;

use std::{net::SocketAddr, sync::Arc};

#[derive(Clone, Debug, PartialEq)]
pub enum HandshakeState {
    Waiting,
    Accepted,
    Rejected,
}

/// Establishes a connection between this node and a peer to send messages.
///
/// 1. The server sends a Version message to a peer.
/// 2. The peer responds with a Verack message followed by a Version message.
/// 3. The server verifies the Verack and adds the peer to its peer list.
/// 4. The server sees the Version message and responds with a Verack.
/// 5. The peer verifies the Verack and adds the server to its peer list.
///
/// Essentially receiving a Version message means you should send a Verack message.
/// If you receive a Verack message from a peer and accept it, then the handshake is complete.
/// Peers with completed handshakes are added to your connections and your connected peer list.
#[derive(Clone, Debug)]
pub struct Handshake {
    pub channel: Arc<Channel>,
    state: HandshakeState,
    version: u64,
    height: u32,
    nonce: u64,
}

impl Handshake {
    /// Send the initial Version message to a peer
    pub async fn send_new(
        version: u64,
        height: u32,
        address_sender: SocketAddr,
        address_receiver: SocketAddr,
    ) -> Result<Self, HandshakeError> {
        // Create temporary write only channel
        let channel = Arc::new(Channel::new_write_only(address_receiver).await?);

        // Write Version request
        let message = Version::new(version, height, address_receiver, address_sender);

        channel.write(&message).await?;

        Ok(Self {
            state: HandshakeState::Waiting,
            channel,
            version,
            height,
            nonce: message.nonce,
        })
    }

    /// Receive the initial Version message from a new peer.
    ///
    /// Send a Verack message + Version message
    pub async fn receive_new(
        version: u64,
        height: u32,
        channel: Channel,
        peer_message: Version,
        address_sender: SocketAddr,
    ) -> Result<Handshake, HandshakeError> {
        let peer_address = peer_message.address_sender;

        // Connect to the address specified in the peer_message
        let channel = channel.update_writer(peer_address).await?;

        // Write Verack response
        channel.write(&Verack::new(peer_message.clone())).await?;

        // Write Version request
        channel
            .write(&Version::from(
                version,
                height,
                peer_address,
                address_sender,
                peer_message.nonce,
            ))
            .await?;

        Ok(Self {
            state: HandshakeState::Waiting,
            channel: Arc::new(channel),
            version,
            height,
            nonce: peer_message.nonce,
        })
    }

    /// Receive the Version message for an existing peer handshake.
    ///
    /// Send a Verack message.
    pub async fn receive(&mut self, message: Version) -> Result<(), HandshakeError> {
        self.channel.write(&Verack::new(message)).await?;
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
    pub fn update_reader(&mut self, read_channel: Channel) {
        self.channel = Arc::new(self.channel.update_reader(read_channel.reader))
    }

    /// Returns current handshake state.
    pub fn get_state(&self) -> HandshakeState {
        self.state.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{message::Message, test_data::random_socket_address};
    use serial_test::serial;
    use tokio::net::TcpListener;

    #[tokio::test]
    #[serial]
    async fn test_handshake_full() {
        let server_address = random_socket_address();
        let peer_address = random_socket_address();

        // 1. Bind to peer address

        let mut peer_listener = TcpListener::bind(peer_address).await.unwrap();

        tokio::spawn(async move {
            let mut server_listener = TcpListener::bind(server_address).await.unwrap();

            // 2. Server connects to peer, server sends server_hand Version

            let mut server_hand = Handshake::send_new(1u64, 0u32, server_address, peer_address)
                .await
                .unwrap();

            let (reader, _socket) = server_listener.accept().await.unwrap();
            let read_channel = Channel::new_read_only(reader).unwrap();

            server_hand.update_reader(read_channel);

            // 5. Server accepts server_hand Verack

            let (_name, bytes) = server_hand.channel.read().await.unwrap();
            let message = Verack::deserialize(bytes).unwrap();

            server_hand.accept(message).await.unwrap();

            // 6. Server receives peer_hand Version

            let (_name, bytes) = server_hand.channel.read().await.unwrap();
            let message = Version::deserialize(bytes).unwrap();

            // 7. Server sends peer_hand Verack

            server_hand.receive(message).await.unwrap();
        });

        // 3. Peer accepts Server connection

        let (reader, _socket) = peer_listener.accept().await.unwrap();
        let read_channel = Channel::new_read_only(reader).unwrap();
        let (_name, bytes) = read_channel.read().await.unwrap();

        // 4. Peer receives server_handshake Version.
        // Peer sends server_handshake Verack, peer_handshake Version

        let mut peer_hand = Handshake::receive_new(
            1u64,
            0u32,
            read_channel,
            Version::deserialize(bytes).unwrap(),
            server_address,
        )
        .await
        .unwrap();

        // 8. Peer accepts peer_handshake Verack

        let (_name, bytes) = peer_hand.channel.read().await.unwrap();
        let message = Verack::deserialize(bytes).unwrap();

        peer_hand.accept(message).await.unwrap();
    }
}
