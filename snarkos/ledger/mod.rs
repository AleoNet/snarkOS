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

mod block;
pub use block::*;

mod header;
pub use header::*;

mod transaction;
pub use transaction::*;

mod transactions;
pub use transactions::*;

use crate::{
    message::DisconnectReason,
    peers::{Peers, PeersHandler, PeersRequest},
    Account,
};

use snarkos_environment::{helpers::Status, Environment};
use snarkvm::{
    circuit::Aleo,
    compiler::{Process, Program},
    prelude::*,
};

use anyhow::Result;
use once_cell::race::OnceBox;
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tokio::{net::TcpListener, sync::oneshot};

#[derive(Debug)]
pub struct Ledger<N: Network> {
    /// The current block.
    block: Block<N>,
}

impl<N: Network> Ledger<N> {
    /// Initializes the ledger from genesis.
    pub fn genesis<A: Aleo<Network = N, BaseField = N::Field>>() -> Result<Self> {
        // Return the ledger.
        Ok(Self {
            block: Block::genesis::<A>()?,
        })
    }
}
