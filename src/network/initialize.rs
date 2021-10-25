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
    state::{State, StateRequest, StateRouter},
    Environment,
    NodeType,
};
use snarkos_ledger::storage::rocksdb::RocksDB;
use snarkvm::prelude::*;

use ::rand::{thread_rng, Rng};
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
    /// The ledger state of the node.
    ledger: Arc<RwLock<Ledger<N>>>,
    /// The list of tasks spawned by the node.
    tasks: Tasks<task::JoinHandle<()>>,
}

impl<N: Network, E: Environment> Server<N, E> {
    ///
    /// Starts the connection listener for peers.
    ///
    pub(crate) async fn initialize(port: u16, miner: Option<Address<N>>) -> Result<Self> {
        // Initialize a new TCP listener at the given IP.
        let (local_ip, listener) = match TcpListener::bind(&format!("0.0.0.0:{}", port)).await {
            Ok(listener) => (listener.local_addr().expect("Failed to fetch the local IP"), listener),
            Err(error) => panic!("Failed to bind listener: {:?}. Check if another Aleo node is running", error),
        };

        // Initialize the tasks handler.
        let mut tasks = Tasks::new();

        // Initialize a new instance for managing peers.
        let (peers, peers_router) = Self::initialize_peers(&mut tasks, local_ip);

        // Initialize a new instance for managing the ledger.
        let (ledger, ledger_router) = Self::initialize_ledger(&mut tasks)?;

        // Initialize a new instance for managing state.
        let (state, state_router) =
            Self::initialize_state(&mut tasks, local_ip, peers_router.clone(), ledger.clone(), ledger_router.clone());

        // Initialize the connection listener for new peers.
        Self::initialize_listener(&mut tasks, local_ip, listener, peers_router.clone(), state_router.clone());

        // Initialize a new instance of the heartbeat.
        Self::initialize_heartbeat(&mut tasks, peers_router.clone(), state_router.clone());

        let message = PeersRequest::Connect("0.0.0.0:4133".parse().unwrap(), peers_router.clone(), state_router.clone());
        peers_router.send(message).await?;

        // Sleep for 15 seconds.
        tokio::time::sleep(Duration::from_secs(15)).await;

        // Initialize a new instance of the miner.
        Self::initialize_miner(
            &mut tasks,
            local_ip,
            miner,
            peers.clone(),
            peers_router.clone(),
            ledger_router.clone(),
            state.clone(),
        );

        // Initialize a new instance of the RPC server.
        let rpc_ip = "0.0.0.0:3032".parse()?;
        Self::initialize_rpc(&mut tasks, rpc_ip, None, None, ledger.clone(), ledger_router);

        Ok(Self { peers, ledger, tasks })
    }

    ///
    /// Initialize a new instance for managing peers.
    ///
    fn initialize_peers(tasks: &mut Tasks<task::JoinHandle<()>>, local_ip: SocketAddr) -> (Arc<RwLock<Peers<N, E>>>, PeersRouter<N, E>) {
        // Initialize the `Peers` struct.
        let peers = Arc::new(RwLock::new(Peers::new(local_ip)));

        // Initialize an mpsc channel for sending requests to the `Peers` struct.
        let (peers_router, mut peers_handler) = mpsc::channel(1024);

        // Initialize the peers router process.
        let peers_clone = peers.clone();
        tasks.append(task::spawn(async move {
            // Asynchronously wait for a peers request.
            loop {
                tokio::select! {
                    // Channel is routing a request to peers.
                    Some(request) = peers_handler.recv() => {
                        // Hold the peers write lock briefly, to update the state of the peers.
                        peers_clone.write().await.update(request).await;
                    }
                }
            }
        }));

        (peers, peers_router)
    }

    ///
    /// Initialize a new instance for managing the ledger.
    ///
    fn initialize_ledger(tasks: &mut Tasks<task::JoinHandle<()>>) -> Result<(Arc<RwLock<Ledger<N>>>, LedgerRouter<N, E>)> {
        // Open the ledger from storage.
        let ledger = Ledger::<N>::open::<RocksDB, _>(&format!(".ledger-{}", thread_rng().gen::<u8>()))?;
        let ledger = Arc::new(RwLock::new(ledger));

        // Initialize an mpsc channel for sending requests to the `Ledger` struct.
        let (ledger_router, mut ledger_handler) = mpsc::channel(1024);

        // Initialize the ledger router process.
        let ledger_clone = ledger.clone();
        tasks.append(task::spawn(async move {
            // Asynchronously wait for a ledger request.
            while let Some(request) = ledger_handler.recv().await {
                // Hold the ledger write lock briefly, to update the state of the ledger.
                if let Err(error) = ledger_clone.write().await.update::<E>(request).await {
                    error!("{}", error);
                }
            }
        }));

        Ok((ledger, ledger_router))
    }

