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

#[cfg_attr(nightly, doc(include = "../../../documentation/network_messages/block.md"))]
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Block {
    /// Serialized block data
    pub data: Vec<u8>,
}

impl Block {
    pub fn new(data: Vec<u8>) -> Self {
        Self { data }
    }
}

impl Message for Block {
    fn name() -> MessageName {
        MessageName::from("block")
    }

    fn deserialize(bytes: &[u8]) -> Result<Self, MessageError> {
        Ok(Self {
            data: bincode::deserialize(bytes)?,
        })
    }

    fn serialize(&self) -> Result<Vec<u8>, MessageError> {
        Ok(bincode::serialize(&self.data)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use snarkos_testing::consensus::BLOCK_1;

    #[test]
    fn test_block() {
        let message = Block::new(BLOCK_1.to_vec());

        let serialized = message.serialize().unwrap();
        let deserialized = Block::deserialize(&serialized).unwrap();

        assert_eq!(message, deserialized);
    }
}
