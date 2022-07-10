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

use crate::{
    message::DisconnectReason,
    peers::{Peers, PeersHandler, PeersRequest},
    Account,
};

use snarkos_environment::{helpers::Status, Environment};
use snarkvm::prelude::*;

#[cfg(feature = "rpc")]
use snarkos_rpc::{initialize_rpc_node, RpcContext};

#[cfg(any(feature = "test", feature = "prometheus"))]
use snarkos_metrics as metrics;

use anyhow::Result;
use once_cell::race::OnceBox;
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tokio::{net::TcpListener, sync::oneshot};

#[macro_export]
macro_rules! spawn_task {
    // Spawns a new task, without a task ID.
    ($logic:block) => {{
        let (router, handler) = tokio::sync::oneshot::channel();
        // Register the task with the environment.
        // No need to provide an id, as the task will run indefinitely.
        E::resources().register_task(None, tokio::task::spawn(async move {
            // Notify the outer function that the task is ready.
            let _ = router.send(());
            $logic
        }));
        // Wait until the task is ready.
        let _ = handler.await;
    }};

    // Spawns a new task, without a task ID.
    ($logic:expr) => {{ $crate::spawn_task!(None, { $logic }) }};

    // Spawns a new task, with a task ID.
    ($id:expr, $logic:block) => {{
        let (router, handler) = tokio::sync::oneshot::channel();
        // Register the task with the environment.
        E::resources().register_task(Some($id), tokio::task::spawn(async move {
            // Notify the outer function that the task is ready.
            let _ = router.send(());
            $logic
            E::resources().deregister($id);
        }));
        // Wait until the task is ready.
        let _ = handler.await;
    }};

    // Spawns a new task, with a task ID.
    ($id:expr, $logic:expr) => {{ $crate::spawn_task!($id, { $logic }) }};
}

#[derive(Clone)]
pub struct State<N: Network, E: Environment> {
    /// The local IP of the node.
    local_ip: Arc<SocketAddr>,
    /// The Aleo account of the node.
    account: Arc<Account<N>>,
    /// The list of peers for the node.
    peers: Arc<OnceBox<Peers<N, E>>>,
}

impl<N: Network, E: Environment> State<N, E> {
    /// Initializes a new `State` instance.
    pub async fn new(node_ip: SocketAddr, account: Account<N>) -> Result<Self> {
        // Initialize a new TCP listener at the given IP.
        let (local_ip, listener) = match TcpListener::bind(node_ip).await {
            Ok(listener) => (listener.local_addr().expect("Failed to fetch the local IP"), listener),
            Err(error) => panic!("Failed to bind listener: {:?}. Check if another Aleo node is running", error),
        };

        // Construct the state.
        let state = Self {
            local_ip: Arc::new(local_ip),
            account: Arc::new(account),
            peers: Default::default(),
        };

        // Initialize a new peers module.
        let (peers, peers_handler) = Peers::new(state.clone()).await;
        // Set the peers into state.
        state
            .peers
            .set(peers.into())
            .map_err(|_| anyhow!("Failed to set peers into state"))?;
        // Initialize the peers.
        state.initialize_peers(peers_handler).await;

        // Initialize the listener.
        state.initialize_listener(listener).await;
        // Initialize the heartbeat.
        state.initialize_heartbeat().await;

        Ok(state)
    }

    /// Returns the IP address of this node.
    pub fn local_ip(&self) -> &SocketAddr {
        &self.local_ip
    }

    /// Returns the Aleo address of this node.
    pub fn address(&self) -> &Address<N> {
        self.account.address()
    }

    /// Returns the peers module of this node.
    pub fn peers(&self) -> &Peers<N, E> {
        &self.peers.get().unwrap()
    }

    /// Returns `true` if the given IP is this node.
    pub fn is_local_ip(&self, ip: &SocketAddr) -> bool {
        *ip == *self.local_ip || (ip.ip().is_unspecified() || ip.ip().is_loopback()) && ip.port() == self.local_ip.port()
    }
}

