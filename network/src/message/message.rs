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
use snarkvm_ledger::BlockHeaderHash;

use std::{fmt, net::SocketAddr};

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum Direction {
    Inbound(SocketAddr),
    Outbound(SocketAddr),
}

impl fmt::Display for Direction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Inbound(addr) => write!(f, "from {}", addr),
            Self::Outbound(addr) => write!(f, "to {}", addr),
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
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum Payload {
    // todo: readd in Rust 1.54
    // #[cfg_attr(nightly, doc(include = "../../documentation/network_messages/block.md"))]
    Block(Vec<u8>),
    // todo: readd in Rust 1.54
    // #[cfg_attr(nightly, doc(include = "../../documentation/network_messages/get_blocks.md"))]
    GetBlocks(Vec<BlockHeaderHash>),
    // todo: readd in Rust 1.54
    // #[cfg_attr(nightly, doc(include = "../../documentation/network_messages/get_memory_pool.md"))]
    GetMemoryPool,
    // todo: readd in Rust 1.54
    // #[cfg_attr(nightly, doc(include = "../../documentation/network_messages/get_peers.md"))]
    GetPeers,
    // todo: readd in Rust 1.54
    // #[cfg_attr(nightly, doc(include = "../../documentation/network_messages/get_sync.md"))]
    GetSync(Vec<BlockHeaderHash>),
    // todo: readd in Rust 1.54
    // #[cfg_attr(nightly, doc(include = "../../documentation/network_messages/memory_pool.md"))]
    MemoryPool(Vec<Vec<u8>>),
    // todo: readd in Rust 1.54
    // #[cfg_attr(nightly, doc(include = "../../documentation/network_messages/peers.md"))]
    Peers(Vec<SocketAddr>),
    // todo: readd in Rust 1.54
    // #[cfg_attr(nightly, doc(include = "../../documentation/network_messages/ping.md"))]
    Ping(BlockHeight),
    // todo: readd in Rust 1.54
    // #[cfg_attr(nightly, doc(include = "../../documentation/network_messages/pong.md"))]
    Pong,
    // todo: readd in Rust 1.54
    // #[cfg_attr(nightly, doc(include = "../../documentation/network_messages/sync.md"))]
    Sync(Vec<BlockHeaderHash>),
    // todo: readd in Rust 1.54
    // #[cfg_attr(nightly, doc(include = "../../documentation/network_messages/sync_block.md"))]
    SyncBlock(Vec<u8>),
    // todo: readd in Rust 1.54
    // #[cfg_attr(nightly, doc(include = "../../documentation/network_messages/transaction.md"))]
    Transaction(Vec<u8>),

    // a placeholder indicating the introduction of a new payload type; used for forward compatibility
    #[doc(hidden)]
    Unknown,
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
            Self::Unknown => "unknown",
        };

        f.write_str(str)
    }
}