    ///
    /// Initialize a new instance for managing state.
    ///
    fn initialize_state(
        tasks: &mut Tasks<task::JoinHandle<()>>,
        local_ip: SocketAddr,
        peers_router: PeersRouter<N, E>,
        ledger: Arc<RwLock<Ledger<N>>>,
        ledger_router: LedgerRouter<N, E>,
    ) -> (Arc<RwLock<State<N, E>>>, StateRouter<N, E>) {
        // Initialize the `State` struct.
        let state = Arc::new(RwLock::new(State::new(local_ip)));

        // Initialize an mpsc channel for sending requests to the `State` struct.
        let (state_router, mut state_handler) = mpsc::channel(1024);

        // Initialize the state router process.
        let state_clone = state.clone();
        tasks.append(task::spawn(async move {
            // Asynchronously wait for a state request.
            loop {
                tokio::select! {
                    // Channel is routing a request to state.
                    Some(request) = state_handler.recv() => {
                        // Hold the state write lock briefly, to update the state manager.
                        state_clone.write().await.update(request, peers_router.clone(), ledger.clone(), ledger_router.clone()).await;
                    }
                }
            }
        }));

        (state, state_router)
    }

    ///
    /// Initialize the connection listener for new peers.
    ///
    fn initialize_listener(
        tasks: &mut Tasks<task::JoinHandle<()>>,
        local_ip: SocketAddr,
        listener: TcpListener,
        peers_router: PeersRouter<N, E>,
        state_router: StateRouter<N, E>,
    ) {
        tasks.append(task::spawn(async move {
            info!("Listening for peers at {}", local_ip);
            loop {
                // Asynchronously wait for an inbound TcpStream.
                match listener.accept().await {
                    // Process the inbound connection request.
                    Ok((stream, peer_ip)) => {
                        let request = PeersRequest::PeerConnecting(stream, peer_ip, peers_router.clone(), state_router.clone());
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
    fn initialize_heartbeat(tasks: &mut Tasks<task::JoinHandle<()>>, peers_router: PeersRouter<N, E>, state_router: StateRouter<N, E>) {
        // Initialize a process to maintain an adequate number of peers.
        let peers_router_clone = peers_router.clone();
        let state_router_clone = state_router.clone();
        tasks.append(task::spawn(async move {
            loop {
                // Transmit a heartbeat request to the peers.
                let request = PeersRequest::Heartbeat(peers_router_clone.clone(), state_router.clone());
                if let Err(error) = peers_router_clone.send(request).await {
                    error!("Failed to send request to peers: {}", error)
                }
                // Transmit a heartbeat request to the state manager.
                let request = StateRequest::Heartbeat;
                if let Err(error) = state_router_clone.send(request).await {
                    error!("Failed to send request to state manager: {}", error)
                }
                // Sleep for 5 seconds.
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }));
    }

    ///
    /// Initialize a new instance of the miner.
    ///
    fn initialize_miner(
        tasks: &mut Tasks<task::JoinHandle<()>>,
        local_ip: SocketAddr,
        miner: Option<Address<N>>,
        peers: Arc<RwLock<Peers<N, E>>>,
        peers_router: PeersRouter<N, E>,
        ledger_router: LedgerRouter<N, E>,
        state: Arc<RwLock<State<N, E>>>,
    ) {
        if E::NODE_TYPE == NodeType::Miner {
            if let Some(recipient) = miner {
                let ledger_router = ledger_router.clone();
                let peers_router = peers_router.clone();
                tasks.append(task::spawn(async move {
                    loop {
                        // Skip if the state manager is syncing.
                        if state.read().await.is_syncing() {
                            continue;
                        }
                        // Skip if the node server is not connected to the minimum number of peers.
                        if peers.read().await.num_connected_peers() < E::MINIMUM_NUMBER_OF_PEERS {
                            continue;
                        }
                        // Start the mining process.
                        else {
                            let request = LedgerRequest::Mine(local_ip, recipient, peers_router.clone(), ledger_router.clone());
                            if let Err(error) = ledger_router.send(request).await {
                                error!("Failed to send request to ledger: {}", error);
                            }
                        }
                        // Sleep for 1 second.
                        tokio::time::sleep(Duration::from_secs(1)).await;
                    }
                }));
            } else {
                error!("Missing miner address. Please specify an Aleo address in order to mine");
            }
        }
    }

    ///
    /// Initialize a new instance of the RPC server.
    ///
    fn initialize_rpc(
        tasks: &mut Tasks<task::JoinHandle<()>>,
        rpc_ip: SocketAddr,
        username: Option<String>,
        password: Option<String>,
        ledger: Arc<RwLock<Ledger<N>>>,
        ledger_router: LedgerRouter<N, E>,
    ) {
        let ledger_router = ledger_router.clone();
        tasks.append(initialize_rpc_server::<N>(rpc_ip, username, password, ledger.clone()));
    }
}
