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

#[cfg_attr(nightly, doc(include = "../../../documentation/network_messages/get_peers.md"))]
#[derive(Debug, PartialEq, Clone)]
pub struct GetPeers;

impl Message for GetPeers {
    fn name() -> MessageName {
        MessageName::from("getpeers")
    }

    fn deserialize(vec: Vec<u8>) -> Result<Self, MessageError> {
        if vec.len() != 0 {
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
    fn test_getpeers() {
        let message = GetPeers;

        let serialized = message.serialize().unwrap();
        let deserialized = GetPeers::deserialize(serialized).unwrap();

        assert_eq!(message, deserialized);
    }
}
