use crate::message::{Message, MessageName};
use snarkos_errors::network::message::MessageError;

use chrono::{DateTime, Utc};
use std::{collections::HashMap, net::SocketAddr};

/// A response to a GetPeers request.
#[derive(Debug, PartialEq, Clone)]
pub struct Peers {
    /// A list of gossiped peer addresses and their last seen dates
    pub addresses: HashMap<SocketAddr, DateTime<Utc>>,
}

impl Peers {
    pub fn new(addresses: HashMap<SocketAddr, DateTime<Utc>>) -> Self {
        Self { addresses }
    }
}

impl Message for Peers {
    fn name() -> MessageName {
        MessageName::from("peers")
    }

    fn deserialize(vec: Vec<u8>) -> Result<Self, MessageError> {
        Ok(Self {
            addresses: bincode::deserialize(&vec)?,
        })
    }

    fn serialize(&self) -> Result<Vec<u8>, MessageError> {
        Ok(bincode::serialize(&self.addresses)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_peers() {
        let message = Peers {
            addresses: HashMap::<SocketAddr, DateTime<Utc>>::new(),
        };

        let serialized = message.serialize().unwrap();
        let deserialized = Peers::deserialize(serialized).unwrap();

        assert_eq!(message, deserialized);
    }
}
