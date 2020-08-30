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
    message_types::{Ping, Pong},
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
