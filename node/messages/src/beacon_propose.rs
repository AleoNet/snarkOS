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
    fn name(&self) -> Cow<'static, str> {
        format!("BeaconPropose {}", self.block_height).into()
    }

    /// Serializes the message into the buffer.
    #[inline]
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<()> {
        self.version.write_le(&mut *writer)?;
        self.round.write_le(&mut *writer)?;
        self.block_height.write_le(&mut *writer)?;
        self.block_hash.write_le(&mut *writer)?;
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
