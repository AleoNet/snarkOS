// Copyright (C) 2019-2020 Aleo Systems Inc.
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

use crate::external::{message_types::*, Channel, Message, MessageHeader, MessageName};

use std::{fmt, net::SocketAddr};
use tokio::io::AsyncWriteExt;

#[derive(Debug)]
pub enum Response {
    /// Received a version message and preparing to send a verack message back.
    VersionToVerack(SocketAddr, Version),
    /// Receive handler is connecting to the given peer with the given nonce.
    ConnectingTo(SocketAddr, u64),
    /// Receive handler has connected to the given peer with the given nonce.
    ConnectedTo(SocketAddr, u64),
    /// Receive handler has signaled to drop the connection with the given peer.
    DisconnectFrom(SocketAddr),
    /// Receive handler received a new transaction from the given peer.
    Transaction(SocketAddr, Transaction),
    /// Receive handler received a getpeers message.
    GetPeers(SocketAddr),
    /// Receive handler received a peers response.
    Peers(SocketAddr, Peers),
    /// Receive handler received a block.
    Block(SocketAddr, Block, bool),
    /// Receive handler received a getblock message.
    GetBlock(SocketAddr, GetBlock),
    /// Receive handler received a getmemorypool message.
    GetMemoryPool(SocketAddr),
    /// Receive handler received a memory pool.
    MemoryPool(MemoryPool),
    /// Receive handler received a getsync message.
    GetSync(SocketAddr, GetSync),
}
