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

#![forbid(unsafe_code)]

#[macro_use]
extern crate tracing;

mod helpers;
pub use helpers::*;

mod routes;
pub use routes::*;

mod start;
pub use start::*;

use snarkos_node_ledger::{Ledger, RecordsFilter};
use snarkvm::{
    console::{
        account::{Address, ViewKey},
        program::ProgramID,
        types::Field,
    },
    prelude::Network,
    synthesizer::{ConsensusStorage, Program, Transaction},
};

use anyhow::Result;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::{str::FromStr, sync::Arc};
use tokio::{sync::mpsc, task::JoinHandle};
use warp::{http::StatusCode, reject, reply, Filter, Rejection, Reply};

/// Shorthand for the parent half of the `Ledger` message channel.
pub type LedgerSender<N> = mpsc::Sender<LedgerRequest<N>>;
/// Shorthand for the child half of the `Ledger` message channel.
pub type LedgerReceiver<N> = mpsc::Receiver<LedgerRequest<N>>;

/// An enum of requests that the `Ledger` struct processes.
#[derive(Debug)]
pub enum LedgerRequest<N: Network> {
    TransactionBroadcast(Transaction<N>),
}

/// A REST API server for the ledger.
#[derive(Clone)]
pub struct Server<N: Network, C: ConsensusStorage<N>> {
    /// The ledger.
    ledger: Ledger<N, C>,
    /// The ledger sender.
    ledger_sender: LedgerSender<N>,
    /// The server handles.
    handles: Vec<Arc<JoinHandle<()>>>,
}

impl<N: Network, C: ConsensusStorage<N>> Server<N, C> {
    /// Returns the ledger.
    pub const fn ledger(&self) -> &Ledger<N, C> {
        &self.ledger
    }

    /// Returns the ledger sender.
    pub const fn ledger_sender(&self) -> &LedgerSender<N> {
        &self.ledger_sender
    }

    /// Returns the handles.
    pub const fn handles(&self) -> &Vec<Arc<JoinHandle<()>>> {
        &self.handles
    }
}
