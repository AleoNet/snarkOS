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

use crate::CLI;

use snarkos_consensus::account::Account;
use snarkos_environment::{helpers::Status, Environment};
use snarkos_network::{message::*, peers::*, state::State};
use snarkvm::prelude::*;

#[cfg(feature = "rpc")]
use snarkos_rpc::{initialize_rpc_server, RpcContext};

#[cfg(any(feature = "test", feature = "prometheus"))]
use snarkos_metrics as metrics;

use anyhow::Result;
use std::net::SocketAddr;
use tokio::sync::oneshot;

#[derive(Clone)]
pub struct Node<N: Network, E: Environment> {
    /// The current state of the node.
    state: State<N, E>,
}

impl<N: Network, E: Environment> Node<N, E> {
    /// Initializes a new instance of the node.
    pub async fn new(cli: &CLI, account: Account<N>) -> Result<Self> {
        let address = account.address().clone();

        // Initialize the state.
        let state = State::new(cli.node, account).await?;

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

        #[cfg(feature = "rpc")]
        node.initialize_rpc(&cli, Some(address.clone())).await;

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
            let (rpc_node_addr, rpc_node_handle) = initialize_rpc_server::<N, E>(cli.rpc, rpc_context).await;

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
