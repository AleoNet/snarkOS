use snarkos_utilities::bytes::{FromBytes, ToBytes};

use std::{
    fmt,
    io::{Read, Result as IoResult, Write},
};

/// Represents the network the node operating on
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Network {
    Mainnet,
    Testnet1,
    Custom(u8),
}

impl Network {
    /// Returns the id of the network
    pub fn id(&self) -> u8 {
        match self {
            Network::Mainnet => 0,
            Network::Testnet1 => 1,
            Network::Custom(id) => *id,
        }
    }

    /// Returns the network from a given network id
    pub fn from_network_id(network_id: u8) -> Self {
        match network_id {
            0 => Network::Mainnet,
            1 => Network::Testnet1,
            id => Network::Custom(id),
        }
    }
}

impl ToBytes for Network {
    #[inline]
    fn write<W: Write>(&self, mut writer: W) -> IoResult<()> {
        self.id().write(&mut writer)
    }
}

impl FromBytes for Network {
    #[inline]
    fn read<R: Read>(mut reader: R) -> IoResult<Self> {
        let network_id: u8 = FromBytes::read(&mut reader)?;

        Ok(Self::from_network_id(network_id))
    }
}

impl fmt::Display for Network {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.id())
    }
}
