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

use tokio::{io::AsyncWriteExt, net::tcp::OwnedWriteHalf, sync::Mutex as AsyncMutex};

use std::{net::SocketAddr, sync::Arc};

/// A channel for writing messages to a peer.
/// The write stream is protected by an Arc + Mutex to enable cloning.
#[derive(Clone, Debug)]
pub struct ConnWriter {
    pub addr: SocketAddr,
    pub writer: Arc<AsyncMutex<OwnedWriteHalf>>,
}

impl ConnWriter {
    pub fn new(addr: SocketAddr, writer: OwnedWriteHalf) -> Self {
        Self {
            addr,
            writer: Arc::new(AsyncMutex::new(writer)),
        }
    }

    /// Writes a message consisting of a header and payload.
    pub async fn write_message(&self, payload: &Payload) -> Result<(), ConnectError> {
        let serialized_payload = bincode::serialize(payload).map_err(|e| ConnectError::MessageError(e.into()))?;
        let header = MessageHeader::from(serialized_payload.len());

        {
            let mut writer = self.writer.lock().await;
            writer.write_all(&header.as_bytes()[..]).await?;
            writer.write_all(&serialized_payload).await?;
        }

        debug!("Sent a {} to {}", payload, self.addr);

        Ok(())
    }
}
