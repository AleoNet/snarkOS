// Copyright (C) 2019-2023 Aleo Systems Inc.
// This file is part of the snarkOS library.

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at:
// http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use super::*;

use std::borrow::Cow;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockResponse<N: Network> {
    /// The original block request.
    pub request: BlockRequest,
    /// The blocks.
    pub blocks: Data<DataBlocks<N>>,
}

impl<N: Network> MessageTrait for BlockResponse<N> {
    /// Returns the message name.
    #[inline]
    fn name(&self) -> Cow<'static, str> {
        let start = self.request.start_height;
        let end = self.request.end_height;
        match start + 1 == end {
            true => format!("BlockResponse {start}"),
            false => format!("BlockResponse {start}..{end}"),
        }
        .into()
    }

    /// Serializes the message into the buffer.
    #[inline]
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<()> {
        self.request.serialize(writer)?;
        self.blocks.serialize_blocking_into(writer)
    }

    /// Deserializes the given buffer into a message.
    #[inline]
    fn deserialize(bytes: BytesMut) -> Result<Self> {
        let mut reader = bytes.reader();
        let request = BlockRequest {
            start_height: bincode::deserialize_from(&mut reader)?,
            end_height: bincode::deserialize_from(&mut reader)?,
        };
        let blocks = Data::Buffer(reader.into_inner().freeze());
        Ok(Self { request, blocks })
    }
}

/// A wrapper for a list of blocks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DataBlocks<N: Network>(pub Vec<Block<N>>);

impl<N: Network> DataBlocks<N> {
    /// TODO (howardwu): Evaluate the merits of multi-block requests in the new sync model.
    /// The maximum number of blocks that can be sent in a single message.
    pub const MAXIMUM_NUMBER_OF_BLOCKS: u8 = 1;
}

impl<N: Network> Deref for DataBlocks<N> {
    type Target = Vec<Block<N>>;

    /// Returns the list of blocks.
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<N: Network> ToBytes for DataBlocks<N> {
    /// Writes the blocks to the given writer.
    #[inline]
    fn write_le<W: Write>(&self, mut writer: W) -> IoResult<()> {
        // Prepare the number of blocks.
        let num_blocks = self.0.len() as u8;
        // Ensure that the number of blocks is within the allowed range.
        if num_blocks > Self::MAXIMUM_NUMBER_OF_BLOCKS {
            return Err(error("Block response exceeds maximum number of blocks"));
        }
        // Write the number of blocks.
        num_blocks.write_le(&mut writer)?;
        // Write the blocks.
        self.0.iter().take(num_blocks as usize).try_for_each(|block| block.write_le(&mut writer))
    }
}

impl<N: Network> FromBytes for DataBlocks<N> {
    /// Reads the message from the given reader.
    #[inline]
    fn read_le<R: Read>(mut reader: R) -> IoResult<Self> {
        // Read the number of blocks.
        let num_blocks = u8::read_le(&mut reader)?;
        // Ensure that the number of blocks is within the allowed range.
        if num_blocks > Self::MAXIMUM_NUMBER_OF_BLOCKS {
            return Err(error("Block response exceeds maximum number of blocks"));
        }
        // Read the blocks.
        let blocks = (0..num_blocks).map(|_| Block::read_le(&mut reader)).collect::<Result<Vec<_>, _>>()?;
        Ok(Self(blocks))
    }
}
