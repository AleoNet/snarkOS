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
pub struct BeaconVote<N: Network> {
    pub version: u8,
    pub round: u64,
    pub block_height: u32,
    pub block_hash: N::BlockHash,
    pub timestamp: u64,
    pub signature: Data<Signature<N>>,
}

impl<N: Network> BeaconVote<N> {
    /// The current version of this message.
    pub const INTERNAL_VERSION: u8 = 0;

    /// Initializes a new message.
    pub const fn new(
        round: u64,
        block_height: u32,
        block_hash: N::BlockHash,
        timestamp: u64,
        signature: Data<Signature<N>>,
    ) -> Self {
        Self { version: Self::INTERNAL_VERSION, round, block_height, block_hash, timestamp, signature }
    }
}

impl<N: Network> MessageTrait for BeaconVote<N> {
    /// Returns the message name.
    #[inline]
    fn name(&self) -> Cow<'static, str> {
        format!("BeaconVote {}", self.block_height).into()
    }

    /// Serializes the message into the buffer.
    #[inline]
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<()> {
        self.version.write_le(&mut *writer)?;
        self.round.write_le(&mut *writer)?;
        self.block_height.write_le(&mut *writer)?;
        self.block_hash.write_le(&mut *writer)?;
        self.timestamp.write_le(&mut *writer)?;
        self.signature.serialize_blocking_into(writer)
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
            timestamp: bincode::deserialize_from(&mut reader)?,
            signature: Data::Buffer(reader.into_inner().freeze()),
        })
    }
}
