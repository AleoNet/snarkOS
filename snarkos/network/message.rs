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

use snarkvm::prelude::*;

use ::bytes::{Buf, BufMut, Bytes, BytesMut};
use std::marker::PhantomData;
use tokio::task;
use tokio_util::codec::{Decoder, Encoder, LengthDelimitedCodec};

/// This object enables deferred deserialization / ahead-of-time serialization for objects that
/// take a while to deserialize / serialize, in order to allow these operations to be non-blocking.
#[derive(Clone, Debug)]
pub enum Data<T: FromBytes + ToBytes + Send + 'static> {
    Object(T),
    Buffer(Bytes),
}

impl<T: FromBytes + ToBytes + Send + 'static> Data<T> {
    pub fn deserialize_blocking(self) -> Result<T> {
        match self {
            Self::Object(x) => Ok(x),
            Self::Buffer(bytes) => T::from_bytes_le(&bytes),
        }
    }

    pub async fn deserialize(self) -> Result<T> {
        match self {
            Self::Object(x) => Ok(x),
            Self::Buffer(bytes) => match task::spawn_blocking(move || T::from_bytes_le(&bytes)).await {
                Ok(x) => x,
                Err(err) => Err(err.into()),
            },
        }
    }

    pub fn serialize_blocking_into<W: Write>(&self, writer: &mut W) -> Result<()> {
        match self {
            Self::Object(x) => {
                let bytes = x.to_bytes_le()?;
                Ok(writer.write_all(&bytes)?)
            }
            Self::Buffer(bytes) => Ok(writer.write_all(bytes)?),
        }
    }

    pub async fn serialize(self) -> Result<Bytes> {
        match self {
            Self::Object(x) => match task::spawn_blocking(move || x.to_bytes_le()).await {
                Ok(bytes) => bytes.map(|vec| vec.into()),
                Err(err) => Err(err.into()),
            },
            Self::Buffer(bytes) => Ok(bytes),
        }
    }
}

#[derive(Clone)]
pub enum Message<N: Network> {
    /// Ping with the current block height.
    Ping,
    /// Pong with the current block height.
    Pong(u32),
    /// Request a block for a given height.
    BlockRequest(u32),
    /// A response to a `BlockRequest`.
    BlockResponse(Data<Block<N>>),
    /// A message containing a transaction to be broadcast.
    TransactionBroadcast(Data<Transaction<N>>),
    /// A message containing a new block to be broadcast.
    BlockBroadcast(Data<Block<N>>),
}

impl<N: Network> Message<N> {
    /// Returns the message name.
    #[inline]
    pub fn name(&self) -> &str {
        match self {
            Self::Ping => "Ping",
            Self::Pong(..) => "Pong",
            Self::BlockRequest(..) => "BlockRequest",
            Self::BlockResponse(..) => "BlockResponse",
            Self::TransactionBroadcast(..) => "TransactionBroadcast",
            Self::BlockBroadcast(..) => "BlockBroadcast",
        }
    }

    /// Returns the message ID.
    #[inline]
    pub fn id(&self) -> u16 {
        match self {
            Self::Ping => 0,
            Self::Pong(..) => 1,
            Self::BlockRequest(..) => 2,
            Self::BlockResponse(..) => 3,
            Self::TransactionBroadcast(..) => 4,
            Self::BlockBroadcast(..) => 5,
        }
    }

    /// Returns the message data as bytes.
    #[inline]
    fn serialize_data_into<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_all(&self.id().to_le_bytes()[..])?;

        match self {
            Self::Ping => Ok(()),
            Self::Pong(block_height) => Ok(writer.write_all(&block_height.to_le_bytes())?),
            Self::BlockRequest(block_height) => Ok(writer.write_all(&block_height.to_le_bytes())?),
            Self::BlockResponse(block) | Self::BlockBroadcast(block) => block.serialize_blocking_into(writer),
            Self::TransactionBroadcast(transaction) => transaction.serialize_blocking_into(writer),
        }
    }

    /// Deserialize the given buffer into a message.
    fn deserialize(mut bytes: BytesMut) -> Result<Self> {
        if bytes.remaining() < 2 {
            bail!("Missing message ID");
        }

        // Read the message ID.
        let id: u16 = bytes.get_u16_le();

        // Deserialize the data field.
        let message = match id {
            0 => {
                if bytes.remaining() != 0 {
                    bail!("Unexpected data for Ping");
                }
                Message::<N>::Ping
            }
            1 => {
                let mut reader = bytes.reader();
                Message::<N>::Pong(bincode::deserialize_from(&mut reader)?)
            }
            2 => {
                let mut reader = bytes.reader();
                Message::<N>::BlockRequest(bincode::deserialize_from(&mut reader)?)
            }
            3 => Message::<N>::BlockResponse(Data::Buffer(bytes.freeze())),
            4 => Message::<N>::TransactionBroadcast(Data::Buffer(bytes.freeze())),
            5 => Message::<N>::BlockBroadcast(Data::Buffer(bytes.freeze())),
            _ => bail!("Unknown message ID"),
        };

        Ok(message)
    }
}

/// The maximum size of a message that can be transmitted in the network.
const MAXIMUM_MESSAGE_SIZE: usize = 128 * 1024 * 1024; // 128 MiB

/// The codec used to decode and encode network `Message`s.
pub struct MessageCodec<N: Network> {
    codec: LengthDelimitedCodec,
    _phantom: PhantomData<N>,
}

impl<N: Network> Default for MessageCodec<N> {
    fn default() -> Self {
        Self {
            codec: LengthDelimitedCodec::builder()
                .max_frame_length(MAXIMUM_MESSAGE_SIZE)
                .little_endian()
                .new_codec(),
            _phantom: Default::default(),
        }
    }
}

impl<N: Network> Encoder<Message<N>> for MessageCodec<N> {
    type Error = std::io::Error;

    fn encode(&mut self, message: Message<N>, dst: &mut BytesMut) -> Result<(), Self::Error> {
        // Serialize the payload directly into dst.
        message
            .serialize_data_into(&mut dst.writer())
            // This error should never happen, the conversion is for greater compatibility.
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidData, "serialization error"))?;

        let serialized_message = dst.split_to(dst.len()).freeze();

        self.codec.encode(serialized_message, dst)
    }
}

impl<N: Network> Decoder for MessageCodec<N> {
    type Error = std::io::Error;
    type Item = Message<N>;

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
                warn!("Failed to deserialize a message: {}", error);
                Err(std::io::ErrorKind::InvalidData.into())
            }
        }
    }
}
