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

#![deny(missing_docs)]
#![deny(unsafe_code)]

//! **P2P** is a simple, low-level, and customizable implementation of a TCP P2P node.

mod config;
mod known_peers;
mod node;
mod stats;

pub mod connections;
pub mod protocols;

pub use config::Config;
pub use connections::{Connection, ConnectionSide};
pub use known_peers::KnownPeers;
pub use node::Network;
pub use stats::Stats;

/// A trait for objects containing a [`Network`]; it is required to implement protocols.
pub trait P2P {
    /// Returns a clonable reference to the node.
    fn network(&self) -> &Network;
}
