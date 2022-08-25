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

use ::bytes::{Buf, BufMut, BytesMut};
use std::marker::PhantomData;
use tokio_util::codec::{Decoder, Encoder, LengthDelimitedCodec};

pub enum Message<N: Network> {
    /// Ping with the current block height.
    Ping,
    /// Pong with the current block height.
    Pong(u32),
    /// Request a block for a given height.
    BlockRequest(u32),
    /// A response to a `BlockRequest`.
    BlockResponse(Block<N>),
    /// A message containing a transaction to be broadcast.
    TransactionBroadcast(Transaction<N>),
    /// A message containing a new block to be broadcast.
    BlockBroadcast(Block<N>),
    // TODO (raychu86): Send an actual coinbase puzzle object.
    /// A message containing a coinbase puzzle for the given block height.
    CoinbasePuzzle(u32),
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
            Self::CoinbasePuzzle(..) => "BlockBroadcast",
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
            Self::CoinbasePuzzle(..) => 5,
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
            Self::BlockResponse(block) => Ok(writer.write_all(&block.to_bytes_le()?)?),
            Self::TransactionBroadcast(transaction) => Ok(writer.write_all(&transaction.to_bytes_le()?)?),
            Self::BlockBroadcast(block) => Ok(writer.write_all(&block.to_bytes_le()?)?),
            Self::CoinbasePuzzle(block_height) => Ok(writer.write_all(&block_height.to_bytes_le()?)?),
        }
    }

    /// Deserialize the given buffer into a message.
    fn deserialize(mut bytes: BytesMut) -> Result<Self> {
        if bytes.remaining() < 1 {
            bail!("Missing message ID");
        }

        // Read the message ID.
        let id: u16 = bytes.get_u16_le();

        // Deserialize the data field.

        match id {
            0 => {
                if bytes.remaining() != 0 {
                    bail!("Unexpected data for Ping");
                }
                Ok(Message::<N>::Ping)
            }
            1 => {
                let mut reader = bytes.reader();
                let message = Message::<N>::Pong(bincode::deserialize_from(&mut reader)?);
                Ok(message)
            }
            2 => {
                let mut reader = bytes.reader();
                let message = Message::<N>::BlockRequest(bincode::deserialize_from(&mut reader)?);
                Ok(message)
            }
            3 => {
                let mut reader = bytes.reader();
                let message = Message::<N>::BlockResponse(Block::read_le(&mut reader)?);
                Ok(message)
            }
            4 => {
                let mut reader = bytes.reader();
                let message = Message::<N>::TransactionBroadcast(Transaction::read_le(&mut reader)?);
                Ok(message)
            }
            5 => {
                let mut reader = bytes.reader();
                let message = Message::<N>::BlockBroadcast(Block::read_le(&mut reader)?);
                Ok(message)
            }
            6 => {
                let mut reader = bytes.reader();
                let message = Message::<N>::CoinbasePuzzle(bincode::deserialize_from(&mut reader)?);
                Ok(message)
            }
            _ => bail!("Unknown message ID"),
        }
    }
}

impl<N: Network> FromBytes for Message<N> {
    /// Reads the message from a buffer.
    fn read_le<R: Read>(mut reader: R) -> IoResult<Self> {
        let id = u16::read_le(&mut reader)?;

        let message = match id {
            0 => Self::Ping,
            1 => Self::Pong(u32::read_le(&mut reader)?),
            2 => Self::BlockRequest(u32::read_le(&mut reader)?),
            3 => Self::BlockResponse(Block::read_le(&mut reader)?),
            4 => Self::TransactionBroadcast(Transaction::read_le(&mut reader)?),
            5 => Self::BlockBroadcast(Block::read_le(&mut reader)?),
            6 => Self::CoinbasePuzzle(u32::read_le(&mut reader)?),
            7.. => return Err(error(format!("Failed to decode message id {id}"))),
        };

        Ok(message)
    }
}

impl<N: Network> ToBytes for Message<N> {
    /// Writes the message to a buffer.
    fn write_le<W: Write>(&self, mut writer: W) -> IoResult<()> {
        self.id().write_le(&mut writer)?;

        match self {
            Message::Ping => Ok(()),
            Message::Pong(height) => height.write_le(&mut writer),
            Message::BlockRequest(height) => height.write_le(&mut writer),
            Message::BlockResponse(block) => block.write_le(&mut writer),
            Message::TransactionBroadcast(transaction) => transaction.write_le(&mut writer),
            Message::BlockBroadcast(block) => block.write_le(&mut writer),
            Message::CoinbasePuzzle(block_height) => block_height.write_le(&mut writer),
        }
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
