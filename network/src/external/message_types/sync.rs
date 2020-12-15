// Copyright (C) 2019-2020 Aleo Systems Inc.
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

use crate::external::message::{Message, MessageName};
use snarkos_errors::network::message::MessageError;
use snarkos_objects::BlockHeaderHash;

#[cfg_attr(nightly, doc(include = "../../../documentation/network_messages/sync.md"))]
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Sync {
    /// Known hashes of blocks to share
    pub block_hashes: Vec<BlockHeaderHash>,
}

impl Sync {
    pub fn new(block_hashes: Vec<BlockHeaderHash>) -> Self {
        Self { block_hashes }
    }
}

impl Message for Sync {
    fn name() -> MessageName {
        MessageName::from("sync")
    }

    fn deserialize(bytes: &[u8]) -> Result<Self, MessageError> {
        Ok(Self {
            block_hashes: bincode::deserialize(bytes)?,
        })
    }

    fn serialize(&self) -> Result<Vec<u8>, MessageError> {
        Ok(bincode::serialize(&self.block_hashes)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use snarkos_testing::consensus::BLOCK_1_HEADER_HASH;

    #[test]
    fn test_sync() {
        let data = BlockHeaderHash::new(BLOCK_1_HEADER_HASH.to_vec());
        let message = Sync::new(vec![data]);

        let serialized = message.serialize().unwrap();
        let deserialized = Sync::deserialize(serialized).unwrap();

        assert_eq!(message, deserialized);
    }
}
