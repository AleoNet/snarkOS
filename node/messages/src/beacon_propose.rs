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

use super::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BeaconPropose<N: Network> {
    pub version: u8,
    pub round: u64,
    pub block_height: u32,
    pub block_hash: N::BlockHash,
    pub block: Data<Block<N>>,
}

impl<N: Network> BeaconPropose<N> {
    /// The current version of this message.
    pub const INTERNAL_VERSION: u8 = 0;

    /// Initializes a new message.
    pub const fn new(round: u64, block_height: u32, block_hash: N::BlockHash, block: Data<Block<N>>) -> Self {
        Self { version: Self::INTERNAL_VERSION, round, block_height, block_hash, block }
    }
}

impl<N: Network> MessageTrait for BeaconPropose<N> {
    /// Returns the message name.
    #[inline]
    fn name(&self) -> String {
        format!("BeaconPropose {}", self.block_height)
    }

    /// Serializes the message into the buffer.
    #[inline]
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_all(&self.version.to_le_bytes())?;
        writer.write_all(&self.round.to_le_bytes())?;
        writer.write_all(&self.block_height.to_le_bytes())?;
        writer.write_all(&self.block_hash.to_bytes_le()?)?;
        self.block.serialize_blocking_into(writer)
    }

    /// Deserializes the given buffer into a message.
    #[inline]
    fn deserialize(bytes: BytesMut) -> Result<Self> {
        let mut reader = bytes.reader();
        Ok(Self {
            version: bincode::deserialize_from(&mut reader)?,
            round: bincode::deserialize_from(&mut reader)?,
            block_height: bincode::deserialize_from(&mut reader)?,
            block_hash: N::BlockHash::read_le(&mut reader)?,
            block: Data::Buffer(reader.into_inner().freeze()),
        })
    }
}
