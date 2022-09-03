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

use crate::{
    connect_to_leader,
    environment::helpers::NodeType,
    handle_listener,
    handle_peer,
    request_genesis_block,
    send_pings,
    Account,
    Ledger,
    Prover,
    ProverHandler,
};
use snarkos_environment::{helpers::Status, Environment};
use snarkvm::prelude::Network;

use anyhow::{bail, Result};
use once_cell::race::OnceBox;
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::oneshot;

#[derive(Clone)]
pub struct Node<N: Network, E: Environment> {
    /// The ledger.
    ledger: Arc<Ledger<N>>,
    /// The prover.
    prover: Arc<OnceBox<Prover<N, E>>>,
}

impl<N: Network, E: Environment> Node<N, E> {
    /// Initializes a new instance of the node.
    pub async fn new(cli: &CLI, account: Account<N>) -> Result<Self> {
        // Initialize the ledger.
        let ledger = match cli.dev {
            None => {
                // Initialize the ledger.
                let ledger = Ledger::<N>::load(*account.private_key())?;
                // Sync the ledger with the network.
                ledger.initial_sync_with_network(cli.beacon_addr.ip()).await?;

                ledger
            }
            Some(_) => {
                // TODO (raychu86): Formalize this process via network messages.
                //  Currently this operations pulls from the leader's server.
                // Request genesis block from the beacon leader.
                let genesis_block = request_genesis_block::<N>(cli.beacon_addr.ip()).await?;

                // Initialize the ledger from the provided genesis block.
                Ledger::<N>::new_with_genesis(*account.private_key(), genesis_block)?
            }
        };

        // TODO (raychu86): Make `handle_listener`, `connect_to_leader` and `send_pings` dedicated `Node` methods.

        // Initialize the listener.
        let listener = tokio::net::TcpListener::bind(cli.node).await?;

        // Handle incoming connections.
        let _handle_listener = handle_listener::<N, E>(listener, ledger.clone());

        // Connect to the leader node and listen for new blocks.
        let leader_addr = cli.beacon_addr;
        trace!("Connecting to '{}'...", leader_addr);
        let _leader_conn_task = connect_to_leader::<N, E>(leader_addr, ledger.clone());

        // Send pings to all peers every 10 seconds.
        let _pings = send_pings::<N>(ledger.clone());

        let node = Self {
            ledger,
            prover: Arc::new(OnceBox::new()),
        };

        // Initialize a new instance for managing the prover.
        let (prover, prover_handler) = Prover::new(node.ledger().clone())?;

        // Initialize the prover.
        node.initialize_prover(prover, prover_handler).await;

        // Run the prover.
        node.prover().start_prover().await;

        Ok(node)
    }

    /// Sends a connection request to the given IP address.
    pub async fn connect_to(&self, peer_ip: SocketAddr) -> Result<()> {
        trace!("Attempting to connect to peer {}", peer_ip);
        match tokio::net::TcpStream::connect(peer_ip).await {
            Ok(stream) => {
                let ledger = self.ledger.clone();
                tokio::spawn(async move {
                    if let Err(err) = handle_peer::<N, E>(stream, peer_ip, ledger).await {
                        warn!("Failed to handle connection with peer {}: {:?}", peer_ip, err);
                    }
                });
                Ok(())
            }
            Err(error) => {
                warn!("Failed to connect to peer {}: {}", peer_ip, error);
                bail!("{error}")
            }
        }
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
        // self.state.ledger().shut_down().await;

        // Flush the tasks.
        E::resources().shut_down();
        trace!("Node has shut down.");
    }

    /// Initializes a new prover.
    pub async fn initialize_prover(&self, prover: Prover<N, E>, mut prover_handler: ProverHandler) {
        self.prover.set(prover.into()).map_err(|_| ()).unwrap();

        // Initialize the prover, if the node type is a prover.
        if E::NODE_TYPE == NodeType::Prover {
            let node = self.clone();
            let (router, handler) = oneshot::channel();

            E::resources().register_task(
                None, // No need to provide an id, as the task will run indefinitely.
                tokio::spawn(async move {
                    // Notify the outer function that the task is ready.
                    let _ = router.send(());

                    // Asynchronously wait for a prover request.
                    while let Some(request) = prover_handler.recv().await {
                        // Update the state of the prover.
                        node.prover().update(request).await;
                    }
                }),
            );

            // Wait until the prover handler is ready.
            let _ = handler.await;
        }
    }

    /// Returns the nodes ledger.
    pub(crate) fn ledger(&self) -> &Arc<Ledger<N>> {
        &self.ledger
    }

    /// Returns the nodes prover.
    pub(crate) fn prover(&self) -> &Prover<N, E> {
        self.prover.get().unwrap()
    }
}
