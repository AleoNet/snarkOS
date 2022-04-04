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
    helpers::{BlockLocators, NodeType, State},
    Environment,
};
use snarkvm::{dpc::posw::PoSWProof, prelude::*};

use ::bytes::{Buf, BufMut, Bytes, BytesMut};
use anyhow::{bail, Result};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{io::Write, marker::PhantomData, net::SocketAddr, time::Instant};
use tokio::task;
use tokio_util::codec::{Decoder, Encoder, LengthDelimitedCodec};

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

/// The reason behind the node disconnecting from a peer.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub enum DisconnectReason {
    /// The fork length limit was exceeded.
    ExceededForkRange,
    /// The peer's client uses an invalid fork depth.
    InvalidForkDepth,
    /// The node is a sync node and the peer is ahead.
    INeedToSyncFirst,
    /// No reason given.
    NoReasonGiven,
    /// The peer's client is outdated, judging by its version.
    OutdatedClientVersion,
    /// Dropping a dead connection.
    PeerHasDisconnected,
    /// The node is shutting down.
    ShuttingDown,
    /// The sync node has served its purpose.
    SyncComplete,
    /// The peer has caused too many failures.
    TooManyFailures,
    /// The node has too many connections already.
    TooManyPeers,
    /// The peer is a sync node that's behind our node, and it needs to sync itself first.
    YouNeedToSyncFirst,
    /// The peer's listening port is closed.
    YourPortIsClosed(u16),
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
    Disconnect(DisconnectReason),
    /// PeerRequest := ()
    PeerRequest,
    /// PeerResponse := (\[peer_ip\])
    PeerResponse(Vec<SocketAddr>, Option<Instant>),
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
            Self::Disconnect(..) => "Disconnect",
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
            Self::Disconnect(..) => 4,
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
            Self::Disconnect(reason) => Ok(bincode::serialize_into(writer, reason)?),
            Self::PeerRequest => Ok(()),
            Self::PeerResponse(peer_ips, _) => Ok(bincode::serialize_into(writer, peer_ips)?),
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
    pub fn deserialize(mut bytes: BytesMut) -> Result<Self> {
        // Ensure there is at least a message ID in the buffer.
        if bytes.remaining() < 2 {
            bail!("Missing message ID");
        }

        // Read the message ID.
        let id: u16 = bytes.get_u16_le();

        // Deserialize the data field.
        let message = match id {
            0 => {
                let mut reader = bytes.reader();
                Self::BlockRequest(bincode::deserialize_from(&mut reader)?, bincode::deserialize_from(&mut reader)?)
            }
            1 => Self::BlockResponse(Data::Buffer(bytes.freeze())),
            2 => {
                let (version, fork_depth, node_type, status, listener_port, nonce, cumulative_weight) =
                    bincode::deserialize_from(&mut bytes.reader())?;

                Self::ChallengeRequest(version, fork_depth, node_type, status, listener_port, nonce, cumulative_weight)
            }
            3 => Self::ChallengeResponse(Data::Buffer(bytes.freeze())),
            4 => {
                if bytes.remaining() == 0 {
                    Self::Disconnect(DisconnectReason::NoReasonGiven)
                } else if let Ok(reason) = bincode::deserialize_from(&mut bytes.reader()) {
                    Self::Disconnect(reason)
                } else {
                    bail!("Invalid 'Disconnect' message");
                }
            }
            5 => match bytes.remaining() == 0 {
                true => Self::PeerRequest,
                false => bail!("Invalid 'PeerRequest' message"),
            },
            6 => Self::PeerResponse(bincode::deserialize_from(&mut bytes.reader())?, None),
            7 => {
                let mut reader = bytes.reader();
                let (version, fork_depth, node_type, status, block_hash) = bincode::deserialize_from(&mut reader)?;
                let block_header = Data::Buffer(reader.into_inner().freeze());

                Self::Ping(version, fork_depth, node_type, status, block_hash, block_header)
            }
            8 => {
                // Make sure a byte for the fork flag is available.
                if bytes.remaining() == 0 {
                    bail!("Missing fork flag in a 'Pong'");
                }

                let fork_flag = bytes.get_u8();

                let is_fork = match fork_flag {
                    0 => None,
                    1 => Some(true),
                    2 => Some(false),
                    _ => bail!("Invalid 'Pong' message"),
                };

                Self::Pong(is_fork, Data::Buffer(bytes.freeze()))
            }
            9 => {
                let mut reader = bytes.reader();
                Self::UnconfirmedBlock(
                    bincode::deserialize_from(&mut reader)?,
                    bincode::deserialize_from(&mut reader)?,
                    Data::Buffer(reader.into_inner().freeze()),
                )
            }
            10 => Self::UnconfirmedTransaction(Data::Buffer(bytes.freeze())),
            11 => Self::PoolRegister(bincode::deserialize_from(&mut bytes.reader())?),
            12 => {
                let mut reader = bytes.reader();
                Self::PoolRequest(bincode::deserialize_from(&mut reader)?, Data::Buffer(reader.into_inner().freeze()))
            }
            13 => {
                let mut reader = bytes.reader();
                Self::PoolResponse(
                    bincode::deserialize_from(&mut reader)?,
                    bincode::deserialize_from(&mut reader)?,
                    Data::Buffer(reader.into_inner().freeze()),
                )
            }
            _ => bail!("Invalid message ID {}", id),
        };

        Ok(message)
    }
}

/// The codec used to decode and encode network `Message`s.
pub struct MessageCodec<N: Network, E: Environment> {
    codec: LengthDelimitedCodec,
    _phantoms: (PhantomData<N>, PhantomData<E>),
}

impl<N: Network, E: Environment> Default for MessageCodec<N, E> {
    fn default() -> Self {
        Self {
            codec: LengthDelimitedCodec::builder()
                .max_frame_length(E::MAXIMUM_MESSAGE_SIZE)
                .little_endian()
                .new_codec(),
            _phantoms: Default::default(),
        }
    }
}

impl<N: Network, E: Environment> Encoder<Message<N, E>> for MessageCodec<N, E> {
    type Error = io::Error;

    fn encode(&mut self, message: Message<N, E>, dst: &mut BytesMut) -> Result<(), Self::Error> {
        // Serialize the payload directly into dst.
        message
            .serialize_into(&mut dst.writer())
            // This error should never happen, the conversion is for greater compatibility.
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "serialization error"))?;

        let serialized_message = dst.split_to(dst.len()).freeze();

        self.codec.encode(serialized_message, dst)
    }
}

impl<N: Network, E: Environment> Decoder for MessageCodec<N, E> {
    type Error = std::io::Error;
    type Item = Message<N, E>;

    fn decode(&mut self, source: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        // Decode a frame containing bytes belonging to a message.
        let bytes = match self.codec.decode(source)? {
            Some(bytes) => bytes,
            None => return Ok(None),
        };

        // Convert the bytes to a message, or fail if it is not valid.
        match Message::deserialize(bytes) {
            Ok(message) => Ok(Some(message)),
            Err(error) => {
                error!("Failed to deserialize a message: {}", error);
                Err(io::ErrorKind::InvalidData.into())
            }
        }
    }
}
