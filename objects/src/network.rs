/// Represents the network the node operating on
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
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
