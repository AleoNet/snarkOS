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

use bincode::Options;
use std::borrow::Cow;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ping<N: Network> {
    pub version: u32,
    pub node_type: NodeType,
    pub block_locators: Option<BlockLocators<N>>,
}

impl<N: Network> MessageTrait for Ping<N> {
    /// Returns the message name.
    #[inline]
    fn name(&self) -> Cow<'static, str> {
        "Ping".into()
    }

    /// Serializes the message into the buffer.
    #[inline]
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<()> {
        Ok(bincode::serialize_into(&mut *writer, &(self.version, self.node_type, &self.block_locators))?)
    }

    /// Deserializes the given buffer into a message.
    #[inline]
    fn deserialize(bytes: BytesMut) -> Result<Self> {
        let options =
            bincode::options().with_limit(MAXIMUM_MESSAGE_SIZE as u64).with_fixint_encoding().allow_trailing_bytes();
        let mut reader = bytes.reader();
        let (version, node_type, block_locators) = options.deserialize_from(&mut reader)?;
        Ok(Self { version, node_type, block_locators })
    }
}

impl<N: Network> Ping<N> {
    pub fn new(node_type: NodeType, block_locators: Option<BlockLocators<N>>) -> Self {
        Self { version: <Message<N>>::VERSION, node_type, block_locators }
    }
}
