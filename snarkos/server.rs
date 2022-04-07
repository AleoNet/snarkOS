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

use crate::{display::notification_message, Node};
use snarkos_environment::{
    helpers::{NodeType, Status},
    network::DisconnectReason,
    Environment,
};
use snarkos_network::{
    ledger::{Ledger, LedgerRequest},
    operator::Operator,
    peers::{Peers, PeersRequest},
    prover::Prover,
    State,
};
use snarkvm::prelude::*;

#[cfg(feature = "rpc")]
use snarkos_rpc::{initialize_rpc_server, RpcContext};

#[cfg(any(feature = "test", feature = "prometheus"))]
use snarkos_metrics as metrics;
#[cfg(any(feature = "test", feature = "prometheus"))]
use snarkos_network::LedgerReader;

use anyhow::Result;
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tokio::{net::TcpListener, sync::oneshot, task};

///
/// A set of operations to initialize the node server for a specific network.
///
#[derive(Clone)]
pub struct Server<N: Network, E: Environment> {
    pub state: Arc<State<N, E>>,
}

impl<N: Network, E: Environment> Server<N, E> {
    ///
    /// Starts the connection listener for peers.
    ///
    pub async fn initialize(node: &Node, address: Option<Address<N>>, pool_ip: Option<SocketAddr>) -> Result<Self> {
        // Initialize a new TCP listener at the given IP.
        let (local_ip, listener) = match TcpListener::bind(node.node).await {
            Ok(listener) => (listener.local_addr().expect("Failed to fetch the local IP"), listener),
            Err(error) => panic!("Failed to bind listener: {:?}. Check if another Aleo node is running", error),
        };

        // Initialize the ledger storage path.
        let ledger_storage_path = node.ledger_storage_path(local_ip);
        // Initialize the operator storage path.
        let operator_storage_path = node.operator_storage_path(local_ip);
        // Initialize the prover storage path.
        let prover_storage_path = node.prover_storage_path(local_ip);

        // Initialize the shared state.
        let state = Arc::new(State::new(local_ip, address));

        // Initialize a new instance for managing peers.
        let (peers, peers_handler) = Peers::new(None, state.clone()).await;

        // Initialize a new instance for managing the ledger.
        let (ledger, ledger_handler) = Ledger::<N, E>::open::<_>(&ledger_storage_path, state.clone()).await?;

        // Initialize a new instance for managing the prover.
        let (prover, prover_handler) = Prover::open::<_>(&prover_storage_path, pool_ip, state.clone()).await?;

        // Initialize a new instance for managing the operator.
        let (operator, operator_handler) = Operator::open::<_>(&operator_storage_path, state.clone()).await?;

        // Initialise the metrics exporter.
        #[cfg(any(feature = "test", feature = "prometheus"))]
        Self::initialize_metrics(ledger.reader().clone());

        let server = Self { state };

        server.state.initialize_peers(peers, peers_handler).await;
        server.state.initialize_ledger(ledger, ledger_handler).await;
        server.state.initialize_prover(prover, prover_handler).await;
        server.state.initialize_operator(operator, operator_handler).await;

        server.state.prover().initialize_miner().await;
        server.state.prover().initialize_pooling().await;
        server.state.prover().initialize_pool_connection_loop(pool_ip).await;
        server.state.operator().initialize().await;

        server.initialize_notification(address).await;
        server.initialize_listener(listener).await;
        server.initialize_heartbeat().await;
        server.initialize_rpc(node, address).await;

        Ok(server)
    }

    /// Returns the IP address of this node.
    pub fn local_ip(&self) -> SocketAddr {
        self.state.local_ip
    }

    ///
    /// Sends a connection request to the given IP address.
    ///
    pub async fn connect_to(&self, peer_ip: SocketAddr) -> Result<()> {
        // Initialize the connection process.
        let (router, handler) = oneshot::channel();

        // Route a `Connect` request to the peer manager.
        self.state.peers().router().send(PeersRequest::Connect(peer_ip, router)).await?;

        // Wait until the connection task is initialized.
        handler.await.map(|_| ()).map_err(|e| e.into())
    }

    #[inline]
    pub async fn disconnect_from(&self, peer_ip: SocketAddr, reason: DisconnectReason) {
        self.state.ledger().disconnect(peer_ip, reason).await
    }

    ///
    /// Disconnects from peers and proceeds to shut down the node.
    ///
    pub async fn shut_down(&self) {
        info!("Shutting down...");
        // Update the node status.
        E::status().update(Status::ShuttingDown);

        // Shut down the ledger.
        trace!("Proceeding to shut down the ledger...");
        self.state.ledger().shut_down().await;

        // Flush the tasks.
        E::resources().shut_down();
        trace!("Node has shut down.");
    }

