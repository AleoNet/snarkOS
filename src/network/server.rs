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
    display::notification_message,
    helpers::{State, Status, Tasks},
    ledger::{Ledger, LedgerRequest, LedgerRouter},
    peers::{Peers, PeersRequest, PeersRouter},
    prover::{Prover, ProverRouter},
    rpc::initialize_rpc_server,
    Environment,
    Node,
    NodeType,
    Pool,
    PoolRouter,
};
use snarkos_storage::{storage::rocksdb::RocksDB, LedgerState};
use snarkvm::prelude::*;

use anyhow::Result;
use std::{
    net::SocketAddr,
    sync::{atomic::AtomicBool, Arc},
    time::Duration,
};
use tokio::{
    net::TcpListener,
    sync::{oneshot, RwLock},
    task,
};

pub type LedgerReader<N> = Arc<LedgerState<N>>;

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
    peers: Arc<Peers<N, E>>,
    /// The ledger of the node.
    ledger: Arc<Ledger<N, E>>,
    /// The prover of the node.
    prover: Arc<Prover<N, E>>,
    /// The pool of the node.
    pool: Arc<Pool<N, E>>,
    /// The list of tasks spawned by the node.
    tasks: Tasks<task::JoinHandle<()>>,
}

impl<N: Network, E: Environment> Server<N, E> {
    ///
    /// Starts the connection listener for peers.
    ///
    #[inline]
    pub async fn initialize(
        node: &Node,
        address: Option<Address<N>>,
        pool_ip: Option<SocketAddr>,
        mut tasks: Tasks<task::JoinHandle<()>>,
    ) -> Result<Self> {
        // Initialize a new TCP listener at the given IP.
        let (local_ip, listener) = match TcpListener::bind(node.node).await {
            Ok(listener) => (listener.local_addr().expect("Failed to fetch the local IP"), listener),
            Err(error) => panic!("Failed to bind listener: {:?}. Check if another Aleo node is running", error),
        };

        // Initialize the ledger storage path.
        let ledger_storage_path = node.ledger_storage_path(local_ip);
        // Initialize the prover storage path.
        let prover_storage_path = node.prover_storage_path(local_ip);
        // Initialize the pool storage path.
        let pool_storage_path = prover_storage_path.join("-pool");

        // Initialize the status indicator.
        let status = Status::new();
        // Initialize the terminator bit.
        let terminator = Arc::new(AtomicBool::new(false));

        // Initialize a new instance for managing peers.
        let peers = Peers::new(tasks.clone(), local_ip, None, &status).await;
        // Initialize a new instance for managing the ledger.
        let ledger = Ledger::<N, E>::open::<RocksDB, _>(&mut tasks, &ledger_storage_path, &status, &terminator, peers.router()).await?;
        // Initialize a new instance for managing the prover.
        let prover = Prover::open::<RocksDB, _>(
            &mut tasks,
            &prover_storage_path,
            address,
            local_ip,
            pool_ip,
            &status,
            &terminator,
            peers.router(),
            ledger.reader(),
            ledger.router(),
        )
        .await?;

        let pool = Pool::open::<RocksDB, _>(
            &mut tasks,
            &pool_storage_path,
            address.clone(),
            local_ip,
            prover.memory_pool(),
            peers.router(),
            ledger.reader(),
            ledger.router(),
            prover.router(),
        )
        .await?;

        // Initialize the connection listener for new peers.
        Self::initialize_listener(
            &mut tasks,
            local_ip,
            listener,
            peers.router(),
            peers.clone(),
            ledger.reader(),
            ledger.router(),
            prover.router(),
            pool.router(),
        )
        .await;
        // Initialize a new instance of the heartbeat.
        Self::initialize_heartbeat(
            &mut tasks,
            peers.router(),
            ledger.reader(),
            ledger.router(),
            prover.router(),
            pool.router(),
        )
        .await;
        // Initialize a new instance of the RPC server.
        Self::initialize_rpc(
            &mut tasks,
            node,
            &status,
            &peers,
            ledger.reader(),
            prover.router(),
            prover.memory_pool(),
        )
        .await;
        // Initialize a new instance of the notification.
        Self::initialize_notification(&mut tasks, ledger.reader(), prover.clone(), address).await;

        Ok(Self {
            local_ip,
            status,
            peers,
            ledger,
            prover,
            pool,
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
    pub fn peers(&self) -> Arc<Peers<N, E>> {
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
        self.peers
            .router()
            .send(PeersRequest::Connect(
                peer_ip,
                self.ledger.reader(),
                self.ledger.router(),
                self.prover.router(),
                self.pool.router(),
                router,
            ))
            .await?;

        // Wait until the connection task is initialized.
        handler.await.map(|_| ()).map_err(|e| e.into())
    }

    ///
    /// Disconnects from peers and proceeds to shut down the node.
    ///
    #[inline]
    pub async fn shut_down(&self) {
        info!("Shutting down...");
        // Update the node status.
        self.status.update(State::ShuttingDown);

        // Shut down the ledger.
        trace!("Proceeding to shut down the ledger...");
        let (canon_lock, block_requests_lock, storage_map_lock) = self.ledger.shut_down().await;

        // Acquire the locks for ledger.
        trace!("Proceeding to lock the ledger...");
        let _block_requests_lock = block_requests_lock.lock().await;
        let _canon_lock = canon_lock.lock().await;
        let _storage_map_lock = storage_map_lock.write();
        trace!("Ledger has shut down, proceeding to flush tasks...");

        // Flush the tasks.
        self.tasks.flush();
        trace!("Node has shut down.");
    }

    ///
    /// Initialize the connection listener for new peers.
    ///
    #[inline]
    async fn initialize_listener(
        tasks: &mut Tasks<task::JoinHandle<()>>,
        local_ip: SocketAddr,
        listener: TcpListener,
        peers_router: PeersRouter<N, E>,
        peers: Arc<Peers<N, E>>,
        ledger_reader: LedgerReader<N>,
        ledger_router: LedgerRouter<N>,
        prover_router: ProverRouter<N>,
        pool_router: PoolRouter<N>,
    ) {
        // Initialize the listener process.
        let (router, handler) = oneshot::channel();
        tasks.append(task::spawn(async move {
            // Notify the outer function that the task is ready.
            let _ = router.send(());
            info!("Listening for peers at {}", local_ip);
            loop {
                // Don't accept connections if the node is breaching the configured peer limit.
                if peers.number_of_connected_peers().await < E::MAXIMUM_NUMBER_OF_PEERS {
                    // Asynchronously wait for an inbound TcpStream.
                    match listener.accept().await {
                        // Process the inbound connection request.
                        Ok((stream, peer_ip)) => {
                            let request = PeersRequest::PeerConnecting(
                                stream,
                                peer_ip,
                                ledger_reader.clone(),
                                ledger_router.clone(),
                                prover_router.clone(),
                                pool_router.clone(),
                            );
                            if let Err(error) = peers_router.send(request).await {
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
        peers_router: PeersRouter<N, E>,
        ledger_reader: LedgerReader<N>,
        ledger_router: LedgerRouter<N>,
        prover_router: ProverRouter<N>,
        pool_router: PoolRouter<N>,
    ) {
        // Initialize the heartbeat process.
        let (router, handler) = oneshot::channel();
        tasks.append(task::spawn(async move {
            // Notify the outer function that the task is ready.
            let _ = router.send(());
            loop {
                // Transmit a heartbeat request to the ledger.
                if let Err(error) = ledger_router.send(LedgerRequest::Heartbeat(prover_router.clone())).await {
                    error!("Failed to send heartbeat to ledger: {}", error)
                }
                // Transmit a heartbeat request to the peers.
                let request = PeersRequest::Heartbeat(
                    ledger_reader.clone(),
                    ledger_router.clone(),
                    prover_router.clone(),
                    pool_router.clone(),
                );
                if let Err(error) = peers_router.send(request).await {
                    error!("Failed to send heartbeat to peers: {}", error)
                }
                // Sleep for `E::HEARTBEAT_IN_SECS` seconds.
                tokio::time::sleep(Duration::from_secs(E::HEARTBEAT_IN_SECS)).await;
            }
        }));
        // Wait until the heartbeat task is ready.
        let _ = handler.await;
    }

    ///
    /// Initialize a new instance of the RPC server.
    ///
    #[inline]
    async fn initialize_rpc(
        tasks: &mut Tasks<task::JoinHandle<()>>,
        node: &Node,
        status: &Status,
        peers: &Arc<Peers<N, E>>,
        ledger_reader: LedgerReader<N>,
        prover_router: ProverRouter<N>,
        memory_pool: Arc<RwLock<MemoryPool<N>>>,
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
                    prover_router,
                    memory_pool,
                )
                .await,
            );
        }
    }

    ///
    /// Initialize a new instance of the notification.
    ///
    #[inline]
    async fn initialize_notification(
        tasks: &mut Tasks<task::JoinHandle<()>>,
        ledger: LedgerReader<N>,
        prover: Arc<Prover<N, E>>,
        address: Option<Address<N>>,
    ) {
        // Initialize the heartbeat process.
        let (router, handler) = oneshot::channel();
        tasks.append(task::spawn(async move {
            // Notify the outer function that the task is ready.
            let _ = router.send(());
            loop {
                info!("{}", notification_message(address));

                if E::NODE_TYPE == NodeType::Miner {
                    if let Some(miner_address) = address {
                        // Retrieve the latest block height.
                        let latest_block_height = ledger.latest_block_height();

                        // Prepare a list of confirmed and pending coinbase records.
                        let mut confirmed = vec![];
                        let mut pending = vec![];

                        // Iterate through the coinbase records from storage.
                        for (block_height, record) in prover.to_coinbase_records() {
                            // Filter the coinbase records by determining if they exist on the canonical chain.
                            if let Ok(true) = ledger.contains_commitment(&record.commitment()) {
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
        }));
        // Wait until the heartbeat task is ready.
        let _ = handler.await;
    }
}
