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
