// Copyright (C) 2019-2021 Aleo Systems Inc.
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
    helpers::Tasks,
    ledger::{Ledger, LedgerRequest, LedgerRouter},
    peers::{Peers, PeersRequest, PeersRouter},
    rpc::initialize_rpc_server,
    Environment,
    NodeType,
};
use snarkos_ledger::{storage::rocksdb::RocksDB, LedgerState};
use snarkvm::prelude::*;

use anyhow::Result;
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tokio::{
    net::TcpListener,
    sync::{mpsc, RwLock},
    task,
};

///
/// A set of operations to initialize the node server for a specific network.
///
pub(crate) struct Server<N: Network, E: Environment> {
    /// The list of peers for the node.
    peers: Arc<RwLock<Peers<N, E>>>,
    /// The peers router of the node.
    peers_router: PeersRouter<N, E>,
    /// The ledger state of the node.
    ledger: Arc<RwLock<Ledger<N, E>>>,
    /// The ledger router of the node.
    ledger_router: LedgerRouter<N, E>,
    /// The list of tasks spawned by the node.
    tasks: Tasks<task::JoinHandle<()>>,
}

impl<N: Network, E: Environment> Server<N, E> {
    ///
    /// Starts the connection listener for peers.
    ///
    #[inline]
    pub(crate) async fn initialize(
        node_port: u16,
        rpc_port: u16,
        username: String,
        password: String,
        miner: Option<Address<N>>,
    ) -> Result<Self> {
        // Initialize a new TCP listener at the given IP.
        let (local_ip, listener) = match TcpListener::bind(&format!("0.0.0.0:{}", node_port)).await {
            Ok(listener) => (listener.local_addr().expect("Failed to fetch the local IP"), listener),
            Err(error) => panic!("Failed to bind listener: {:?}. Check if another Aleo node is running", error),
        };

        // Initialize the ledger storage path.
        let storage_path = format!(".ledger-{}", (node_port as u16 - 4130) as u8);

        // Initialize the tasks handler.
        let mut tasks = Tasks::new();

        // Initialize a new instance for managing peers.
        let (peers, peers_router) = Self::initialize_peers(&mut tasks, local_ip);
        // Initialize a new instance for managing the ledger.
        let (ledger, ledger_router) = Self::initialize_ledger(&mut tasks, &storage_path, &peers_router)?;

        // Initialize the connection listener for new peers.
        Self::initialize_listener(&mut tasks, local_ip, listener, &peers_router, &ledger_router);
        // Initialize a new instance of the heartbeat.
        Self::initialize_heartbeat(&mut tasks, &peers_router, &ledger_router);
        // Initialize a new instance of the miner.
        Self::initialize_miner(&mut tasks, local_ip, miner, &ledger_router);

        tokio::time::sleep(Duration::from_secs(2)).await;

        // Initialize a new instance of the RPC server.
        tasks.append(initialize_rpc_server::<N, E>(
            format!("0.0.0.0:{}", rpc_port).parse()?,
            username,
            password,
            LedgerState::open::<RocksDB, _>(&storage_path, true)?,
            &ledger_router,
        ));

        Ok(Self {
            peers,
            peers_router,
            ledger,
            ledger_router,
            tasks,
        })
    }

    ///
    /// Sends a connection request to the given IP address.
    ///
    #[inline]
    pub async fn connect_to(&self, peer_ip: SocketAddr) -> Result<()> {
        let message = PeersRequest::Connect(peer_ip, self.ledger_router.clone());
        self.peers_router.send(message).await?;
        Ok(())
    }

    ///
    /// Disconnects from peers and proceeds to shut down the node.
    ///
    #[inline]
    pub(crate) fn shut_down(&self) {
        info!("Shutting down...");
        self.tasks.flush();
    }

    ///
    /// Initialize a new instance for managing peers.
    ///
    #[inline]
    fn initialize_peers(tasks: &mut Tasks<task::JoinHandle<()>>, local_ip: SocketAddr) -> (Arc<RwLock<Peers<N, E>>>, PeersRouter<N, E>) {
        // Initialize the `Peers` struct.
        let peers = Arc::new(RwLock::new(Peers::new(local_ip)));

        // Initialize an mpsc channel for sending requests to the `Peers` struct.
        let (peers_router, mut peers_handler) = mpsc::channel(1024);

        // Initialize the peers router process.
        let peers_clone = peers.clone();
        let peers_router_clone = peers_router.clone();
        tasks.append(task::spawn(async move {
            // Asynchronously wait for a peers request.
            loop {
                tokio::select! {
                    // Channel is routing a request to peers.
                    Some(request) = peers_handler.recv() => {
                        // Hold the peers write lock briefly, to update the state of the peers.
                        peers_clone.write().await.update(request, &peers_router_clone).await;
                    }
                }
            }
        }));

        (peers, peers_router)
    }

