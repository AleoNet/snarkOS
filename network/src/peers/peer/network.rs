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

use tokio::{
    io::{AsyncRead, AsyncReadExt},
    net::tcp::{OwnedReadHalf, OwnedWriteHalf},
};

use crate::{NetworkError, Payload};

use super::cipher::Cipher;

// used in integration tests
#[doc(hidden)]
pub struct PeerIOHandle {
    pub reader: Option<OwnedReadHalf>,
    pub writer: OwnedWriteHalf,
    pub cipher: Cipher,
}

impl PeerIOHandle {
    pub async fn write_payload(&mut self, payload: &Payload) -> Result<(), NetworkError> {
        let serialized_payload = Payload::serialize(&payload)?;
        self.cipher
            .write_packet(&mut self.writer, &serialized_payload[..])
            .await?;
        Ok(())
    }

    pub fn read_payload(&mut self, payload: &[u8]) -> Result<&[u8], NetworkError> {
        self.cipher.read_packet(payload)
    }

    pub fn take_reader(&mut self) -> PeerReader<OwnedReadHalf> {
        PeerReader {
            reader: self.reader.take().unwrap(),
            buffer: vec![0u8; crate::MAX_MESSAGE_SIZE].into(),
        }
    }
}

#[doc(hidden)]
pub struct PeerReader<R: AsyncRead + Unpin + 'static> {
    pub reader: R,
    pub buffer: Box<[u8]>,
}

impl<R: AsyncRead + Unpin + 'static> PeerReader<R> {
    pub async fn read_raw_payload(&mut self) -> Result<&[u8], NetworkError> {
        let length = self.reader.read_u32().await? as usize;
        if length > crate::MAX_MESSAGE_SIZE {
            return Err(NetworkError::MessageTooBig(length));
        } else if length == 0 {
            return Err(NetworkError::ZeroLengthMessage);
        }
        self.reader.read_exact(&mut self.buffer[..length]).await?;
        Ok(&self.buffer[..length])
    }
}
