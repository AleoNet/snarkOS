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

use super::*;
use http::header::HeaderName;

impl<N: Network, B: 'static + BlockStorage<N>, P: 'static + ProgramStorage<N>> Server<N, B, P> {
    /// Initializes a new instance of the server.
    pub fn start(
        ledger: Arc<RwLock<Ledger<N, B, P>>>,
        additional_routes: Option<impl Filter<Extract = impl Reply, Error = Rejection> + Clone + Sync + Send + 'static>,
        custom_port: Option<u16>,
    ) -> Result<Self> {
        // Initialize a channel to send requests to the ledger.
        let (ledger_sender, ledger_receiver) = mpsc::channel(64);

        // Initialize the server.
        let mut server = Self { ledger, ledger_sender, handles: vec![] };
        // Spawn the server.
        server.spawn_server(additional_routes, custom_port);
        // Spawn the ledger handler.
        server.spawn_ledger_handler(ledger_receiver);

        // Return the server.
        Ok(server)
    }
}

impl<N: Network, B: 'static + BlockStorage<N>, P: 'static + ProgramStorage<N>> Server<N, B, P> {
    /// Initializes the server.
    fn spawn_server(
        &mut self,
        additional_routes: Option<impl Filter<Extract = impl Reply, Error = Rejection> + Clone + Sync + Send + 'static>,
        custom_port: Option<u16>,
    ) {
        let cors = warp::cors()
            .allow_any_origin()
            .allow_header(HeaderName::from_static("content-type"))
            .allow_methods(vec!["GET", "POST", "OPTIONS"]);

        // Initialize the routes.
        let routes = self.routes();
        // Spawn the server.
        self.handles.push(Arc::new(tokio::spawn(async move {
            // Initialize the listening IP.
            let ip = match custom_port {
                Some(port) => ([0, 0, 0, 0], port),
                None => ([0, 0, 0, 0], 80),
            };
            println!("\n🌐 Server is running at http://0.0.0.0:{}\n", ip.1);
            // Start the server, with optional additional routes.
            match additional_routes {
                Some(additional_routes) => warp::serve(routes.or(additional_routes).with(cors)).run(ip).await,
                None => warp::serve(routes.with(cors)).run(ip).await,
            }
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
                        match ledger.write().add_to_memory_pool(transaction) {
                            Ok(()) => trace!("✉️ Added transaction '{transaction_id}' to the memory pool"),
                            Err(error) => {
                                warn!("⚠️ Failed to add transaction '{transaction_id}' to the memory pool: {error}")
                            }
                        }
                    }
                };
            }
        })))
    }
}
