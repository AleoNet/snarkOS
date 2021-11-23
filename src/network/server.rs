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
    helpers::{Status, Tasks},
    ledger::{Ledger, LedgerRequest, LedgerRouter},
    peers::{Peers, PeersRequest, PeersRouter},
    rpc::initialize_rpc_server,
    Environment,
    Node,
    NodeType,
};
use snarkos_ledger::{storage::rocksdb::RocksDB, LedgerState};
use snarkvm::prelude::*;

use anyhow::Result;
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tokio::{
    net::TcpListener,
    sync::{mpsc, oneshot, RwLock},
    task,
};

pub type LedgerReader<N> = Arc<RwLock<LedgerState<N>>>;

///
/// A set of operations to initialize the node server for a specific network.
///
#[derive(Clone)]
pub struct Server<N: Network, E: Environment> {
    /// The local address of the node.
    local_ip: SocketAddr,
    /// The status of the node.
    status: Status,
    /// The list of peers for the node.
    peers: Arc<RwLock<Peers<N, E>>>,
    /// The peers router of the node.
    peers_router: PeersRouter<N, E>,
    /// The ledger state of the node.
    ledger_reader: LedgerReader<N>,
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
    pub async fn initialize(node: &Node, miner: Option<Address<N>>, mut tasks: Tasks<task::JoinHandle<()>>) -> Result<Self> {
        // Initialize a new TCP listener at the given IP.
        let (local_ip, listener) = match TcpListener::bind(node.node).await {
            Ok(listener) => (listener.local_addr().expect("Failed to fetch the local IP"), listener),
            Err(error) => panic!("Failed to bind listener: {:?}. Check if another Aleo node is running", error),
        };

        // Initialize the ledger storage path.
        let storage_path = node.storage_path(local_ip);
        // Initialize the status indicator.
        let status = Status::new();
        // Initialize a new instance for managing peers.
        let (peers, peers_router) = Self::initialize_peers(&mut tasks, local_ip, status.clone()).await;
        // Initialize a new instance for managing the ledger.
        let (ledger_reader, ledger_router) = Self::initialize_ledger(&mut tasks, storage_path, status.clone(), &peers_router).await?;

        // Initialize the connection listener for new peers.
        Self::initialize_listener(&mut tasks, local_ip, listener, &peers_router, &ledger_reader, &ledger_router).await;
        // Initialize a new instance of the heartbeat.
        Self::initialize_heartbeat(&mut tasks, &peers_router, &ledger_reader, &ledger_router).await;
        // Initialize a new instance of the miner.
        Self::initialize_miner(&mut tasks, local_ip, miner, &ledger_router).await;
        // Initialize a new instance of the RPC server.
        Self::initialize_rpc(&mut tasks, node, &status, &peers, &ledger_reader, &ledger_router).await;

        Ok(Self {
            local_ip,
            status,
            peers,
            peers_router,
            ledger_reader,
            ledger_router,
            tasks,
        })
    }

    /// Returns the IP address of this node.
    pub fn local_ip(&self) -> SocketAddr {
        self.local_ip
    }

    /// Returns the status of this node.
    pub fn status(&self) -> Status {
        self.status.clone()
    }

    /// Returns the peer manager of this node.
    pub fn peers(&self) -> Arc<RwLock<Peers<N, E>>> {
        self.peers.clone()
    }

