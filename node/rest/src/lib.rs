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

use snarkos_node_consensus::Consensus;
use snarkos_node_ledger::{Ledger, RecordsFilter};
use snarkos_node_messages::{Data, Message, UnconfirmedTransaction};
use snarkos_node_router::{Router, RouterRequest};
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
use tokio::task::JoinHandle;
use warp::{http::StatusCode, reject, reply, Filter, Rejection, Reply};

/// A REST API server for the ledger.
#[derive(Clone)]
pub struct Rest<N: Network, C: ConsensusStorage<N>> {
    /// The node address.
    address: Address<N>,
    /// The consensus module.
    consensus: Option<Consensus<N, C>>,
    /// The ledger.
    ledger: Ledger<N, C>,
    /// The router.
    router: Router<N>,
    /// The server handles.
    handles: Vec<Arc<JoinHandle<()>>>,
}

impl<N: Network, C: 'static + ConsensusStorage<N>> Rest<N, C> {
    /// Initializes a new instance of the server.
    pub fn start(
        rest_ip: SocketAddr,
        address: Address<N>,
        consensus: Option<Consensus<N, C>>,
        ledger: Ledger<N, C>,
        router: Router<N>,
    ) -> Result<Self> {
        // Initialize the server.
        let mut server = Self { address, consensus, ledger, router, handles: vec![] };
        // Spawn the server.
        server.spawn_server(rest_ip);
        // Return the server.
        Ok(server)
    }
}

impl<N: Network, C: ConsensusStorage<N>> Rest<N, C> {
    /// Returns the ledger.
    pub const fn ledger(&self) -> &Ledger<N, C> {
        &self.ledger
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

        // Add custom logging for each request.
        let custom_log = warp::log::custom(|info| match info.remote_addr() {
            Some(addr) => debug!("Received '{} {}' from '{addr}' ({})", info.method(), info.path(), info.status()),
            None => debug!("Received '{} {}' ({})", info.method(), info.path(), info.status()),
        });

        // Spawn the server.
        let address = self.address;
        self.handles.push(Arc::new(tokio::spawn(async move {
            println!("🌐 Starting the REST server at {}.\n", rest_ip.to_string().bold());

            if let Ok(jwt_token) = helpers::Claims::new(address).to_jwt_string() {
                println!("JSON Web Token: {}\n", jwt_token);
            }

            // Start the server.
            warp::serve(routes.with(cors).with(custom_log)).run(rest_ip).await
        })))
    }
}
