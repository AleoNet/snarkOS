// Copyright (C) 2019-2022 Aleo Systems Inc.
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

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Hash)]
#[repr(u8)]
pub enum NodeType {
    /// A client node is a full node, capable of syncing with the network.
    Client = 0,
    /// A prover is a full node, capable of producing proofs for consensus.
    Prover,
    /// A validator is a full node, capable of validating blocks.
    Validator,
    /// A beacon is a full node, capable of producing blocks.
    Beacon,
}

impl NodeType {
    /// Returns a string representation of the node type.
    pub const fn description(&self) -> &str {
        match self {
            Self::Client => "a client node",
            Self::Prover => "a prover node",
            Self::Validator => "a validator node",
            Self::Beacon => "a beacon node",
        }
    }

    /// Returns `true` if the node type is a client.
    pub const fn is_client(&self) -> bool {
        matches!(self, Self::Client)
    }

    /// Returns `true` if the node type is a prover.
    pub const fn is_prover(&self) -> bool {
        matches!(self, Self::Prover)
    }

    /// Returns `true` if the node type is a validator.
    pub const fn is_validator(&self) -> bool {
        matches!(self, Self::Validator)
    }

    /// Returns `true` if the node type is a beacon.
    pub const fn is_beacon(&self) -> bool {
        matches!(self, Self::Beacon)
    }
}

impl core::fmt::Display for NodeType {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", match self {
            Self::Client => "Client",
            Self::Prover => "Prover",
            Self::Validator => "Validator",
            Self::Beacon => "Beacon",
        })
    }
}
