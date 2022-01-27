// Copyright (C) 2019-2022 Aleo Systems Inc.
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

use crate::{
    helpers::{NodeType, State},
    Environment,
};
use snarkos_storage::BlockLocators;
use snarkvm::{dpc::posw::PoSWProof, prelude::*};

use ::bytes::{Buf, BufMut, Bytes, BytesMut};
use anyhow::{anyhow, Result};
use serde::{de::DeserializeOwned, Serialize};
use std::{
    io::{Cursor, Seek, Write},
    marker::PhantomData,
    net::SocketAddr,
};
use tokio::task;
use tokio_util::codec::{Decoder, Encoder};

/// This object enables deferred deserialization / ahead-of-time serialization for objects that
/// take a while to deserialize / serialize, in order to allow these operations to be non-blocking.
#[derive(Clone, Debug)]
pub enum Data<T: 'static + Serialize + DeserializeOwned + Send> {
    Object(T),
    Buffer(Bytes),
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

    pub fn serialize_blocking_into<W: Write>(&self, writer: &mut W) -> Result<()> {
        match self {
            Self::Object(x) => Ok(bincode::serialize_into(writer, x)?),
            Self::Buffer(bytes) => Ok(writer.write_all(bytes)?),
        }
    }

    pub async fn serialize(self) -> bincode::Result<Bytes> {
        match self {
            Self::Object(x) => match task::spawn_blocking(move || bincode::serialize(&x)).await {
                Ok(bytes) => bytes.map(|vec| vec.into()),
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
    UnconfirmedTransaction(Data<Transaction<N>>),
    /// PoolRegister := (address)
    PoolRegister(Address<N>),
    /// PoolRequest := (share_difficulty, block_template)
    PoolRequest(u64, Data<BlockTemplate<N>>),
    /// PoolResponse := (address, nonce, proof)
    PoolResponse(Address<N>, N::PoSWNonce, Data<PoSWProof<N>>),
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
            Self::PoolRegister(..) => "PoolRegister",
            Self::PoolRequest(..) => "PoolRequest",
            Self::PoolResponse(..) => "PoolResponse",
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
            Self::PoolRegister(..) => 11,
            Self::PoolRequest(..) => 12,
            Self::PoolResponse(..) => 13,
            Self::Unused(..) => 14,
        }
    }

    /// Returns the message data as bytes.
    #[inline]
    pub fn serialize_data_into<W: Write>(&self, writer: &mut W) -> Result<()> {
        match self {
            Self::BlockRequest(start_block_height, end_block_height) => {
                let bytes = to_bytes_le![start_block_height, end_block_height]?;
                Ok(writer.write_all(&bytes)?)
            }
            Self::BlockResponse(block) => block.serialize_blocking_into(writer),
            Self::ChallengeRequest(version, fork_depth, node_type, status, listener_port, nonce, cumulative_weight) => {
                Ok(bincode::serialize_into(
                    writer,
                    &(version, fork_depth, node_type, status, listener_port, nonce, cumulative_weight),
                )?)
            }
            Self::ChallengeResponse(block_header) => Ok(block_header.serialize_blocking_into(writer)?),
            Self::Disconnect => Ok(()),
            Self::PeerRequest => Ok(()),
            Self::PeerResponse(peer_ips) => Ok(bincode::serialize_into(writer, peer_ips)?),
            Self::Ping(version, fork_depth, node_type, status, block_hash, block_header) => {
                bincode::serialize_into(&mut *writer, &(version, fork_depth, node_type, status, block_hash))?;
                block_header.serialize_blocking_into(writer)
            }
            Self::Pong(is_fork, block_locators) => {
                let serialized_is_fork: u8 = match is_fork {
                    None => 0,
                    Some(fork) => match fork {
                        true => 1,
                        false => 2,
                    },
                };

                writer.write_all(&[serialized_is_fork])?;
                block_locators.serialize_blocking_into(writer)
            }
            Self::UnconfirmedBlock(block_height, block_hash, block) => {
                writer.write_all(&block_height.to_le_bytes())?;
                writer.write_all(&block_hash.to_bytes_le()?)?;
                block.serialize_blocking_into(writer)
            }
            Self::UnconfirmedTransaction(transaction) => Ok(transaction.serialize_blocking_into(writer)?),
            Self::PoolRegister(address) => Ok(bincode::serialize_into(writer, address)?),
            Self::PoolRequest(share_difficulty, block_template) => {
                bincode::serialize_into(&mut *writer, share_difficulty)?;
                block_template.serialize_blocking_into(writer)
            }
            Self::PoolResponse(address, nonce, proof) => {
                bincode::serialize_into(&mut *writer, address)?;
                bincode::serialize_into(&mut *writer, nonce)?;
                proof.serialize_blocking_into(writer)
            }
            Self::Unused(_) => Ok(()),
        }
    }

    /// Serializes the given message into bytes.
    #[inline]
    pub fn serialize_into<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_all(&self.id().to_le_bytes()[..])?;

        self.serialize_data_into(writer)
    }

    /// Deserializes the given buffer into a message.
    #[inline]
    pub fn deserialize<R: Read + Seek>(reader: &mut R) -> Result<Self> {
        // Read the message ID.
        let id: u16 = bincode::deserialize_from(&mut *reader)?;

        // Helper function to read all the remaining bytes from a reader.
        let read_to_end = |reader: &mut R| -> Result<Bytes> {
            let mut data = vec![];
            reader.read_to_end(&mut data)?;

            Ok(data.into())
        };

        // Deserialize the data field.
        let message = match id {
            0 => Self::BlockRequest(bincode::deserialize_from(&mut *reader)?, bincode::deserialize_from(&mut *reader)?),
            1 => Self::BlockResponse(Data::Buffer(read_to_end(&mut *reader)?)),
            2 => {
                let (version, fork_depth, node_type, status, listener_port, nonce, cumulative_weight) =
                    bincode::deserialize_from(&mut *reader)?;

                Self::ChallengeRequest(version, fork_depth, node_type, status, listener_port, nonce, cumulative_weight)
            }
            3 => Self::ChallengeResponse(Data::Buffer(read_to_end(&mut *reader)?)),
            4 => {
                let data = read_to_end(&mut *reader)?;

                match data.is_empty() {
                    true => Self::Disconnect,
                    false => return Err(anyhow!("Invalid 'Disconnect' message: {:?}", data)),
                }
            }
            5 => {
                let data = read_to_end(&mut *reader)?;

                match data.is_empty() {
                    true => Self::PeerRequest,
                    false => return Err(anyhow!("Invalid 'PeerRequest' message: {:?}", data)),
                }
            }
            6 => Self::PeerResponse(bincode::deserialize_from(&mut *reader)?),
            7 => {
                let (version, fork_depth, node_type, status, block_hash) = bincode::deserialize_from(&mut *reader)?;
                let block_header = Data::Buffer(read_to_end(&mut *reader)?);

                Self::Ping(version, fork_depth, node_type, status, block_hash, block_header)
            }
            8 => {
                let fork_flag: u8 = bincode::deserialize_from(&mut *reader)?;
                let data = read_to_end(&mut *reader)?;

                let is_fork = match fork_flag {
                    0 => None,
                    1 => Some(true),
                    2 => Some(false),
                    _ => return Err(anyhow!("Invalid 'Pong' message: {:?}", data)),
                };

                Self::Pong(is_fork, Data::Buffer(data))
            }
            9 => Self::UnconfirmedBlock(
                bincode::deserialize_from(&mut *reader)?,
                bincode::deserialize_from(&mut *reader)?,
                Data::Buffer(read_to_end(&mut *reader)?),
            ),
            10 => Self::UnconfirmedTransaction(Data::Buffer(read_to_end(&mut *reader)?)),
            11 => Self::PoolRegister(bincode::deserialize_from(&mut *reader)?),
            12 => Self::PoolRequest(bincode::deserialize_from(&mut *reader)?, Data::Buffer(read_to_end(&mut *reader)?)),
            13 => Self::PoolResponse(
                bincode::deserialize_from(&mut *reader)?,
                bincode::deserialize_from(&mut *reader)?,
                Data::Buffer(read_to_end(&mut *reader)?),
            ),
            _ => return Err(anyhow!("Invalid message ID {}", id)),
        };

        Ok(message)
    }
}

impl<N: Network, E: Environment> Encoder<Message<N, E>> for Message<N, E> {
    type Error = anyhow::Error;

    fn encode(&mut self, message: Message<N, E>, dst: &mut BytesMut) -> Result<(), Self::Error> {
        // Prepare the room for the length of the payload.
        dst.extend_from_slice(&0u32.to_le_bytes());

        // Serialize the payload directly into dst.
        message.serialize_into(&mut dst.writer())?;

        // Calculate the length of the serialized payload.
        let len_slice = (dst[4..].len() as u32).to_le_bytes();

        // Overwrite the initial 4B reserved before with the length of the payload.
        dst[..4].copy_from_slice(&len_slice);

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
        let message = match Message::deserialize(&mut Cursor::new(&source[4..][..length])) {
            Ok(message) => Ok(Some(message)),
            Err(error) => Err(std::io::Error::new(std::io::ErrorKind::InvalidData, error)),
        };

        // Use `advance` to modify the source such that it no longer contains this frame.
        source.advance(4 + length);

        message
    }
}
