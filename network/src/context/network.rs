use crate::bootnodes::*;

use serde::Serialize;

pub const MAGIC_MAINNET: u32 = 0xD9B4BEF9;
pub const MAGIC_TESTNET: u32 = 0x0709110B;

pub const PORT_MAINNET: u16 = 4130;
pub const PORT_TESTNET: u16 = 14130;

pub const RPC_PORT_MAINNET: u16 = 3030;
pub const RPC_PORT_TESTNET: u16 = 13030;

pub type Magic = u32;
pub type Port = u16;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub enum Network {
    /// The main Aleo network for sending and receiving live transactions.
    Mainnet,

    /// The test Aleo network for trying the technology without real financial value.
    Testnet,
    // /// An independent network using Aleo technology but not maintained by the Aleo team.
    // Other(u32),
}

impl Network {
    /// Bytes that are appended to the front of a message to designate the intended network.
    pub fn magic(&self) -> Magic {
        match *self {
            Network::Mainnet => MAGIC_MAINNET,
            Network::Testnet => MAGIC_TESTNET,
            // Network::Other(magic) => magic,
        }
    }

    /// The designated default port for the server to receive peer connections on.
    pub fn port(&self) -> Port {
        match *self {
            Network::Mainnet => PORT_MAINNET,
            Network::Testnet => PORT_TESTNET,
        }
    }

    /// The designated default port for the node to receive rpc requests on.
    pub fn rpc_port(&self) -> Port {
        match *self {
            Network::Mainnet => RPC_PORT_MAINNET,
            Network::Testnet => RPC_PORT_TESTNET,
        }
    }

    /// Hardcoded bootnodes maintained by Aleo.
    pub fn bootnodes(&self) -> Vec<String> {
        match *self {
            Network::Mainnet => MAINNET_BOOTNODES,
            Network::Testnet => TESTNET_BOOTNODES,
        }
        .iter()
        .map(|node| (*node).to_string())
        .collect::<Vec<String>>()
    }

    /// The hex-encoded bytes of the genesis block.
    pub fn genesis(&self) -> String {
        // match *self {
        //     Network::Mainnet => "00000000000000000000000000000000000000000000000000000000000000008c8d4f393f39c063c40a617c6e2584e6726448c4c0f7da7c848bfa573e628388fbf1285e00000000ffffffffff7f00005e4401000101000000010000000000000000000000000000000000000000000000000000000000000000ffffffff04010000000100e1f505000000001976a914ef5392fc02643be8b98f6aaca5c1ffaab238916a88ac".into(),
        //     Network::Testnet => //TODO: Mine testnet genesis
        // }
        "00000000000000000000000000000000000000000000000000000000000000008c8d4f393f39c063c40a617c6e2584e6726448c4c0f7da7c848bfa573e628388fbf1285e00000000ffffffffff7f00005e4401000101000000010000000000000000000000000000000000000000000000000000000000000000ffffffff04010000000100e1f505000000001976a914ef5392fc02643be8b98f6aaca5c1ffaab238916a88ac".into()
    }
}
