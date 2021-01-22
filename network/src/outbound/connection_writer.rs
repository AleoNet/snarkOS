// Copyright (C) 2019-2021 Aleo Systems Inc.
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

use parking_lot::Mutex;
use tokio::{io::AsyncWriteExt, net::tcp::OwnedWriteHalf, sync::Mutex as AsyncMutex};

use std::{net::SocketAddr, sync::Arc};

/// A channel for writing messages to a peer.
/// The write stream is protected by an Arc + Mutex to enable cloning.
#[derive(Debug)]
pub struct ConnWriter {
    pub addr: SocketAddr,
    pub writer: AsyncMutex<OwnedWriteHalf>,
    buffer: AsyncMutex<Box<[u8]>>,
    noise: Arc<Mutex<snow::TransportState>>,
}

impl ConnWriter {
    pub fn new(
        addr: SocketAddr,
        writer: OwnedWriteHalf,
        buffer: Box<[u8]>,
        noise: Arc<Mutex<snow::TransportState>>,
    ) -> Self {
        Self {
            addr,
            writer: AsyncMutex::new(writer),
            buffer: AsyncMutex::new(buffer),
            noise,
        }
    }

    /// Writes a message consisting of a header and payload.
    pub async fn write_message(&self, payload: &Payload) -> Result<(), ConnectError> {
        let serialized_payload = bincode::serialize(payload).map_err(|e| ConnectError::MessageError(e.into()))?;

        {
            let mut buffer = self.buffer.lock().await;
            let len = self
                .noise
                .lock()
                .write_message(&serialized_payload, &mut buffer)
                .map_err(|e| ConnectError::Message(e.to_string()))?;

            let header = MessageHeader::from(len);
            let mut writer = self.writer.lock().await;
            writer.write_all(&header.as_bytes()[..]).await?;
            writer.write_all(&buffer[..len]).await?;
        }

        debug!("Sent a {} to {}", payload, self.addr);

        Ok(())
    }
}