impl<N: Network, E: Environment> State<N, E> {
    ///
    /// Initialize the connection listener for new peers.
    ///
    async fn initialize_peers(&self, mut peers_handler: PeersHandler<N>) {
        let state = self.clone();
        spawn_task!({
            // Asynchronously wait for a peers request.
            while let Some(request) = peers_handler.recv().await {
                let state = state.clone();
                // Procure a resource ID for the task, as it may terminate at any time.
                let resource_id = E::resources().procure_id();
                // Asynchronously process a peers request.
                E::resources().register_task(
                    Some(resource_id),
                    tokio::spawn(async move {
                        // Update the state of the peers.
                        state.peers().update(request).await;

                        E::resources().deregister(resource_id);
                    }),
                );
            }
        });
    }

    ///
    /// Initialize the connection listener for new peers.
    ///
    async fn initialize_listener(&self, listener: TcpListener) {
        let state = self.clone();
        spawn_task!({
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
        });
    }

    ///
    /// Initialize a new instance of the heartbeat.
    ///
    async fn initialize_heartbeat(&self) {
        let state = self.clone();
        spawn_task!({
            loop {
                // // Transmit a heartbeat request to the ledger.
                // if let Err(error) = state.ledger().router().send(LedgerRequest::Heartbeat).await {
                //     error!("Failed to send heartbeat to ledger: {}", error)
                // }
                // Transmit a heartbeat request to the peers.
                if let Err(error) = state.peers().router().send(PeersRequest::Heartbeat).await {
                    error!("Failed to send heartbeat to peers: {}", error)
                }
                // Sleep for `E::HEARTBEAT_IN_SECS` seconds.
                tokio::time::sleep(Duration::from_secs(E::HEARTBEAT_IN_SECS)).await;
            }
        });
    }
}

#[derive(Clone)]
pub struct Node<N: Network, E: Environment> {
    /// The current state of the node.
    state: State<N, E>,
}

impl<N: Network, E: Environment> Node<N, E> {
    /// Initializes a new instance of the node.
    pub async fn new(node_ip: SocketAddr, account: Account<N>) -> Result<Self> {
        // Initialize the state.
        let state = State::new(node_ip, account).await?;

        let node = Self { state };

        // /// Returns the storage path of the ledger.
        // pub(crate) fn ledger_storage_path(&self, _local_ip: SocketAddr) -> PathBuf {
        //     if cfg!(feature = "test") {
        //         // Tests may use any available ports, and removes the storage artifacts afterwards,
        //         // so that there is no need to adhere to a specific number assignment logic.
        //         PathBuf::from(format!("/tmp/snarkos-test-ledger-{}", _local_ip.port()))
        //     } else {
        //         aleo_std::aleo_ledger_dir(self.network, self.dev)
        //     }
        // }
        //
        // /// Returns the storage path of the validator.
        // pub(crate) fn validator_storage_path(&self, _local_ip: SocketAddr) -> PathBuf {
        //     if cfg!(feature = "test") {
        //         // Tests may use any available ports, and removes the storage artifacts afterwards,
        //         // so that there is no need to adhere to a specific number assignment logic.
        //         PathBuf::from(format!("/tmp/snarkos-test-validator-{}", _local_ip.port()))
        //     } else {
        //         // TODO (howardwu): Rename to validator.
        //         aleo_std::aleo_operator_dir(self.network, self.dev)
        //     }
        // }
        //
        // /// Returns the storage path of the prover.
        // pub(crate) fn prover_storage_path(&self, _local_ip: SocketAddr) -> PathBuf {
        //     if cfg!(feature = "test") {
        //         // Tests may use any available ports, and removes the storage artifacts afterwards,
        //         // so that there is no need to adhere to a specific number assignment logic.
        //         PathBuf::from(format!("/tmp/snarkos-test-prover-{}", _local_ip.port()))
        //     } else {
        //         aleo_std::aleo_prover_dir(self.network, self.dev)
        //     }
        // }
        //
        // // Initialize the ledger storage path.
        // let ledger_storage_path = node.ledger_storage_path(local_ip);
        // // Initialize the prover storage path.
        // let prover_storage_path = node.prover_storage_path(local_ip);
        // // Initialize the validator storage path.
        // let validator_storage_path = node.validator_storage_path(local_ip);

        // // Initialize a new instance for managing the ledger.
        // let (ledger, ledger_handler) = Ledger::<N, E>::open::<_>(&ledger_storage_path, state.clone()).await?;
        //
        // // Initialize a new instance for managing the prover.
        // let (prover, prover_handler) = Prover::open::<_>(&prover_storage_path, state.clone()).await?;
        //
        // // Initialize a new instance for managing the validator.
        // let (validator, validator_handler) = Operator::open::<_>(&validator_storage_path, state.clone()).await?;

        // // Initialise the metrics exporter.
        // #[cfg(any(feature = "test", feature = "prometheus"))]
        // Self::initialize_metrics(ledger.reader().clone());

        // node.state.initialize_ledger(ledger, ledger_handler).await;
        // node.state.initialize_prover(prover, prover_handler).await;
        // node.state.initialize_validator(validator, validator_handler).await;

        // node.state.validator().initialize().await;

        // node.initialize_notification(address).await;
        // node.initialize_rpc(node, address).await;

        Ok(node)
    }

