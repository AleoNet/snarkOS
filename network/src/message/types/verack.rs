use crate::message::{types::Version, Message, MessageName};
use snarkos_errors::network::message::MessageError;

#[derive(Debug, PartialEq, Clone)]
pub struct Verack {
    /// Random nonce sequence number
    pub nonce: u64, // todo: make private
}

impl Verack {
    pub fn new(version: Version) -> Self {
        Self { nonce: version.nonce }
    }
}

impl Message for Verack {
    fn name() -> MessageName {
        MessageName::from("verack")
    }

    fn deserialize(vec: Vec<u8>) -> Result<Self, MessageError> {
        if vec.len() != 8 {
            return Err(MessageError::InvalidLength(vec.len(), 8));
        }

        Ok(Self {
            nonce: bincode::deserialize(&vec)?,
        })
    }

    fn serialize(&self) -> Result<Vec<u8>, MessageError> {
        Ok(bincode::serialize(&self.nonce)?)
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
