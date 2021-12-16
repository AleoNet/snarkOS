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

use crate::{helpers::State, Environment, NodeType};
use snarkos_storage::{BlockLocators, BlockTemplate};
use snarkvm::prelude::*;

use ::bytes::{Buf, BytesMut};
use anyhow::{anyhow, Result};
use serde::{de::DeserializeOwned, Serialize};
use std::{marker::PhantomData, net::SocketAddr};
use tokio::task;
use tokio_util::codec::{Decoder, Encoder};

/// This object enables deferred deserialization / ahead-of-time serialization for objects that
/// take a while to deserialize / serialize, in order to allow these operations to be non-blocking.
#[derive(Clone, Debug)]
pub enum Data<T: 'static + Serialize + DeserializeOwned + Send> {
    Object(T),
    Buffer(Vec<u8>),
}

impl<T: 'static + Serialize + DeserializeOwned + Send> Data<T> {
    pub fn deserialize_blocking(self) -> bincode::Result<T> {
        match self {
            Self::Object(x) => Ok(x),
            Self::Buffer(bytes) => bincode::deserialize(&bytes),
        }
    }

    pub async fn deserialize(self) -> bincode::Result<T> {
        match self {
            Self::Object(x) => Ok(x),
            Self::Buffer(bytes) => match task::spawn_blocking(move || bincode::deserialize(&bytes)).await {
                Ok(x) => x,
                Err(error) => Err(Box::new(bincode::ErrorKind::Custom(format!(
                    "Dedicated deserialization failed: {}",
                    error
                )))),
            },
        }
    }

    pub fn serialize_blocking(&self) -> bincode::Result<Vec<u8>> {
        match self {
            Self::Object(x) => bincode::serialize(x),
            Self::Buffer(bytes) => Ok(bytes.to_vec()),
        }
    }

    pub async fn serialize(self) -> bincode::Result<Vec<u8>> {
        match self {
            Self::Object(x) => match task::spawn_blocking(move || bincode::serialize(&x)).await {
                Ok(bytes) => bytes,
                Err(error) => Err(Box::new(bincode::ErrorKind::Custom(format!(
                    "Dedicated serialization failed: {}",
                    error
                )))),
            },
            Self::Buffer(bytes) => Ok(bytes),
        }
    }
}

#[derive(Clone, Debug)]
pub enum Message<N: Network, E: Environment> {
    /// BlockRequest := (start_block_height, end_block_height (inclusive))
    BlockRequest(u32, u32),
    /// BlockResponse := (block)
    BlockResponse(Data<Block<N>>),
    /// ChallengeRequest := (version, fork_depth, node_type, status, listener_port, nonce, cumulative_weight)
    ChallengeRequest(u32, u32, NodeType, State, u16, u64, u128),
    /// ChallengeResponse := (block_header)
    ChallengeResponse(Data<BlockHeader<N>>),
    /// Disconnect := ()
    Disconnect,
    /// PeerRequest := ()
    PeerRequest,
    /// PeerResponse := (\[peer_ip\])
    PeerResponse(Vec<SocketAddr>),
    /// Ping := (version, fork_depth, node_type, status, block_hash, block_header)
    Ping(u32, u32, NodeType, State, N::BlockHash, Data<BlockHeader<N>>),
    /// Pong := (is_fork, block_locators)
    Pong(Option<bool>, Data<BlockLocators<N>>),
    /// UnconfirmedBlock := (block_height, block_hash, block)
    UnconfirmedBlock(u32, N::BlockHash, Data<Block<N>>),
    /// UnconfirmedTransaction := (transaction)
    UnconfirmedTransaction(Transaction<N>),
    /// GetWork := (address)
    GetWork(Address<N>),
    /// BlockTemplate := (share_difficulty, block_template)
    BlockTemplate(u64, Data<BlockTemplate<N>>),
    /// SendShare := (address, block)
    SendShare(Address<N>, Data<Block<N>>),
    /// Unused
    #[allow(unused)]
    Unused(PhantomData<E>),
}

