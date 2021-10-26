// Copyright (C) 2019-2021 Aleo Systems Inc.
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

use serde::{Deserialize, Serialize};
use std::{fmt::Debug, time::Duration};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum NodeType {
    /// A client node is a full node, capable of sending and receiving blocks.
    Client,
    /// A mining node is a full node, capable of producing new blocks.
    Miner,
    /// A peer node is a discovery node, capable of sharing peers of the network.
    Peer,
    /// A sync node is a discovery node, capable of syncing nodes for the network.
    Sync,
}

#[rustfmt::skip]
pub trait Environment: 'static + Clone + Debug + Default + Send + Sync {
    const NODE_TYPE: NodeType;

    /// If `true`, a mining node will craft public coinbase transactions.
    const COINBASE_IS_PUBLIC: bool;
    /// If `true`, a node will remote fetch blocks from genesis.
    const FAST_SYNC: bool = true;

    // /// The port for communication with a node server.
    // const NODE_PORT: u16 = 4130 + N::NETWORK_ID;
    // /// The port for communication with an RPC server.
    // const RPC_PORT: u16 = 3030 + N::NETWORK_ID;

    /// The list of peer nodes to bootstrap the node server with.
    const PEER_NODES: Vec<&'static str>;
    /// The list of sync nodes to bootstrap the node server with.
    const SYNC_NODES: Vec<&'static str>;

    /// The minimum number of peers required to maintain connections with.
    const MINIMUM_NUMBER_OF_PEERS: usize = 1;
    /// The maximum number of peers permitted to maintain connections with.
    const MAXIMUM_NUMBER_OF_PEERS: usize = 21;
    /// The maximum number of candidate peers permitted to be stored in the node.
    const MAXIMUM_CANDIDATE_PEERS: usize = 10_000;

    /// The maximum amount of time in which a handshake with a regular node can conclude before dropping the
    /// connection; it should be no greater than the `peer_sync_interval`.
    const HANDSHAKE_TIMEOUT_SECS: u64 = 5;

    /// The amount of time after which a peer will be considered inactive an disconnected from if they have
    /// not sent any messages in the meantime.
    const MAX_PEER_INACTIVITY_SECS: u8 = 30;
    /// The maximum size of a message that can be transmitted in the network.
    const MAX_MESSAGE_SIZE: usize = 8 * 1024 * 1024; // 8MiB

    const CONNECTION_TIMEOUT_SECS: u64 = 3;

    const FAILURE_EXPIRY_TIME: Duration = Duration::from_secs(15 * 60);
    const FAILURE_THRESHOLD: usize = 5;

    /// The version of the network protocol; it can be incremented in order to force users to update.
    const MESSAGE_VERSION: u32 = 0;

    const REBASE_THRESHOLD: usize = 1024;
}

#[derive(Clone, Debug, Default)]
pub struct Client;

#[rustfmt::skip]
impl Environment for Client {
    const NODE_TYPE: NodeType = NodeType::Client;

    const COINBASE_IS_PUBLIC: bool = true;

    const PEER_NODES: Vec<&'static str> = vec![];
    const SYNC_NODES: Vec<&'static str> = vec![];
}

#[derive(Clone, Debug, Default)]
pub struct Miner;

#[rustfmt::skip]
impl Environment for Miner {
    const NODE_TYPE: NodeType = NodeType::Miner;

    const COINBASE_IS_PUBLIC: bool = true;

    const PEER_NODES: Vec<&'static str> = vec![];
    const SYNC_NODES: Vec<&'static str> = vec![];
}
