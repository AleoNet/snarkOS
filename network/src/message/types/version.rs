use crate::message::{Message, MessageName};
use snarkos_errors::network::message::MessageError;

use chrono::{DateTime, Utc};
use rand::Rng;
use std::net::SocketAddr;

#[derive(Debug, PartialEq, Clone)]
pub struct Version {
    /// The network version number
    pub version: u64,

    /// Latest block number of node sending this message
    pub height: u32,

    /// Random nonce sequence number
    pub nonce: u64,

    /// Message timestamp
    pub timestamp: DateTime<Utc>,

    /// Network address of message recipient
    pub address_receiver: SocketAddr,

    /// Network address of message sender
    pub address_sender: SocketAddr,
}

impl Version {
    pub fn new(version: u64, height: u32, address_receiver: SocketAddr, address_sender: SocketAddr) -> Self {
        let mut rng = rand::thread_rng();

        Self {
            version,
            height,
            nonce: rng.gen::<u64>(),
            timestamp: Utc::now(),
            address_receiver,
            address_sender,
        }
    }
}

impl Message for Version {
    fn name() -> MessageName {
        MessageName::from("version")
    }

    fn deserialize(vec: Vec<u8>) -> Result<Self, MessageError> {
        if vec.len() != 75 {
            return Err(MessageError::InvalidLength(vec.len(), 75));
        }

        Ok(Version {
            version: bincode::deserialize(&vec[..8])?,
            height: bincode::deserialize(&vec[8..12])?,
            nonce: bincode::deserialize(&vec[12..20])?,
            timestamp: bincode::deserialize(&vec[20..55])?,
            address_receiver: bincode::deserialize(&vec[55..65])?,
            address_sender: bincode::deserialize(&vec[65..75])?,
        })
    }

    fn serialize(&self) -> Result<Vec<u8>, MessageError> {
        let mut writer = vec![];
        writer.extend_from_slice(&bincode::serialize(&self.version)?);
        writer.extend_from_slice(&bincode::serialize(&self.height)?);
        writer.extend_from_slice(&bincode::serialize(&self.nonce)?);
        writer.extend_from_slice(&bincode::serialize(&self.timestamp)?);
        writer.extend_from_slice(&bincode::serialize(&self.address_receiver)?);
        writer.extend_from_slice(&bincode::serialize(&self.address_sender)?);
        Ok(writer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        let version = Version::new(
            1u64,
            1u32,
            "127.0.0.1:4130".parse::<SocketAddr>().unwrap(),
            "127.0.0.1:4130".parse::<SocketAddr>().unwrap(),
        );

        let serialized = version.serialize().unwrap();

        let deserialized = Version::deserialize(serialized).unwrap();

        assert_eq!(version, deserialized);
    }
}
