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

use snarkos_node_ledger::{Ledger, RecordsFilter};
use snarkos_node_router::Router;
use snarkvm::{
    console::{
        account::{Address, ViewKey},
        program::ProgramID,
        types::Field,
    },
    prelude::{cfg_into_iter, Network},
    synthesizer::{ConsensusStorage, Program, Transaction},
};

use anyhow::Result;
use colored::*;
use http::header::HeaderName;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, str::FromStr, sync::Arc};
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
pub struct Rest<N: Network, C: ConsensusStorage<N>> {
    /// The ledger.
    ledger: Ledger<N, C>,
    /// The ledger sender.
    ledger_sender: LedgerSender<N>,
    /// The router.
    router: Router<N>,
    /// The server handles.
    handles: Vec<Arc<JoinHandle<()>>>,
}

impl<N: Network, C: 'static + ConsensusStorage<N>> Rest<N, C> {
    /// Initializes a new instance of the server.
    pub fn start(rest_ip: SocketAddr, ledger: Ledger<N, C>, router: Router<N>) -> Result<Self> {
        // Initialize a channel to send requests to the ledger.
        let (ledger_sender, ledger_receiver) = mpsc::channel(64);

        // Initialize the server.
        let mut server = Self { ledger, ledger_sender, router, handles: vec![] };
        // Spawn the server.
        server.spawn_server(rest_ip);
        // Spawn the ledger handler.
        server.spawn_ledger_handler(ledger_receiver);

        // Return the server.
        Ok(server)
    }
}

impl<N: Network, C: ConsensusStorage<N>> Rest<N, C> {
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

impl<N: Network, C: 'static + ConsensusStorage<N>> Rest<N, C> {
    /// Initializes the server.
    fn spawn_server(&mut self, rest_ip: SocketAddr) {
        let cors = warp::cors()
            .allow_any_origin()
            .allow_header(HeaderName::from_static("content-type"))
            .allow_methods(vec!["GET", "POST", "OPTIONS"]);

        // Initialize the routes.
        let routes = self.routes();
        // Spawn the server.
        self.handles.push(Arc::new(tokio::spawn(async move {
            println!("🌐 Starting the REST server at {}.\n", rest_ip.to_string().bold());
            // Start the server.
            warp::serve(routes.with(cors)).run(rest_ip).await
        })))
    }

    /// Initializes the ledger handler.
    fn spawn_ledger_handler(&mut self, mut ledger_receiver: LedgerReceiver<N>) {
        // Prepare the ledger.
        let ledger = self.ledger.clone();
        // Spawn the ledger handler.
        self.handles.push(Arc::new(tokio::spawn(async move {
            while let Some(request) = ledger_receiver.recv().await {
                match request {
                    LedgerRequest::TransactionBroadcast(transaction) => {
                        // Retrieve the transaction ID.
                        let transaction_id = transaction.id();
                        // Add the transaction to the memory pool.
                        if let Err(error) = ledger.add_unconfirmed_transaction(transaction) {
                            warn!("⚠️ Failed to add transaction '{transaction_id}' to the memory pool: {error}")
                        }
                    }
                };
            }
        })))
    }
}
