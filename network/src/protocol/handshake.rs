use snarkos_errors::network::HandshakeError;

use crate::message::{
    types::{Verack, Version},
    Channel,
};
use std::{net::SocketAddr, sync::Arc};

#[derive(Clone, Debug, PartialEq)]
pub enum HandshakeState {
    Waiting,
    Accepted,
    Rejected,
}

/// Handshake protocol
/// 1. The server sends a Version message to a peer.
/// 2. The peer responds with a Verack message followed by a Version message.
/// 3. The server verifies the Verack and adds the peer to its peer list.
/// 4. The server sees the Version message and responds with a Verack.
/// 5. The peer verifies the Verack and adds the server to its peer list.
#[derive(Clone, Debug)]
pub struct Handshake {
    pub state: HandshakeState,
    pub channel: Arc<Channel>,
    pub version: u64,
    pub height: u32,
    pub nonce: u64,
    pub address_sender: SocketAddr,
}

impl Handshake {
    /// Send the initial version message to a peer
    pub async fn send_new(
        channel: Arc<Channel>,
        version: u64,
        height: u32,
        address_sender: SocketAddr,
    ) -> Result<Self, HandshakeError> {
        info!("Sending Handshake Version to {:?}", channel.address);
        let message = Version::new(version, height, channel.address, address_sender);
        channel.write(&message).await?;

        Ok(Self {
            state: HandshakeState::Waiting,
            channel,
            version,
            height,
            nonce: message.nonce,
            address_sender,
        })
    }

    /// Accept the initial version message from a peer
    pub async fn receive_new(
        message: Version,
        new_peer: bool,
        channel: Arc<Channel>,
        version: u64,
        height: u32,
        address_sender: SocketAddr,
    ) -> Result<Option<Handshake>, HandshakeError> {
        info!("Sending Handshake Verack to:  {:?}", channel.address);

        channel.write(&Verack::new(message)).await?;

        let mut handshake = None;

        if new_peer {
            handshake = Some(Handshake::send_new(channel.clone(), version, height, address_sender).await?);
        }
        Ok(handshake)
    }

    /// Accept the verack from a peer
    pub async fn accept(&mut self, message: Verack) -> Result<(), HandshakeError> {
        if self.nonce != message.nonce {
            self.state = HandshakeState::Rejected;

            return Err(HandshakeError::InvalidNonce(self.nonce, message.nonce));
        }

        self.state = HandshakeState::Accepted;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        message::Message,
        test_data::{get_next_channel, random_socket_address},
    };
    use serial_test::serial;
    use tokio::net::TcpListener;

    #[tokio::test]
    #[serial]
    async fn test_handshake_full() {
        let version = 1u64;
        let height = 0u32;
        let server_address = random_socket_address();
        let peer_address = random_socket_address();

        // 1. Bind listener to Server address

        let mut server_listener = TcpListener::bind(server_address).await.unwrap();

        tokio::spawn(async move {
            // 2. Peer connects to Server address

            let channel = Arc::new(Channel::connect(server_address).await.unwrap());

            // 4. Peer sends peer_handshake Version

            let mut peer_hand = Handshake::send_new(channel.clone(), 1u64, 0u32, peer_address)
                .await
                .unwrap();

            // 7. Peer accepts peer_handshake Verack

            let (_name, bytes) = channel.read().await.unwrap();
            let message = Verack::deserialize(bytes).unwrap();

            peer_hand.accept(message).await.unwrap();

            // 8. Peer receives server_handshake Version

            let (_name, bytes) = channel.read().await.unwrap();
            let message = Version::deserialize(bytes).unwrap();

            // 9. Peer sends server_handshake Verack

            let none_value = Handshake::receive_new(message, false, channel.clone(), version, height, peer_address)
                .await
                .unwrap();
            assert!(none_value.is_none());
        });

        // 3. Server accepts Peer connection

        let channel = get_next_channel(&mut server_listener).await;

        // 5. Server receives peer_handshake Version

        let (_name, bytes) = channel.read().await.unwrap();
        let message = Version::deserialize(bytes).unwrap();

        // 6. Server sends peer_handshake Verack, server_handshake Version

        let server_hand = Handshake::receive_new(message, true, channel.clone(), version, height, server_address)
            .await
            .unwrap();

        // 10. Server accepts server_handshake Verack

        let (_name, bytes) = channel.read().await.unwrap();
        let message = Verack::deserialize(bytes).unwrap();

        server_hand.unwrap().accept(message).await.unwrap();
    }
}
