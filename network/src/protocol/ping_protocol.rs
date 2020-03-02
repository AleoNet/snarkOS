use crate::{
    message::types::{Ping, Pong},
    Channel,
};
use snarkos_errors::network::PingProtocolError;
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq)]
pub enum PingState {
    Waiting,
    Accepted,
    Rejected,
}

/// Ping protocol
/// 1. The server sends a Ping message to a peer.
/// 2. The peer responds with a Pong message.
/// 3. The server verifies the Pong message and updates the peer's last seen date
#[derive(Clone, Debug)]
pub struct PingProtocol {
    pub state: PingState,
    pub channel: Arc<Channel>,
    pub nonce: u64,
}

impl PingProtocol {
    /// Send the initial ping message to a peer
    pub async fn send(channel: Arc<Channel>) -> Result<Self, PingProtocolError> {
        let message = Ping::new();
        channel.write(&message).await?;

        Ok(Self {
            state: PingState::Waiting,
            channel,
            nonce: message.nonce,
        })
    }

    /// Receive the initial ping message from a peer. Respond with a pong.
    pub async fn receive(message: Ping, channel: Arc<Channel>) -> Result<(), PingProtocolError> {
        channel.write(&Pong::new(message)).await?;

        Ok(())
    }

    /// Accept the pong from a peer.
    pub async fn accept(&mut self, message: Pong) -> Result<(), PingProtocolError> {
        if self.nonce != message.nonce {
            self.state = PingState::Rejected;

            return Err(PingProtocolError::InvalidNonce(self.nonce, message.nonce));
        }

        self.state = PingState::Accepted;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        message::Message,
        test_data::{get_channel, random_socket_address},
    };
    use serial_test::serial;
    use tokio::{net::TcpListener, sync::Mutex};

    #[tokio::test]
    #[serial]
    async fn test_ping_protocol() {
        let server_address = random_socket_address();
        let peer_address = random_socket_address();

        // 1. Bind listener to Server address

        let mut server_listener = TcpListener::bind(server_address).await.unwrap();

        tokio::spawn(async move {
            let mut peer_listener = TcpListener::bind(peer_address).await.unwrap();

            // 2. Peer connects to server address
            let channel = Channel::new_write_only(server_address).await.unwrap();

            let (reader, _socket) = peer_listener.accept().await.unwrap();

            let channel = Arc::new(channel.update_reader(Arc::new(Mutex::new(reader))));

            // 4. Peer send ping request

            let mut peer_ping = PingProtocol::send(channel.clone()).await.unwrap();

            // 5. Peer accepts server pong response

            let (name, bytes) = channel.read().await.unwrap();

            assert_eq!(Pong::name(), name);

            peer_ping.accept(Pong::deserialize(bytes).unwrap()).await.unwrap();
        });

        // 3. Server accepts Peer connection

        let channel = Arc::new(get_channel(&mut server_listener, peer_address).await);

        // 4. Server receives peer ping request. Sends pong response

        let (name, bytes) = channel.read().await.unwrap();

        assert_eq!(Ping::name(), name);

        PingProtocol::receive(Ping::deserialize(bytes).unwrap(), channel)
            .await
            .unwrap();
    }
}