    ///
    /// Initialize the connection listener for new peers.
    ///
    async fn initialize_listener(&self, listener: TcpListener) {
        // Initialize the listener process.
        let (router, handler) = oneshot::channel();
        let state = self.state.clone();
        E::resources().register_task(
            None, // No need to provide an id, as the task will run indefinitely.
            task::spawn(async move {
                // Notify the outer function that the task is ready.
                let _ = router.send(());
                info!("Listening for peers at {}", state.local_ip);
                loop {
                    // Don't accept connections if the node is breaching the configured peer limit.
                    if state.peers().number_of_connected_peers().await < E::MAXIMUM_NUMBER_OF_PEERS {
                        // Asynchronously wait for an inbound TcpStream.
                        match listener.accept().await {
                            // Process the inbound connection request.
                            Ok((stream, peer_ip)) => {
                                let request = PeersRequest::PeerConnecting(stream, peer_ip);
                                if let Err(error) = state.peers().router().send(request).await {
                                    error!("Failed to send request to peers: {}", error)
                                }
                            }
                            Err(error) => error!("Failed to accept a connection: {}", error),
                        }
                        // Add a small delay to prevent overloading the network from handshakes.
                        tokio::time::sleep(Duration::from_millis(150)).await;
                    } else {
                        // Add a sleep delay as the node has reached peer capacity.
                        tokio::time::sleep(Duration::from_secs(5)).await;
                    }
                }
            }),
        );

        // Wait until the listener task is ready.
        let _ = handler.await;
    }

    ///
    /// Initialize a new instance of the heartbeat.
    ///
    async fn initialize_heartbeat(&self) {
        // Initialize the heartbeat process.
        let (router, handler) = oneshot::channel();
        let state = self.state.clone();
        E::resources().register_task(
            None, // No need to provide an id, as the task will run indefinitely.
            task::spawn(async move {
                // Notify the outer function that the task is ready.
                let _ = router.send(());
                loop {
                    // Transmit a heartbeat request to the ledger.
                    if let Err(error) = state.ledger().router().send(LedgerRequest::Heartbeat).await {
                        error!("Failed to send heartbeat to ledger: {}", error)
                    }
                    // Transmit a heartbeat request to the peers.
                    let request = PeersRequest::Heartbeat;
                    if let Err(error) = state.peers().router().send(request).await {
                        error!("Failed to send heartbeat to peers: {}", error)
                    }
                    // Sleep for `E::HEARTBEAT_IN_SECS` seconds.
                    tokio::time::sleep(Duration::from_secs(E::HEARTBEAT_IN_SECS)).await;
                }
            }),
        );

        // Wait until the heartbeat task is ready.
        let _ = handler.await;
    }

    ///
    /// Initialize a new instance of the RPC server.
    ///
    #[cfg(feature = "rpc")]
    async fn initialize_rpc(&self, node: &Node, address: Option<Address<N>>) {
        if !node.norpc {
            // Initialize a new instance of the RPC server.
            let rpc_context = RpcContext::new(node.rpc_username.clone(), node.rpc_password.clone(), address, self.state.clone());
            let (rpc_server_addr, rpc_server_handle) = initialize_rpc_server::<N, E>(node.rpc, rpc_context).await;

            debug!("JSON-RPC server listening on {}", rpc_server_addr);

            // Register the task; no need to provide an id, as it will run indefinitely.
            E::resources().register_task(None, rpc_server_handle);
        }
    }

    ///
    /// Initialize a new instance of the notification.
    ///
    async fn initialize_notification(&self, address: Option<Address<N>>) {
        // Initialize the heartbeat process.
        let (router, handler) = oneshot::channel();
        let state = self.state.clone();
        E::resources().register_task(
            None, // No need to provide an id, as the task will run indefinitely.
            task::spawn(async move {
                // Notify the outer function that the task is ready.
                let _ = router.send(());
                loop {
                    info!("{}", notification_message(address));

                    if E::NODE_TYPE == NodeType::Miner {
                        if let Some(miner_address) = address {
                            // Retrieve the latest block height.
                            let latest_block_height = state.ledger().reader().latest_block_height();

                            // Prepare a list of confirmed and pending coinbase records.
                            let mut confirmed = vec![];
                            let mut pending = vec![];

                            // Iterate through the coinbase records from storage.
                            for (block_height, record) in state.prover().to_coinbase_records() {
                                // Filter the coinbase records by determining if they exist on the canonical chain.
                                if let Ok(true) = state.ledger().reader().contains_commitment(&record.commitment()) {
                                    // Ensure the record owner matches.
                                    if record.owner() == miner_address {
                                        // Add the block to the appropriate list.
                                        match block_height + 2048 < latest_block_height {
                                            true => confirmed.push((block_height, record)),
                                            false => pending.push((block_height, record)),
                                        }
                                    }
                                }
                            }

                            info!(
                                "Mining Report (confirmed_blocks = {}, pending_blocks = {}, miner_address = {})",
                                confirmed.len(),
                                pending.len(),
                                miner_address
                            );
                        }
                    }

                    // Sleep for `120` seconds.
                    tokio::time::sleep(Duration::from_secs(120)).await;
                }
            }),
        );

        // Wait until the heartbeat task is ready.
        let _ = handler.await;
    }

    #[cfg(any(feature = "test", feature = "prometheus"))]
    fn initialize_metrics(ledger: LedgerReader<N>) {
        #[cfg(not(feature = "test"))]
        if let Some(handler) = snarkos_metrics::initialize() {
            // No need to provide an id, as the task will run indefinitely.
            E::resources().register_task(None, handler);
        }

        // Set the block height as it could already be non-zero.
        metrics::gauge!(metrics::blocks::HEIGHT, ledger.latest_block_height() as f64);
    }
}