    ///
    /// Initialize a new instance for managing the ledger.
    ///
    #[inline]
    fn initialize_ledger(
        tasks: &mut Tasks<task::JoinHandle<()>>,
        storage_path: &str,
        peers_router: &PeersRouter<N, E>,
    ) -> Result<(Arc<RwLock<Ledger<N, E>>>, LedgerRouter<N, E>)> {
        // Open the ledger from storage.
        let ledger = Ledger::<N, E>::open::<RocksDB, _>(storage_path)?;
        let ledger = Arc::new(RwLock::new(ledger));

        // Initialize an mpsc channel for sending requests to the `Ledger` struct.
        let (ledger_router, mut ledger_handler) = mpsc::channel(1024);

        // Initialize the ledger router process.
        let peers_router = peers_router.clone();
        let ledger_clone = ledger.clone();
        tasks.append(task::spawn(async move {
            // Asynchronously wait for a ledger request.
            while let Some(request) = ledger_handler.recv().await {
                // Hold the ledger write lock briefly, to update the state of the ledger.
                ledger_clone.write().await.update(request, &peers_router).await;
            }
        }));

        Ok((ledger, ledger_router))
    }

    ///
    /// Initialize the connection listener for new peers.
    ///
    #[inline]
    fn initialize_listener(
        tasks: &mut Tasks<task::JoinHandle<()>>,
        local_ip: SocketAddr,
        listener: TcpListener,
        peers_router: &PeersRouter<N, E>,
        ledger_router: &LedgerRouter<N, E>,
    ) {
        let peers_router = peers_router.clone();
        let ledger_router = ledger_router.clone();
        tasks.append(task::spawn(async move {
            info!("Listening for peers at {}", local_ip);
            loop {
                // Asynchronously wait for an inbound TcpStream.
                match listener.accept().await {
                    // Process the inbound connection request.
                    Ok((stream, peer_ip)) => {
                        let request = PeersRequest::PeerConnecting(stream, peer_ip, ledger_router.clone());
                        if let Err(error) = peers_router.send(request).await {
                            error!("Failed to send request to peers: {}", error)
                        }
                    }
                    Err(error) => error!("Failed to accept a connection: {}", error),
                }
                // Add a small delay to prevent overloading the network from handshakes.
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }));
    }

    ///
    /// Initialize a new instance of the heartbeat.
    ///
    #[inline]
    fn initialize_heartbeat(tasks: &mut Tasks<task::JoinHandle<()>>, peers_router: &PeersRouter<N, E>, ledger_router: &LedgerRouter<N, E>) {
        let peers_router = peers_router.clone();
        let ledger_router = ledger_router.clone();
        tasks.append(task::spawn(async move {
            loop {
                // Transmit a heartbeat request to the peers.
                let request = PeersRequest::Heartbeat(ledger_router.clone());
                if let Err(error) = peers_router.send(request).await {
                    error!("Failed to send request to peers: {}", error)
                }
                // Transmit a heartbeat request to the ledger.
                let request = LedgerRequest::Heartbeat;
                if let Err(error) = ledger_router.send(request).await {
                    error!("Failed to send request to ledger: {}", error)
                }
                // Sleep for 5 seconds.
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }));
    }

    ///
    /// Initialize a new instance of the miner.
    ///
    #[inline]
    fn initialize_miner(
        tasks: &mut Tasks<task::JoinHandle<()>>,
        local_ip: SocketAddr,
        miner: Option<Address<N>>,
        ledger_router: &LedgerRouter<N, E>,
    ) {
        if E::NODE_TYPE == NodeType::Miner {
            if let Some(recipient) = miner {
                let ledger_router = ledger_router.clone();
                tasks.append(task::spawn(async move {
                    loop {
                        // Start the mining process.
                        let request = LedgerRequest::Mine(local_ip, recipient, ledger_router.clone());
                        if let Err(error) = ledger_router.send(request).await {
                            error!("Failed to send request to ledger: {}", error);
                        }
                        // Sleep for 2 seconds.
                        tokio::time::sleep(Duration::from_secs(2)).await;
                    }
                }));
            } else {
                error!("Missing miner address. Please specify an Aleo address in order to mine");
            }
        }
    }
}