impl<N: Network, E: Environment> Message<N, E> {
    /// Returns the message name.
    #[inline]
    pub fn name(&self) -> &str {
        match self {
            Self::BlockRequest(..) => "BlockRequest",
            Self::BlockResponse(..) => "BlockResponse",
            Self::ChallengeRequest(..) => "ChallengeRequest",
            Self::ChallengeResponse(..) => "ChallengeResponse",
            Self::Disconnect => "Disconnect",
            Self::PeerRequest => "PeerRequest",
            Self::PeerResponse(..) => "PeerResponse",
            Self::Ping(..) => "Ping",
            Self::Pong(..) => "Pong",
            Self::UnconfirmedBlock(..) => "UnconfirmedBlock",
            Self::UnconfirmedTransaction(..) => "UnconfirmedTransaction",
            Self::GetWork(..) => "GetWork",
            Self::BlockTemplate(..) => "BlockTemplate",
            Self::SendShare(..) => "SendShare",
            Self::Unused(..) => "Unused",
        }
    }

    /// Returns the message ID.
    #[inline]
    pub fn id(&self) -> u16 {
        match self {
            Self::BlockRequest(..) => 0,
            Self::BlockResponse(..) => 1,
            Self::ChallengeRequest(..) => 2,
            Self::ChallengeResponse(..) => 3,
            Self::Disconnect => 4,
            Self::PeerRequest => 5,
            Self::PeerResponse(..) => 6,
            Self::Ping(..) => 7,
            Self::Pong(..) => 8,
            Self::UnconfirmedBlock(..) => 9,
            Self::UnconfirmedTransaction(..) => 10,
            Self::GetWork(..) => 11,
            Self::BlockTemplate(..) => 12,
            Self::SendShare(..) => 13,
            Self::Unused(..) => 14,
        }
    }

