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

use crate::{errors::message::*, external::message::*, ConnectError};

use parking_lot::Mutex;
use tokio::{net::tcp::OwnedReadHalf, prelude::*};

use std::{net::SocketAddr, sync::Arc};

pub struct ConnReader {
    pub addr: SocketAddr,
    reader: OwnedReadHalf,
    buffer: Box<[u8]>,
    noise_buffer: Box<[u8]>,
    noise: Arc<Mutex<snow::TransportState>>,
}

impl ConnReader {
    pub fn new(
        addr: SocketAddr,
        reader: OwnedReadHalf,
        buffer: Box<[u8]>,
        noise: Arc<Mutex<snow::TransportState>>,
    ) -> Self {
        Self {
            addr,
            reader,
            noise_buffer: vec![0u8; crate::NOISE_BUF_LEN].into(),
            buffer,
            noise,
        }
    }

    /// Returns a message header read from an input stream.
    pub async fn read_header(&mut self) -> Result<MessageHeader, MessageHeaderError> {
        let mut header_arr = [0u8; 4];
        self.reader.read_exact(&mut header_arr).await?;
        let header = MessageHeader::from(header_arr);

        if header.len as usize > crate::MAX_MESSAGE_SIZE {
            Err(MessageHeaderError::TooBig(header.len as usize, crate::MAX_MESSAGE_SIZE))
        } else {
            Ok(header)
        }
    }

    /// Reads a message header + payload.
    pub async fn read_message(&mut self) -> Result<Message, ConnectError> {
        let header = self.read_header().await?;
        let len = header.len();
        let mut decrypted_len = 0;
        let mut processed_len = 0;

        {
            while processed_len < len {
                let chunk_len = std::cmp::min(crate::NOISE_BUF_LEN, len - processed_len);
                self.reader.read_exact(&mut self.noise_buffer[..chunk_len]).await?;
                processed_len += chunk_len;

                decrypted_len += self
                    .noise
                    .lock()
                    .read_message(&self.noise_buffer[..chunk_len], &mut self.buffer[decrypted_len..])
                    .unwrap();
            }
        }
        let payload =
            bincode::deserialize(&self.buffer[..decrypted_len]).map_err(|e| ConnectError::MessageError(e.into()))?;

        debug!("Received a '{}' message from {}", payload, self.addr);

        Ok(Message::new(Direction::Inbound(self.addr), payload))
    }
}
