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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BeaconTimeout<N: Network> {
    pub version: u8,
    pub round: u64,
    pub block_height: u32,
    pub block_hash: N::BlockHash,
    pub signature: Data<Signature<N>>,
}

impl<N: Network> BeaconTimeout<N> {
    /// The current version of this message.
    pub const INTERNAL_VERSION: u8 = 0;

    /// Initializes a new message.
    pub const fn new(round: u64, block_height: u32, block_hash: N::BlockHash, signature: Data<Signature<N>>) -> Self {
        Self { version: Self::INTERNAL_VERSION, round, block_height, block_hash, signature }
    }
}

impl<N: Network> MessageTrait for BeaconTimeout<N> {
    /// Returns the message name.
    #[inline]
    fn name(&self) -> String {
        format!("BeaconTimeout {}", self.block_height)
    }

    /// Serializes the message into the buffer.
    #[inline]
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_all(&self.version.to_le_bytes())?;
        writer.write_all(&self.round.to_le_bytes())?;
        writer.write_all(&self.block_height.to_le_bytes())?;
        writer.write_all(&self.block_hash.to_bytes_le()?)?;
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
            signature: Data::Buffer(reader.into_inner().freeze()),
        })
    }
}
