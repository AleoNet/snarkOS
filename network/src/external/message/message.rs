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

use snarkos_storage::BlockHeight;
use snarkvm_objects::BlockHeaderHash;

use serde::{Deserialize, Serialize};

use std::{fmt, net::SocketAddr};

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum Direction {
    Inbound(SocketAddr),
    Outbound(SocketAddr),
    Internal,
}

impl fmt::Display for Direction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Inbound(addr) => write!(f, "from {}", addr),
            Self::Outbound(addr) => write!(f, "to {}", addr),
            Self::Internal => write!(f, "<internal>"),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Message {
    pub direction: Direction,
    pub payload: Payload,
}

impl Message {
    pub fn new(direction: Direction, payload: Payload) -> Self {
        Self { direction, payload }
    }

    pub fn receiver(&self) -> SocketAddr {
        match self.direction {
            Direction::Outbound(addr) => addr,
            _ => unreachable!("Message::receiver used on a non-outbound Message!"),
        }
    }
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} {}", self.payload, self.direction)
    }
}

/// The actual message transmitted over the network.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum Payload {
    #[cfg_attr(nightly, doc(include = "../../../documentation/network_messages/block.md"))]
    Block(Vec<u8>),
    #[cfg_attr(nightly, doc(include = "../../../documentation/network_messages/get_blocks.md"))]
    GetBlocks(Vec<BlockHeaderHash>),
    #[cfg_attr(nightly, doc(include = "../../../documentation/network_messages/get_memory_pool.md"))]
    GetMemoryPool,
    #[cfg_attr(nightly, doc(include = "../../../documentation/network_messages/get_peers.md"))]
    GetPeers,
    #[cfg_attr(nightly, doc(include = "../../../documentation/network_messages/get_sync.md"))]
    GetSync(Vec<BlockHeaderHash>),
    #[cfg_attr(nightly, doc(include = "../../../documentation/network_messages/memory_pool.md"))]
    MemoryPool(Vec<Vec<u8>>),
    #[cfg_attr(nightly, doc(include = "../../../documentation/network_messages/peers.md"))]
    Peers(Vec<SocketAddr>),
    #[cfg_attr(nightly, doc(include = "../../../documentation/network_messages/ping.md"))]
    Ping(BlockHeight),
    #[cfg_attr(nightly, doc(include = "../../../documentation/network_messages/pong.md"))]
    Pong,
    #[cfg_attr(nightly, doc(include = "../../../documentation/network_messages/sync.md"))]
    Sync(Vec<BlockHeaderHash>),
    #[cfg_attr(nightly, doc(include = "../../../documentation/network_messages/sync_block.md"))]
    SyncBlock(Vec<u8>),
    #[cfg_attr(nightly, doc(include = "../../../documentation/network_messages/transaction.md"))]
    Transaction(Vec<u8>),

    /* internal messages */
    #[doc(hide)]
    ConnectedTo(SocketAddr, Option<SocketAddr>),
    #[doc(hide)]
    ConnectingTo(SocketAddr),
    // TODO: used internally, but can also be used to allow a clean disconnect for connected peers on shutdown
    // add a doc if this is introduced
    #[doc(hide)]
    Disconnect(SocketAddr),
}

impl fmt::Display for Payload {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let str = match self {
            Self::Block(..) => "block",
            Self::GetBlocks(..) => "getblocks",
            Self::GetMemoryPool => "getmempool",
            Self::GetPeers => "getpeers",
            Self::GetSync(..) => "getsync",
            Self::MemoryPool(..) => "memorypool",
            Self::Peers(..) => "peers",
            Self::Ping(..) => "ping",
            Self::Pong => "pong",
            Self::Sync(..) => "sync",
            Self::SyncBlock(..) => "syncblock",
            Self::Transaction(..) => "transaction",
            Self::ConnectedTo(..) => "connectedto",
            Self::ConnectingTo(..) => "connectingto",
            Self::Disconnect(..) => "disconnect",
        };

        f.write_str(str)
    }
}
