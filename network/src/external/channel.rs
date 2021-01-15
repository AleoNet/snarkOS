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

use tokio::{
    io::AsyncWriteExt,
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf},
        TcpStream,
    },
    sync::Mutex as AsyncMutex,
};

use std::{net::SocketAddr, sync::Arc};

/// A channel for writing messages to a peer.
/// The write stream is protected by an Arc + Mutex to enable cloning.
#[derive(Clone, Debug)]
pub struct Channel {
    pub remote_address: SocketAddr,
    pub writer: Arc<AsyncMutex<OwnedWriteHalf>>,
}

impl Channel {
    pub fn new(remote_address: SocketAddr, stream: TcpStream) -> (Self, OwnedReadHalf) {
        let (reader, writer) = stream.into_split();

        let channel = Self {
            remote_address,
            writer: Arc::new(AsyncMutex::new(writer)),
        };

        (channel, reader)
    }

    pub async fn from_addr(remote_address: SocketAddr) -> Result<(Self, OwnedReadHalf), ConnectError> {
        let stream = TcpStream::connect(remote_address).await?;

        Ok(Channel::new(remote_address, stream))
    }

    /// Updates the address associated with the given channel.
    pub async fn update_address(&mut self, remote_address: SocketAddr) {
        self.remote_address = remote_address;
    }

    /// Writes a message header + payload.
    pub async fn write(&self, payload: &Payload) -> Result<(), ConnectError> {
        let serialized_payload = bincode::serialize(payload).map_err(|e| ConnectError::MessageError(e.into()))?;
        let header = MessageHeader::from(serialized_payload.len());

        {
            let mut writer = self.writer.lock().await;
            writer.write_all(&header.as_bytes()[..]).await?;
            writer.write_all(&serialized_payload).await?;
        }

        debug!("Sent a {} to {}", payload, self.remote_address);

        Ok(())
    }
}

/// Reads a message header + payload.
pub(crate) async fn read_from_stream(
    addr: SocketAddr,
    reader: &mut OwnedReadHalf,
    buffer: &mut [u8],
) -> Result<Message, ConnectError> {
    let header = read_header(reader).await?;
    let payload = read_payload(reader, &mut buffer[..header.len()]).await?;
    let payload = bincode::deserialize(&payload).map_err(|e| ConnectError::MessageError(e.into()))?;

    debug!("Received a '{}' message from {}", payload, addr);

    Ok(Message::new(Direction::Inbound(addr), payload))
}
