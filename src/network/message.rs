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

use snarkvm::dpc::{Block, Network};

use std::{fmt, net::SocketAddr};

/// A message transmitted over the network.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum Message<N: Network> {
    // #[doc = include_str!("../../documentation/network_messages/block.md")]
    // Block(Block<N>),
    // #[doc = include_str!("../../documentation/network_messages/get_blocks.md")]
    GetBlock(N::BlockHash),
    // #[doc = include_str!("../../documentation/network_messages/get_memory_pool.md")]
    // GetMemoryPool,
    // #[doc = include_str!("../../documentation/network_messages/get_peers.md")]
    GetPeers,
    // #[doc = include_str!("../../documentation/network_messages/get_sync.md")]
    // GetSync(Vec<Digest>),
    // #[doc = include_str!("../../documentation/network_messages/memory_pool.md")]
    // MemoryPool(Vec<Vec<u8>>),
    // #[doc = include_str!("../../documentation/network_messages/peers.md")]
    Peers(Vec<SocketAddr>),
    // #[doc = include_str!("../../documentation/network_messages/ping.md")]
    Ping(u32),
    // #[doc = include_str!("../../documentation/network_messages/pong.md")]
    Pong,
    // #[doc = include_str!("../../documentation/network_messages/sync.md")]
    // Sync(Vec<Digest>),
    // #[doc = include_str!("../../documentation/network_messages/sync_block.md")]
    // SyncBlock(Vec<u8>, Option<u32>),
    // #[doc = include_str!("../../documentation/network_messages/transaction.md")]
    // Transaction(Vec<u8>),
}

impl<N: Network> fmt::Display for Message<N> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let str = match self {
            // Self::Block(..) => "block",
            Self::GetBlock(..) => "getblock",
            // Self::GetMemoryPool => "getmempool",
            Self::GetPeers => "getpeers",
            // Self::GetSync(..) => "getsync",
            // Self::MemoryPool(..) => "memorypool",
            Self::Peers(..) => "peers",
            Self::Ping(..) => "ping",
            Self::Pong => "pong",
            // Self::Sync(..) => "sync",
            // Self::SyncBlock(..) => "syncblock",
            // Self::Transaction(..) => "transaction",
        };

        f.write_str(str)
    }
}