    ///
    /// Sends a connection request to the given IP address.
    ///
    #[inline]
    pub async fn connect_to(&self, peer_ip: SocketAddr) -> Result<()> {
        // Initialize the connection process.
        let (router, handler) = oneshot::channel();

        // Route a `Connect` request to the peer manager.
        let message = PeersRequest::Connect(peer_ip, self.ledger_reader.clone(), self.ledger_router.clone(), router);
        self.peers_router.send(message).await?;

        // Wait until the connection task is initialized.
        handler.await.map(|_| ()).map_err(|e| e.into())
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
    #[allow(clippy::type_complexity)]
    async fn initialize_peers(
        tasks: &mut Tasks<task::JoinHandle<()>>,
        local_ip: SocketAddr,
        local_status: Status,
    ) -> (Arc<RwLock<Peers<N, E>>>, PeersRouter<N, E>) {
        // Initialize the `Peers` struct.
        let peers = Arc::new(RwLock::new(Peers::new(local_ip, None, local_status)));

        // Initialize an mpsc channel for sending requests to the `Peers` struct.
        let (peers_router, mut peers_handler) = mpsc::channel(1024);

        // Initialize the peers router process.
        {
            let peers = peers.clone();
            let peers_router = peers_router.clone();
            let tasks_clone = tasks.clone();
            let (router, handler) = oneshot::channel();
            tasks.append(task::spawn(async move {
                // Notify the outer function that the task is ready.
                let _ = router.send(());
                // Asynchronously wait for a peers request.
                while let Some(request) = peers_handler.recv().await {
                    let peers = peers.clone();
                    let peers_router = peers_router.clone();
                    // Asynchronously process a peers request.
                    tasks_clone.append(task::spawn(async move {
                        // Hold the peers write lock briefly, to update the state of the peers.
                        peers.write().await.update(request, &peers_router).await;
                    }));
                }
            }));
            // Wait until the peers router task is ready.
            let _ = handler.await;
        }

        (peers, peers_router)
    }

    ///
    /// Initialize a new instance for managing the ledger.
    ///
    #[inline]
    async fn initialize_ledger(
        tasks: &mut Tasks<task::JoinHandle<()>>,
        storage_path: String,
        status: Status,
        peers_router: &PeersRouter<N, E>,
    ) -> Result<(LedgerReader<N>, LedgerRouter<N, E>)> {
        // Open the ledger from storage.
        let path = storage_path.clone();
        let ledger = Arc::new(RwLock::new(
            task::spawn_blocking(move || Ledger::<N, E>::open::<RocksDB, _>(&path, &status)).await??,
        ));

        // Open a ledger reader from storage.
        let ledger_reader = Arc::new(RwLock::new(LedgerState::<N>::open_reader::<RocksDB, _>(&storage_path)?));

        // Initialize an mpsc channel for sending requests to the `Ledger` struct.
        let (ledger_router, mut ledger_handler) = mpsc::channel(1024);

        // Initialize the ledger router process.
        let peers_router = peers_router.clone();
        let (router, handler) = oneshot::channel();
        tasks.append(task::spawn(async move {
            // Notify the outer function that the task is ready.
            let _ = router.send(());
            // Asynchronously wait for a ledger request.
            while let Some(request) = ledger_handler.recv().await {
                // Hold the ledger write lock briefly, to update the state of the ledger.
                // Note: Do not wrap this call in a `task::spawn` as `BlockResponse` messages
                // will end up being processed out of order.
                ledger.write().await.update(request, &peers_router).await;
            }
        }));
        // Wait until the ledger router task is ready.
        let _ = handler.await;

        Ok((ledger_reader, ledger_router))
    }

    ///
    /// Initialize the connection listener for new peers.
    ///
    #[inline]
    async fn initialize_listener(
        tasks: &mut Tasks<task::JoinHandle<()>>,
        local_ip: SocketAddr,
        listener: TcpListener,
        peers_router: &PeersRouter<N, E>,
        ledger_reader: &LedgerReader<N>,
        ledger_router: &LedgerRouter<N, E>,
    ) {
        // Initialize the listener process.
        let peers_router = peers_router.clone();
        let ledger_reader = ledger_reader.clone();
        let ledger_router = ledger_router.clone();
        let (router, handler) = oneshot::channel();
        tasks.append(task::spawn(async move {
            // Notify the outer function that the task is ready.
            let _ = router.send(());
            info!("Listening for peers at {}", local_ip);
            loop {
                // Asynchronously wait for an inbound TcpStream.
                match listener.accept().await {
                    // Process the inbound connection request.
                    Ok((stream, peer_ip)) => {
                        let request = PeersRequest::PeerConnecting(stream, peer_ip, ledger_reader.clone(), ledger_router.clone());
                        if let Err(error) = peers_router.send(request).await {
                            error!("Failed to send request to peers: {}", error)
                        }
                    }
                    Err(error) => error!("Failed to accept a connection: {}", error),
                }
                // Add a small delay to prevent overloading the network from handshakes.
                tokio::time::sleep(Duration::from_millis(150)).await;
            }
        }));
        // Wait until the listener task is ready.
        let _ = handler.await;
    }

    ///
    /// Initialize a new instance of the heartbeat.
    ///
    #[inline]
    async fn initialize_heartbeat(
        tasks: &mut Tasks<task::JoinHandle<()>>,
        peers_router: &PeersRouter<N, E>,
        ledger_reader: &LedgerReader<N>,
        ledger_router: &LedgerRouter<N, E>,
    ) {
        // Initialize the heartbeat process.
        let peers_router = peers_router.clone();
        let ledger_reader = ledger_reader.clone();
        let ledger_router = ledger_router.clone();
        let (router, handler) = oneshot::channel();
        tasks.append(task::spawn(async move {
            // Notify the outer function that the task is ready.
            let _ = router.send(());
            loop {
                // Transmit a heartbeat request to the peers.
                let request = PeersRequest::Heartbeat(ledger_reader.clone(), ledger_router.clone());
                if let Err(error) = peers_router.send(request).await {
                    error!("Failed to send heartbeat to peers: {}", error)
                }
                // Transmit a heartbeat request to the ledger.
                let request = LedgerRequest::Heartbeat(ledger_router.clone());
                if let Err(error) = ledger_router.send(request).await {
                    error!("Failed to send heartbeat to ledger: {}", error)
                }
                // Sleep for `E::HEARTBEAT_IN_SECS` seconds.
                tokio::time::sleep(Duration::from_secs(E::HEARTBEAT_IN_SECS)).await;
            }
        }));
        // Wait until the heartbeat task is ready.
        let _ = handler.await;
    }

    ///
    /// Initialize a new instance of the miner.
    ///
    #[inline]
    async fn initialize_miner(
        tasks: &mut Tasks<task::JoinHandle<()>>,
        local_ip: SocketAddr,
        miner: Option<Address<N>>,
        ledger_router: &LedgerRouter<N, E>,
    ) {
        if E::NODE_TYPE == NodeType::Miner {
            if let Some(recipient) = miner {
                // Initialize the miner process.
                let ledger_router = ledger_router.clone();
                let (router, handler) = oneshot::channel();
                tasks.append(task::spawn(async move {
                    // Notify the outer function that the task is ready.
                    let _ = router.send(());
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
                // Wait until the miner task is ready.
                let _ = handler.await;
            } else {
                error!("Missing miner address. Please specify an Aleo address in order to mine");
            }
        }
    }

    ///
    /// Initialize a new instance of the RPC server.
    ///
    #[inline]
    async fn initialize_rpc(
        tasks: &mut Tasks<task::JoinHandle<()>>,
        node: &Node,
        status: &Status,
        peers: &Arc<RwLock<Peers<N, E>>>,
        ledger_reader: &LedgerReader<N>,
        ledger_router: &LedgerRouter<N, E>,
    ) {
        if !node.norpc {
            // Initialize a new instance of the RPC server.
            tasks.append(
                initialize_rpc_server::<N, E>(
                    node.rpc,
                    node.rpc_username.clone(),
                    node.rpc_password.clone(),
                    status,
                    peers,
                    ledger_reader,
                    ledger_router,
                )
                .await,
            );
        }
    }
}
