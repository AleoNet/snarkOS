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

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct BlockRequest {
    /// The starting block height (inclusive).
    pub start_height: u32,
    /// The ending block height (exclusive).
    pub end_height: u32,
}

impl MessageTrait for BlockRequest {
    /// Returns the message name.
    #[inline]
    fn name(&self) -> Cow<'static, str> {
        let start = self.start_height;
        let end = self.end_height;
        match start + 1 == end {
            true => format!("BlockRequest {start}"),
            false => format!("BlockRequest {start}..{end}"),
        }
        .into()
    }

    /// Serializes the message into the buffer.
    #[inline]
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<()> {
        Ok(bincode::serialize_into(writer, &(self.start_height, self.end_height))?)
    }

    /// Deserializes the given buffer into a message.
    #[inline]
    fn deserialize(bytes: BytesMut) -> Result<Self> {
        let mut reader = bytes.reader();
        Ok(Self {
            start_height: bincode::deserialize_from(&mut reader)?,
            end_height: bincode::deserialize_from(&mut reader)?,
        })
    }
}

impl Display for BlockRequest {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}..{}", self.start_height, self.end_height)
    }
}
