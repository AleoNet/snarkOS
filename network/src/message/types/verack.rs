use crate::message::{types::Version, Message, MessageName};
use snarkos_errors::network::message::MessageError;

use std::net::SocketAddr;

/// A handshake response to a Version message.
///
/// See network/protocol/handshake.rs for more details.
#[derive(Debug, PartialEq, Clone)]
pub struct Verack {
    /// Random nonce sequence number
    pub nonce: u64,

    /// Network address of sending node
    pub address_sender: SocketAddr,
}

impl Verack {
    pub fn new(version: Version) -> Self {
        Self {
            nonce: version.nonce,
            address_sender: version.address_receiver,
        }
    }
}

impl Message for Verack {
    fn name() -> MessageName {
        MessageName::from("verack")
    }

    fn deserialize(vec: Vec<u8>) -> Result<Self, MessageError> {
        if vec.len() != 18 {
            return Err(MessageError::InvalidLength(vec.len(), 18));
        }

        Ok(Self {
            nonce: bincode::deserialize(&vec[0..8])?,
            address_sender: bincode::deserialize(&vec[8..18])?,
        })
    }

    fn serialize(&self) -> Result<Vec<u8>, MessageError> {
        let mut writer = vec![];
        writer.extend_from_slice(&bincode::serialize(&self.nonce)?);
        writer.extend_from_slice(&bincode::serialize(&self.address_sender)?);
        Ok(writer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_data::random_socket_address;

    #[test]
    fn test_verack() {
        let version = Version::new(1u64, 1u32, random_socket_address(), random_socket_address());

        let message = Verack::new(version);

        let serialized = message.serialize().unwrap();
        let deserialized = Verack::deserialize(serialized).unwrap();

        assert_eq!(message, deserialized);
    }
}
