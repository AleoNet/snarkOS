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

use crate::{
    external::{message_types::*, Message, MessageHeader},
    outbound::Channel,
};

use std::{fmt, net::SocketAddr};
use tokio::io::AsyncWriteExt;

pub type Receiver = SocketAddr;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum Request {
    Version(Version),
    Verack(Verack),
    GetBlock(Receiver, GetBlock),
    Block(Receiver, Block),
    SyncBlock(Receiver, SyncBlock),
    MemoryPool(Receiver, MemoryPool),
    GetSync(Receiver, GetSync),
    Sync(Receiver, Sync),
    GetPeers(Receiver, GetPeers),
    Peers(Receiver, Peers),
    Transaction(Receiver, Transaction),
}

impl Request {
    #[inline]
    pub fn name(&self) -> &str {
        match self {
            Request::Version(_) => "Version",
            Request::Verack(_) => "Verack",
            Request::GetBlock(_, _) => "GetBlock",
            Request::Block(_, _) => "Block",
            Request::SyncBlock(_, _) => "SyncBlock",
            Request::MemoryPool(_, _) => "MemoryPool",
            Request::GetSync(_, _) => "GetSync",
            Request::Sync(_, _) => "Sync",
            Request::GetPeers(_, _) => "GetPeers",
            Request::Peers(_, _) => "Peers",
            Request::Transaction(_, _) => "Transaction",
        }
    }

    #[inline]
    pub fn receiver(&self) -> Receiver {
        match self {
            Request::Version(version) => version.receiver,
            Request::Verack(verack) => verack.receiver,
            Request::GetBlock(receiver, _) => *receiver,
            Request::Block(receiver, _) => *receiver,
            Request::SyncBlock(receiver, _) => *receiver,
            Request::MemoryPool(receiver, _) => *receiver,
            Request::GetSync(receiver, _) => *receiver,
            Request::Sync(receiver, _) => *receiver,
            Request::GetPeers(receiver, _) => *receiver,
            Request::Peers(receiver, _) => *receiver,
            Request::Transaction(receiver, _) => *receiver,
        }
    }

    /// Locks the given channel and writes the request to it.
    #[inline]
    pub async fn write_to_channel(&self, channel: &Channel) -> anyhow::Result<()> {
        Ok(channel.lock().await.write_all(&self.serialize()?).await?)
    }

    #[inline]
    pub fn serialize(&self) -> anyhow::Result<Vec<u8>> {
        let (name, data) = match self {
            Request::Version(version) => (Version::name(), version.serialize()?),
            Request::Verack(verack) => (Verack::name(), verack.serialize()?),
            Request::GetBlock(_, message) => (GetBlock::name(), message.serialize()?),
            Request::Block(_, message) => (Block::name(), message.serialize()?),
            Request::SyncBlock(_, message) => (SyncBlock::name(), message.serialize()?),
            Request::MemoryPool(_, message) => (MemoryPool::name(), message.serialize()?),
            Request::GetSync(_, message) => (GetSync::name(), message.serialize()?),
            Request::Sync(_, message) => (Sync::name(), message.serialize()?),
            Request::GetPeers(_, message) => (GetPeers::name(), message.serialize()?),
            Request::Peers(_, message) => (Peers::name(), message.serialize()?),
            Request::Transaction(_, message) => (Transaction::name(), message.serialize()?),
        };

        let mut buffer = MessageHeader::new(name, data.len() as u32).serialize()?;
        buffer.extend_from_slice(&data);
        Ok(buffer)
    }
}

impl fmt::Display for Request {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}