    /// Returns the message data as bytes.
    #[inline]
    pub fn data(&self) -> Result<Vec<u8>> {
        match self {
            Self::BlockRequest(start_block_height, end_block_height) => Ok(to_bytes_le![start_block_height, end_block_height]?),
            Self::BlockResponse(block) => Ok(block.serialize_blocking()?),
            Self::ChallengeRequest(version, fork_depth, node_type, status, listener_port, nonce, cumulative_weight) => Ok(
                bincode::serialize(&(version, fork_depth, node_type, status, listener_port, nonce, cumulative_weight))?,
            ),
            Self::ChallengeResponse(block_header) => Ok(block_header.serialize_blocking()?),
            Self::Disconnect => Ok(vec![]),
            Self::PeerRequest => Ok(vec![]),
            Self::PeerResponse(peer_ips) => Ok(bincode::serialize(peer_ips)?),
            Self::Ping(version, fork_depth, node_type, status, block_hash, block_header) => {
                let non_deferred = bincode::serialize(&(version, fork_depth, node_type, status, block_hash))?;
                Ok([non_deferred, block_header.serialize_blocking()?].concat())
            }
            Self::Pong(is_fork, block_locators) => {
                let serialized_is_fork: u8 = match is_fork {
                    None => 0,
                    Some(fork) => match fork {
                        true => 1,
                        false => 2,
                    },
                };

                Ok([vec![serialized_is_fork], block_locators.serialize_blocking()?].concat())
            }
            Self::UnconfirmedBlock(block_height, block_hash, block) => Ok([
                block_height.to_le_bytes().to_vec(),
                block_hash.to_bytes_le()?,
                block.serialize_blocking()?,
            ]
            .concat()),
            Self::UnconfirmedTransaction(transaction) => Ok(bincode::serialize(transaction)?),
            Self::GetWork(address) => Ok(bincode::serialize(address)?),
            Self::BlockTemplate(share_difficulty, block_template) => {
                Ok([bincode::serialize(share_difficulty)?, block_template.serialize_blocking()?].concat())
            }
            Self::SendShare(address, block) => Ok([bincode::serialize(address)?, block.serialize_blocking()?].concat()),
            Self::Unused(_) => Ok(vec![]),
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
        let (id, data) = (u16::from_le_bytes([buffer[0], buffer[1]]), &buffer[2..]);

        // Deserialize the data field.
        let message = match id {
            0 => Self::BlockRequest(bincode::deserialize(&data[0..4])?, bincode::deserialize(&data[4..8])?),
            1 => Self::BlockResponse(Data::Buffer(data.to_vec())),
            2 => {
                let (version, fork_depth, node_type, status, listener_port, nonce, cumulative_weight) = bincode::deserialize(data)?;
                Self::ChallengeRequest(version, fork_depth, node_type, status, listener_port, nonce, cumulative_weight)
            }
            3 => Self::ChallengeResponse(Data::Buffer(data.to_vec())),
            4 => match data.is_empty() {
                true => Self::Disconnect,
                false => return Err(anyhow!("Invalid 'Disconnect' message: {:?} {:?}", buffer, data)),
            },
            5 => match data.is_empty() {
                true => Self::PeerRequest,
                false => return Err(anyhow!("Invalid 'PeerRequest' message: {:?} {:?}", buffer, data)),
            },
            6 => Self::PeerResponse(bincode::deserialize(data)?),
            7 => {
                let (version, fork_depth, node_type, status, block_hash) = bincode::deserialize(&data[0..48])?;
                let block_header = Data::Buffer(data[48..].to_vec());

                Self::Ping(version, fork_depth, node_type, status, block_hash, block_header)
            }
            8 => {
                let is_fork = match data[0] {
                    0 => None,
                    1 => Some(true),
                    2 => Some(false),
                    _ => return Err(anyhow!("Invalid 'Pong' message: {:?} {:?}", buffer, data)),
                };

                Self::Pong(is_fork, Data::Buffer(data[1..].to_vec()))
            }
            9 => Self::UnconfirmedBlock(
                bincode::deserialize(&data[0..4])?,
                bincode::deserialize(&data[4..36])?,
                Data::Buffer(data[36..].to_vec()),
            ),
            10 => Self::UnconfirmedTransaction(bincode::deserialize(data)?),
            11 => Self::GetWork(bincode::deserialize(data)?),
            12 => Self::BlockTemplate(bincode::deserialize(&data[0..8])?, Data::Buffer(data[8..].to_vec())),
            13 => Self::SendShare(bincode::deserialize(&data[0..32])?, Data::Buffer(data[32..].to_vec())),
            _ => return Err(anyhow!("Invalid message ID {}", id)),
        };

        Ok(message)
    }
}

impl<N: Network, E: Environment> Encoder<Message<N, E>> for Message<N, E> {
    type Error = anyhow::Error;

    fn encode(&mut self, message: Message<N, E>, dst: &mut BytesMut) -> Result<(), Self::Error> {
        // Serialize the message into a buffer.
        let buffer = message.serialize()?;

        // Ensure the message does not exceed the maximum length limit.
        if buffer.len() > E::MAXIMUM_MESSAGE_SIZE {
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

impl<N: Network, E: Environment> Decoder for Message<N, E> {
    type Error = std::io::Error;
    type Item = Message<N, E>;

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
        if length > E::MAXIMUM_MESSAGE_SIZE {
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

        // Convert the buffer to a message, or fail if it is not valid.
        let message = match Message::deserialize(&source[4..][..length]) {
            Ok(message) => Ok(Some(message)),
            Err(error) => Err(std::io::Error::new(std::io::ErrorKind::InvalidData, error)),
        };

        // Use `advance` to modify the source such that it no longer contains this frame.
        source.advance(4 + length);

        message
    }
}
