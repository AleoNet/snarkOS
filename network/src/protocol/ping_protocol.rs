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

/// Maintain connected peers.
/// 1. The server sends a Ping message to a peer.
/// 2. The peer responds with a Pong message.
/// 3. The server verifies the Pong message and updates the peer's last seen date
#[derive(Clone, Debug)]
pub struct PingProtocol {
    state: PingState,
    channel: Arc<Channel>,
    nonce: u64,
}

impl PingProtocol {
    /// Send the initial ping message to a peer.
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

    /// Returns current ping protocol state.
    pub fn get_state(&self) -> PingState {
        self.state.clone()
    }
}
