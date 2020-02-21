use crate::message::{Message, MessageName};
use chrono::{DateTime, Utc};
use snarkos_errors::network::message::MessageError;
use std::net::SocketAddr;

#[derive(Debug, PartialEq, Clone)]
pub struct Version {
    /// The network version number
    pub version: u64,

    /// Message timestamp
    pub timestamp: DateTime<Utc>,

    /// Latest block number of node sending this message
    pub height: u32,

    /// Network address of message recipient
    pub address_receiver: SocketAddr,

    /// Network address of message sender
    pub address_sender: SocketAddr,
}

impl Message for Version {
    fn name() -> MessageName {
        MessageName::from("version")
    }

    fn deserialize(vec: Vec<u8>) -> Result<Self, MessageError> {
        if vec.len() != 67 {
            println!("vec.len {:?}", vec.len());
            return Err(MessageError::InvalidLength(vec.len(), 67));
        }

        Ok(Version {
            version: bincode::deserialize(&vec[..8])?,
            timestamp: bincode::deserialize(&vec[8..43])?,
            height: bincode::deserialize(&vec[43..47])?,
            address_receiver: bincode::deserialize(&vec[47..57])?,
            address_sender: bincode::deserialize(&vec[57..67])?,
        })
    }

    fn serialize(&self) -> Result<Vec<u8>, MessageError> {
        let mut writer = vec![];
        writer.extend_from_slice(&bincode::serialize(&self.version)?);
        writer.extend_from_slice(&bincode::serialize(&self.timestamp)?);
        writer.extend_from_slice(&bincode::serialize(&self.height)?);
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
        let version = Version {
            version: 1u64,
            timestamp: Utc::now(),
            height: 1u32,
            address_receiver: "127.0.0.1:4130".parse::<SocketAddr>().unwrap(),
            address_sender: "127.0.0.1:4130".parse::<SocketAddr>().unwrap(),
        };

        let serialized = version.serialize().unwrap();

        let deserialized = Version::deserialize(serialized).unwrap();

        assert_eq!(version, deserialized);
    }
}
