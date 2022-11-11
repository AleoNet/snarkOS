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
pub struct BlockResponse<N: Network> {
    pub blocks: Data<Blocks<N>>,
}

impl<N: Network> BlockResponse<N> {
    pub fn new(blocks: Vec<Block<N>>) -> Self {
        Self { blocks: Data::Object(Blocks(blocks)) }
    }
}

impl<N: Network> MessageTrait for BlockResponse<N> {
    /// Returns the message name.
    #[inline]
    fn name(&self) -> &str {
        "BlockResponse"
    }

    /// Serializes the message into the buffer.
    #[inline]
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<()> {
        self.blocks.serialize_blocking_into(writer)
    }

    /// Deserializes the given buffer into a message.
    #[inline]
    fn deserialize(bytes: BytesMut) -> Result<Self> {
        Ok(Self { blocks: Data::Buffer(bytes.freeze()) })
    }
}

/// Wrapper struct around a vector of blocks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Blocks<N: Network>(pub Vec<Block<N>>);

impl<N: Network> ToBytes for Blocks<N> {
    #[inline]
    fn write_le<W: Write>(&self, mut writer: W) -> IoResult<()> {
        (self.0.len() as u32).write_le(&mut writer)?;
        for block in &self.0 {
            block.write_le(&mut writer)?;
        }

        Ok(())
    }
}

impl<N: Network> FromBytes for Blocks<N> {
    #[inline]
    fn read_le<R: Read>(mut reader: R) -> IoResult<Self> {
        let num_blocks = u32::read_le(&mut reader)? as usize;
        let mut blocks = Vec::with_capacity(num_blocks);
        for _ in 0..num_blocks {
            blocks.push(Block::read_le(&mut reader)?);
        }

        Ok(Self(blocks))
    }
}
