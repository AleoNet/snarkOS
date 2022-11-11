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
    pub blocks: Vec<Data<Block<N>>>,
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
        writer.write_all(&(self.blocks.len() as u32).to_bytes_le()?)?;
        for block in &self.blocks {
            block.serialize_blocking_into(writer)?;
        }

        Ok(())
    }

    /// Deserializes the given buffer into a message.
    #[inline]
    fn deserialize(mut bytes: BytesMut) -> Result<Self> {
        let num_blocks: u32 = bytes.get_u32();

        let mut blocks = Vec::with_capacity(num_blocks as usize);

        for _ in 0..num_blocks {
            blocks.push(Data::Object(Block::<N>::from_bytes_le(&bytes)?));
        }

        Ok(Self { blocks })
    }
}