    /// Returns the IP address of this node.
    pub fn local_ip(&self) -> &SocketAddr {
        self.state.local_ip()
    }

    /// Returns the Aleo address of this node.
    pub fn address(&self) -> &Address<N> {
        self.state.address()
    }

    /// Returns the peers module of this node.
    pub fn peers(&self) -> &Peers<N, E> {
        self.state.peers()
    }

    /// Sends a connection request to the given IP address.
    pub async fn connect_to(&self, peer_ip: SocketAddr) -> Result<()> {
        // Initialize the connection process.
        let (router, handler) = oneshot::channel();

        // Route a `Connect` request to the peer manager.
        self.peers().router().send(PeersRequest::Connect(peer_ip, router)).await?;

        // Wait until the connection task is initialized.
        handler.await.map(|_| ()).map_err(|e| e.into())
    }

    #[inline]
    pub async fn disconnect_from(&self, _peer_ip: SocketAddr, _reason: DisconnectReason) {
        // self.state.ledger().disconnect(peer_ip, reason).await
        // TODO (raychu86): Handle the disconnect case.
        unimplemented!()
    }

    ///
    /// Initialize a new instance of the RPC node.
    ///
    #[cfg(feature = "rpc")]
    async fn initialize_rpc(&self, cli: &CLI, address: Option<Address<N>>) {
        if !cli.norpc {
            // Initialize a new instance of the RPC node.
            let rpc_context = RpcContext::new(cli.rpc_username.clone(), cli.rpc_password.clone(), address, self.state.clone());
            let (rpc_node_addr, rpc_node_handle) = initialize_rpc_node::<N, E>(cli.rpc, rpc_context).await;

            debug!("JSON-RPC node listening on {}", rpc_node_addr);

            // Register the task; no need to provide an id, as it will run indefinitely.
            E::resources().register_task(None, rpc_node_handle);
        }
    }

    // #[cfg(any(feature = "test", feature = "prometheus"))]
    // fn initialize_metrics(ledger: LedgerReader<N>) {
    //     #[cfg(not(feature = "test"))]
    //     if let Some(handler) = snarkos_metrics::initialize() {
    //         // No need to provide an id, as the task will run indefinitely.
    //         E::resources().register_task(None, handler);
    //     }
    //
    //     // Set the block height as it could already be non-zero.
    //     metrics::gauge!(metrics::blocks::HEIGHT, ledger.latest_block_height() as f64);
    // }

    ///
    /// Disconnects from peers and proceeds to shut down the node.
    ///
    pub async fn shut_down(&self) {
        info!("Shutting down...");
        // Update the node status.
        E::status().update(Status::ShuttingDown);

        // Shut down the ledger.
        trace!("Proceeding to shut down the ledger...");
        // self.state.ledger().shut_down().await;

        // Flush the tasks.
        E::resources().shut_down();
        trace!("Node has shut down.");
    }
}
