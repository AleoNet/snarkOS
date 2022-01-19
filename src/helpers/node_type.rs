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
use std::fmt;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[repr(u8)]
pub enum NodeType {
    /// A client node is a full node, capable of sending and receiving blocks.
    Client = 0,
    /// A mining node is a full node, capable of producing new blocks.
    Miner,
    /// A beacon node is a discovery node, capable of sharing peers of the network.
    Beacon,
    /// A sync node is a discovery node, capable of syncing nodes for the network.
    Sync,
    /// An operating node is a full node, capable of coordinating provers in a pool.
    Operator,
    /// A proving node is a full node, capable of producing proofs for a pool.
    Prover,
}

impl NodeType {
    pub fn description(&self) -> &str {
        match self {
            Self::Client => "a client node",
            Self::Miner => "a mining node",
            Self::Beacon => "a beacon node",
            Self::Sync => "a sync node",
            Self::Operator => "an operating node",
            Self::Prover => "a proving node",
        }
    }
}

impl fmt::Display for NodeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
