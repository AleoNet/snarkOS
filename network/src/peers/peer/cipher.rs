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

use std::convert::TryInto;

use snow::TransportState;
#[cfg(test)]
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::io::{AsyncWrite, AsyncWriteExt};

use crate::NetworkError;

pub struct Cipher {
    state: TransportState,
    buffer: Box<[u8]>,
    noise_buffer: Box<[u8]>,
}

impl Cipher {
    pub fn new(state: TransportState, buffer: Box<[u8]>, noise_buffer: Box<[u8]>) -> Self {
        assert_eq!(buffer.len(), crate::MAX_MESSAGE_SIZE + 4096);
        assert_eq!(noise_buffer.len(), crate::NOISE_BUF_LEN);
        Self {
            state,
            buffer,
            noise_buffer,
        }
    }

    pub async fn write_packet<W: AsyncWrite + Unpin>(
        &mut self,
        writer: &mut W,
        data: &[u8],
    ) -> Result<(), NetworkError> {
        if data.len() > self.buffer.len() {
            return Err(NetworkError::MessageTooBig(data.len()));
        }
        let mut encrypted_len = 0;
        let mut processed_len = 0;

        while processed_len < data.len() {
            let chunk_len = std::cmp::min(
                self.noise_buffer.len() - crate::NOISE_TAG_LEN,
                data[processed_len..].len(),
            );
            let chunk = &data[processed_len..][..chunk_len];

            encrypted_len += self.state.write_message(chunk, &mut self.buffer[encrypted_len..])?;
            processed_len += chunk_len;
        }

        let network_len: u32 = encrypted_len
            .try_into()
            .map_err(|_| NetworkError::MessageTooBig(encrypted_len))?;
        if encrypted_len > crate::MAX_MESSAGE_SIZE {
            return Err(NetworkError::MessageTooBig(encrypted_len));
        }
        writer.write_all(&network_len.to_be_bytes()[..]).await?;
        writer.write_all(&self.buffer[..encrypted_len]).await?;
        writer.flush().await?;
        Ok(())
    }

    pub fn read_packet(&mut self, payload: &[u8]) -> Result<&[u8], NetworkError> {
        let mut decrypted_len = 0;
        let mut processed_len = 0;

        while processed_len < payload.len() {
            let chunk_len = std::cmp::min(crate::NOISE_BUF_LEN, payload.len() - processed_len);
            let chunk = &payload[processed_len..][..chunk_len];

            decrypted_len += self.state.read_message(chunk, &mut self.buffer[decrypted_len..])?;
            processed_len += chunk_len;
        }

        Ok(&self.buffer[..decrypted_len])
    }

    #[cfg(test)]
    pub async fn read_packet_stream<R: AsyncRead + Unpin>(&mut self, reader: &mut R) -> Result<&[u8], NetworkError> {
        let length = reader.read_u32().await? as usize;
        if length > crate::MAX_MESSAGE_SIZE {
            return Err(NetworkError::MessageTooBig(length));
        } else if length == 0 {
            return Err(NetworkError::ZeroLengthMessage);
        }
        reader.read_exact(&mut self.buffer[..length]).await?;
        // only used in tests, so this is fine
        let copied = self.buffer[..length].to_vec();
        self.read_packet(&copied[..])
    }
}
