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

use crate::outbound::message::{Message, MessageName};
use snarkos_errors::network::message::MessageError;
use snarkos_objects::BlockHeaderHash;

#[cfg_attr(nightly, doc(include = "../../documentation/network_messages/get_block.md"))]
#[derive(Debug, PartialEq, Clone)]
pub struct GetBlock {
    /// Header hash of requested block
    pub block_hash: BlockHeaderHash,
}

impl GetBlock {
    pub fn new(block_hash: BlockHeaderHash) -> Self {
        Self { block_hash }
    }
}

impl Message for GetBlock {
    fn name() -> MessageName {
        MessageName::from("getblock")
    }

    fn deserialize(vec: Vec<u8>) -> Result<Self, MessageError> {
        Ok(Self {
            block_hash: bincode::deserialize(&vec)?,
        })
    }

    fn serialize(&self) -> Result<Vec<u8>, MessageError> {
        Ok(bincode::serialize(&self.block_hash)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use snarkos_testing::consensus::BLOCK_1_HEADER_HASH;

    #[test]
    fn test_block() {
        let block_hash = BlockHeaderHash::new(BLOCK_1_HEADER_HASH.to_vec());
        let message = GetBlock::new(block_hash);

        let serialized = message.serialize().unwrap();
        let deserialized = GetBlock::deserialize(serialized).unwrap();

        assert_eq!(message, deserialized);
    }
}
