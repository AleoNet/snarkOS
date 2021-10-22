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

use snarkvm::prelude::*;

use ::bytes::{Buf, BytesMut};
use anyhow::{anyhow, Result};
use std::net::SocketAddr;
use tokio_util::codec::{Decoder, Encoder};

#[derive(Clone, Debug)]
pub enum Message<N: Network> {
    /// ChallengeRequest := (listener_port, block_height)
    ChallengeRequest(u16, u32),
    /// ChallengeResponse := (block_header)
    ChallengeResponse(BlockHeader<N>),
    /// PeerRequest := ()
    PeerRequest,
    /// PeerResponse := (\[peer_ip\])
    PeerResponse(Vec<SocketAddr>),
    /// Ping := (block_height)
    Ping(u32),
    /// Pong := ()
    Pong,
}

impl<N: Network> Message<N> {
    /// Returns the message name.
    #[inline]
    pub fn name(&self) -> &str {
        match self {
            Self::ChallengeRequest(..) => "ChallengeRequest",
            Self::ChallengeResponse(..) => "ChallengeResponse",
            Self::PeerRequest => "PeerRequest",
            Self::PeerResponse(..) => "PeerResponse",
            Self::Ping(..) => "Ping",
            Self::Pong => "Pong",
        }
    }

    /// Returns the message ID.
    #[inline]
    pub fn id(&self) -> u16 {
        match self {
            Self::ChallengeRequest(..) => 0,
            Self::ChallengeResponse(..) => 1,
            Self::PeerRequest => 2,
            Self::PeerResponse(..) => 3,
            Self::Ping(..) => 4,
            Self::Pong => 5,
        }
    }

    /// Returns the message data as bytes.
    #[inline]
    pub fn data(&self) -> Result<Vec<u8>> {
        match self {
            Self::ChallengeRequest(listener_port, block_height) => {
                Ok([listener_port.to_le_bytes().to_vec(), block_height.to_le_bytes().to_vec()].concat())
            }
            Self::ChallengeResponse(block_header) => block_header.to_bytes_le(),
            Self::PeerRequest => Ok(vec![]),
            Self::PeerResponse(peer_ips) => Ok(bincode::serialize(peer_ips)?),
            Self::Ping(block_height) => Ok(block_height.to_le_bytes().to_vec()),
            Self::Pong => Ok(vec![]),
        }
    }

    /// Serializes the given message into bytes.
    #[inline]
    pub fn serialize(&self) -> Result<Vec<u8>> {
        Ok([self.id().to_le_bytes().to_vec(), self.data()?].concat())
    }

    /// Deserializes the given buffer into a message.
    #[inline]
    pub fn deserialize(buffer: &[u8]) -> Result<Self> {
        // Ensure the buffer contains at least the length of an ID.
        if buffer.len() < 2 {
            return Err(anyhow!("Invalid message buffer"));
        }

        // Split the buffer into the ID and data portion.
        let id = u16::from_le_bytes([buffer[0], buffer[1]]);
        let data = &buffer[2..];

        // Deserialize the data field.
        match id {
            0 => Ok(Self::ChallengeRequest(
                bincode::deserialize(&data[0..2])?,
                bincode::deserialize(&data[2..])?,
            )),
            1 => Ok(Self::ChallengeResponse(bincode::deserialize(data)?)),
            2 => match data.len() == 0 {
                true => Ok(Self::PeerRequest),
                false => Err(anyhow!("Invalid 'PeerRequest' message: {:?} {:?}", buffer, data)),
            },
            3 => Ok(Self::PeerResponse(bincode::deserialize(data)?)),
            4 => Ok(Self::Ping(bincode::deserialize(data)?)),
            5 => match data.len() == 0 {
                true => Ok(Self::Pong),
                false => Err(anyhow!("Invalid 'Pong' message: {:?} {:?}", buffer, data)),
            },
            _ => Err(anyhow!("Invalid message ID {}", id)),
        }
    }
}

const MAX: usize = 8 * 1024 * 1024;

impl<N: Network> Encoder<Message<N>> for Message<N> {
    type Error = anyhow::Error;

    fn encode(&mut self, message: Message<N>, dst: &mut BytesMut) -> Result<(), Self::Error> {
        // Serialize the message into a buffer.
        let buffer = message.serialize()?;

        // Ensure the message does not exceed the maximum length limit.
        if buffer.len() > MAX {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Frame of length {} is too large.", buffer.len()),
            )
            .into());
        }

        // Convert the length into a byte array.
        // The cast to u32 cannot overflow due to the length check above.
        let len_slice = u32::to_le_bytes(buffer.len() as u32);

        // Reserve space in the buffer.
        dst.reserve(4 + buffer.len());

        // Write the length and string to the buffer.
        dst.extend_from_slice(&len_slice);
        dst.extend_from_slice(&buffer);
        Ok(())
    }
}

impl<N: Network> Decoder for Message<N> {
    type Error = std::io::Error;
    type Item = Message<N>;

    fn decode(&mut self, source: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        // Ensure there is enough bytes to read the length marker.
        if source.len() < 4 {
            return Ok(None);
        }

        // Read the length marker.
        let mut length_bytes = [0u8; 4];
        length_bytes.copy_from_slice(&source[..4]);
        let length = u32::from_le_bytes(length_bytes) as usize;

        // Check that the length is not too large to avoid a denial of
        // service attack where the node server runs out of memory.
        if length > MAX {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Frame of length {} is too large.", length),
            ));
        }

        if source.len() < 4 + length {
            // The full message has not yet arrived.
            //
            // We reserve more space in the buffer. This is not strictly
            // necessary, but is a good idea performance-wise.
            source.reserve(4 + length - source.len());

            // We inform `Framed` that we need more bytes to form the next frame.
            return Ok(None);
        }

        // Use `advance` to modify the source such that it no longer contains this frame.
        let buffer = source[4..4 + length].to_vec();
        source.advance(4 + length);

        // Convert the buffer to a message, or fail if it is not valid.
        match Message::deserialize(&buffer) {
            Ok(message) => Ok(Some(message)),
            Err(error) => Err(std::io::Error::new(std::io::ErrorKind::InvalidData, error)),
        }
    }
}
