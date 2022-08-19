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

use crate::{connect_to_leader, handle_listener, send_pings, Account, Ledger};
use snarkos_environment::{helpers::Status, Environment};
use snarkvm::prelude::{Network};

use anyhow::Result;
use core::marker::PhantomData;
use std::{net::SocketAddr, sync::Arc, str::FromStr};

// The IP of the leader node to connect to.
const LEADER_IP: &str = "159.203.77.113:4000";

#[derive(Clone)]
pub struct Node<N: Network, E: Environment> {
    /// The ledger.
    ledger: Arc<Ledger<N>>,
    /// PhantomData.
    _phantom: PhantomData<(N, E)>,
}

impl<N: Network, E: Environment> Node<N, E> {
    /// Initializes a new instance of the node.
    pub async fn new(cli: &CLI, account: Account<N>) -> Result<Self> {
        // Initialize the ledger.
        let ledger = Ledger::<N>::load(account.private_key()).await?;

        // Initialize the listener.
        let listener = tokio::net::TcpListener::bind(cli.node).await?;

        // Handle incoming connections.
        let _handle_listener = handle_listener::<N>(listener, ledger.clone()).await;

        // Connect to the leader node and listen for new blocks.
        let leader_addr = SocketAddr::from_str(&LEADER_IP)?;
        let _ = connect_to_leader::<N>(leader_addr, ledger.clone()).await;

        debug!("Connecting to '{}'...", leader_addr);

        // This will prevent the node from generating blocks and will maintain a connection with the leader.
        // Send pings to all peers every 10 seconds.
        let _pings = send_pings::<N>(ledger.clone()).await;

        Ok(Self { ledger: ledger.clone(), _phantom: PhantomData })
    }

    // /// Returns the peers module of this node.
    // pub fn peers(&self) -> &Peers<N, E> {
    //     self.state.peers()
    // }
    //
    // /// Sends a connection request to the given IP address.
    // pub async fn connect_to(&self, peer_ip: SocketAddr) -> Result<()> {
    //     // Initialize the connection process.
    //     let (router, handler) = oneshot::channel();
    //
    //     // Route a `Connect` request to the peer manager.
    //     self.peers().router().send(PeersRequest::Connect(peer_ip, router)).await?;
    //
    //     // Wait until the connection task is initialized.
    //     handler.await.map(|_| ()).map_err(|e| e.into())
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
