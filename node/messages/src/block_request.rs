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
pub struct BlockRequest {
    pub start_block_height: u32,
    pub end_block_height: u32,
}

impl MessageTrait for BlockRequest {
    /// Returns the message name.
    #[inline]
    fn name(&self) -> &str {
        "BlockRequest"
    }

    /// Serializes the message into the buffer.
    #[inline]
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<()> {
        Ok(bincode::serialize_into(writer, &(self.start_block_height, self.end_block_height))?)
    }

    /// Deserializes the given buffer into a message.
    #[inline]
    fn deserialize(bytes: BytesMut) -> Result<Self> {
        let mut reader = bytes.reader();
        Ok(Self {
            start_block_height: bincode::deserialize_from(&mut reader)?,
            end_block_height: bincode::deserialize_from(&mut reader)?,
        })
    }
}
