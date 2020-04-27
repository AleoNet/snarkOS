use crate::message::{Message, MessageName};
use snarkos_errors::network::message::MessageError;

/// A transaction sent by a peer.
#[derive(Debug, PartialEq, Clone)]
pub struct Transaction {
    /// Serialized transaction bytes
    pub(crate) bytes: Vec<u8>,
}

impl Transaction {
    pub fn new(bytes: Vec<u8>) -> Self {
        Self { bytes }
    }
}

impl Message for Transaction {
    fn name() -> MessageName {
        MessageName::from("transaction")
    }

    fn deserialize(vec: Vec<u8>) -> Result<Self, MessageError> {
        Ok(Self {
            bytes: bincode::deserialize(&vec)?,
        })
    }

    fn serialize(&self) -> Result<Vec<u8>, MessageError> {
        Ok(bincode::serialize(&self.bytes)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use snarkos_consensus::test_data::TRANSACTION;

    #[test]
    fn test_transaction() {
        let transaction = hex::decode(TRANSACTION).unwrap();
        let message = Transaction::new(transaction);

        let serialized = message.serialize().unwrap();
        let deserialized = Transaction::deserialize(serialized).unwrap();

        assert_eq!(message, deserialized);
    }
}
