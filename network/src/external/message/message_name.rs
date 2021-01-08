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

use crate::errors::message::MessageNameError;

use std::{convert::TryFrom, fmt, str};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum MessageName {
    Block = 0,
    GetBlock,
    GetMemoryPool,
    GetPeers,
    GetSync,
    MemoryPool,
    Peers,
    Sync,
    SyncBlock,
    Transaction,
    Verack,
    Version,
}

impl TryFrom<u8> for MessageName {
    type Error = MessageNameError;

    fn try_from(byte: u8) -> Result<Self, Self::Error> {
        let msg_type = match byte {
            0 => Self::Block,
            1 => Self::GetBlock,
            2 => Self::GetMemoryPool,
            3 => Self::GetPeers,
            4 => Self::GetSync,
            5 => Self::MemoryPool,
            6 => Self::Peers,
            7 => Self::Sync,
            8 => Self::SyncBlock,
            9 => Self::Transaction,
            10 => Self::Verack,
            11 => Self::Version,
            _ => return Err(MessageNameError::Message("unknown MessageName".into())),
        };

        Ok(msg_type)
    }
}

impl str::FromStr for MessageName {
    type Err = MessageNameError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let msg_type = match s {
            "block" => Self::Block,
            "getblock" => Self::GetBlock,
            "getmempool" => Self::GetMemoryPool,
            "getpeers" => Self::GetPeers,
            "getsync" => Self::GetSync,
            "memorypool" => Self::MemoryPool,
            "peers" => Self::Peers,
            "sync" => Self::Sync,
            "syncblock" => Self::SyncBlock,
            "transaction" => Self::Transaction,
            "verack" => Self::Verack,
            "version" => Self::Version,
            _ => return Err(MessageNameError::Message("unknown MessageName".into())),
        };

        Ok(msg_type)
    }
}

impl fmt::Display for MessageName {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let str = match self {
            Self::Block => "block",
            Self::GetBlock => "getblock",
            Self::GetMemoryPool => "getmempool",
            Self::GetPeers => "getpeers",
            Self::GetSync => "getsync",
            Self::MemoryPool => "memorypool",
            Self::Peers => "peers",
            Self::Sync => "sync",
            Self::SyncBlock => "syncblock",
            Self::Transaction => "transaction",
            Self::Verack => "verack",
            Self::Version => "version",
        };

        f.write_str(str)
    }
}
