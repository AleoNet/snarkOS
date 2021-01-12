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

use crate::{errors::ConnectError, external::message::*};

use bincode;
use tokio::{
    io::AsyncWriteExt,
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf},
        TcpStream,
    },
    sync::Mutex,
};

use std::{net::SocketAddr, sync::Arc};

/// A channel for reading and writing messages to a peer.
/// The channel manages two streams to allow for simultaneous reading and writing.
/// Each stream is protected by an Arc + Mutex to allow for channel cloning.
#[derive(Clone, Debug)]
pub struct Channel {
    pub remote_address: SocketAddr,
    pub reader: Arc<Mutex<OwnedReadHalf>>,
    pub writer: Arc<Mutex<OwnedWriteHalf>>,
}

impl Channel {
    pub fn new(remote_address: SocketAddr, stream: TcpStream) -> Self {
        let (reader, writer) = stream.into_split();

        Self {
            remote_address,
            reader: Arc::new(Mutex::new(reader)),
            writer: Arc::new(Mutex::new(writer)),
        }
    }

    pub async fn from_addr(remote_address: SocketAddr) -> Result<Self, ConnectError> {
        let stream = TcpStream::connect(remote_address).await?;

        Ok(Channel::new(remote_address, stream))
    }

    /// Returns a new channel with the specified address and new writer stream.
    pub async fn update_address(self, remote_address: SocketAddr) -> Result<Self, ConnectError> {
        Ok(Self {
            remote_address,
            reader: self.reader,
            writer: self.writer,
        })
    }

    /// Writes a message header + payload.
    pub async fn write(&self, payload: &Payload) -> Result<(), ConnectError> {
        let serialized_payload = bincode::serialize(payload).map_err(|e| ConnectError::MessageError(e.into()))?;
        let header = MessageHeader {
            len: serialized_payload.len() as u32,
        };

        let mut writer = self.writer.lock().await;
        writer.write_all(&header.as_bytes()[..]).await?;
        writer.write_all(&serialized_payload).await?;

        debug!("Sent a {} to {}", payload, self.remote_address);

        Ok(())
    }

    /// Reads a message header + payload.
    pub async fn read(&self) -> Result<Message, ConnectError> {
        let header = read_header(&mut *self.reader.lock().await).await?;
        let payload = read_message(&mut *self.reader.lock().await, header.len as usize).await?;
        let payload = bincode::deserialize(&payload).map_err(|e| ConnectError::MessageError(e.into()))?;

        debug!("Received a '{}' message from {}", payload, self.remote_address);

        Ok(Message::new(Direction::Inbound(self.remote_address), payload))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use snarkos_testing::network::{random_bound_address, simulate_active_node};

    #[tokio::test]
    async fn channel_write() {
        // 1. Start remote node
        let remote_address = simulate_active_node().await;

        // 2. Server connect to peer
        let server_channel = Channel::from_addr(remote_address).await.unwrap();

        // 3. Server write message to peer
        server_channel.write(&Payload::GetPeers).await.unwrap();
    }

    #[tokio::test]
    async fn channel_read() {
        let (remote_address, remote_listener) = random_bound_address().await;

        tokio::spawn(async move {
            // 1. Server connects to peer.
            let server_channel = Channel::from_addr(remote_address).await.unwrap();

            // 2. Server writes GetPeers message.
            server_channel.write(&Payload::GetPeers).await.unwrap();
        });

        // 2. Peer accepts server connection.
        let (stream, address) = remote_listener.accept().await.unwrap();
        let peer_channel = Channel::new(address, stream);

        // 4. Peer reads GetPeers message.
        let message = peer_channel.read().await.unwrap();

        assert!(matches!(message.payload, Payload::GetPeers));
    }
}
