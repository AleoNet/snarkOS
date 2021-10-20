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

use crate::{network::NetworkError, Environment};
use snarkvm::dpc::Network;

use snow::TransportState;
use std::{convert::TryInto, marker::PhantomData};
use tokio::io::{AsyncWrite, AsyncWriteExt};

#[derive(Debug)]
pub(crate) struct Cipher<N: Network, E: Environment<N>> {
    state: TransportState,
    buffer: Vec<u8>,
    noise_buffer: Box<[u8]>,
    _phantom: PhantomData<(N, E)>,
}

impl<N: Network, E: Environment<N>> Cipher<N, E> {
    pub fn new(state: TransportState, buffer: Vec<u8>, noise_buffer: Box<[u8]>) -> Self {
        assert_eq!(noise_buffer.len(), E::NOISE_BUFFER_LENGTH);
        Self {
            state,
            buffer,
            noise_buffer,
            _phantom: PhantomData,
        }
    }

    pub async fn write_packet<W: AsyncWrite + Unpin>(&mut self, writer: &mut W, data: &[u8]) -> Result<(), NetworkError> {
        if data.len() > E::MAX_MESSAGE_SIZE {
            return Err(NetworkError::MessageTooBig(data.len()));
        }

        let mut encrypted_len = 4; // account for the final message length
        let mut processed_len = 0;

        while processed_len < data.len() {
            let chunk_len = std::cmp::min(self.noise_buffer.len() - E::NOISE_TAG_LENGTH, data[processed_len..].len());
            let chunk = &data[processed_len..][..chunk_len];

            if self.buffer.len() < encrypted_len + chunk_len + E::NOISE_TAG_LENGTH {
                self.buffer.resize(encrypted_len + chunk_len + E::NOISE_TAG_LENGTH, 0);
            }

            encrypted_len += self.state.write_message(chunk, &mut self.buffer[encrypted_len..])?;
            processed_len += chunk_len;
        }

        let network_len: u32 = encrypted_len.try_into().map_err(|_| NetworkError::MessageTooBig(encrypted_len))?;
        if encrypted_len > E::MAX_MESSAGE_SIZE {
            return Err(NetworkError::MessageTooBig(encrypted_len));
        }

        self.buffer[..4].copy_from_slice(&(network_len - 4).to_be_bytes()[..]);
        writer.write_all(&self.buffer[..encrypted_len]).await?;
        writer.flush().await?;

        Ok(())
    }

    pub fn read_packet(&mut self, payload: &[u8]) -> Result<&[u8], NetworkError> {
        if self.buffer.len() < payload.len() {
            self.buffer.resize(payload.len(), 0);
        }

        let mut decrypted_len = 0;
        let mut processed_len = 0;

        while processed_len < payload.len() {
            let chunk_len = std::cmp::min(E::NOISE_BUFFER_LENGTH, payload.len() - processed_len);
            let chunk = &payload[processed_len..][..chunk_len];

            decrypted_len += self.state.read_message(chunk, &mut self.buffer[decrypted_len..])?;
            processed_len += chunk_len;
        }

        Ok(&self.buffer[..decrypted_len])
    }
}
