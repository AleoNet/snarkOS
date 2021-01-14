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

use crate::{
    errors::message::MessageError,
    external::message::{Message, MessageName},
};

#[cfg_attr(nightly, doc(include = "../../../documentation/network_messages/get_memory_pool.md"))]
#[derive(Debug, PartialEq, Clone)]
pub struct GetMemoryPool;

impl Message for GetMemoryPool {
    fn name() -> MessageName {
        MessageName::from("getmempool")
    }

    fn deserialize(vec: Vec<u8>) -> Result<Self, MessageError> {
        if !vec.is_empty() {
            return Err(MessageError::InvalidLength(vec.len(), 0));
        }

        Ok(Self)
    }

    fn serialize(&self) -> Result<Vec<u8>, MessageError> {
        Ok(vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_memory_pool() {
        let message = GetMemoryPool;

        let serialized = message.serialize().unwrap();
        let deserialized = GetMemoryPool::deserialize(serialized).unwrap();

        assert_eq!(message, deserialized);
    }
}
