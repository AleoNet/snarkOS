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
    peers::{OutboundRouter, PeersRequest, PeersRouter},
    Environment,
    Message,
    NodeType,
    Peers,
};
use snarkos_ledger::storage::rocksdb::RocksDB;
use snarkvm::prelude::*;

use ::rand::{thread_rng, Rng};
use anyhow::Result;
use futures::SinkExt;
use std::time::Duration;

use std::{net::SocketAddr, sync::Arc};

use tokio::{
    net::TcpListener,
    sync::{mpsc, Mutex},
    task,
    time::timeout,
};
use tokio_stream::StreamExt;
use tokio_util::codec::Framed;

///
/// A set of operations to initialize the node server.
///
pub(crate) struct Initialize<N: Network, E: Environment> {
    /// The ledger state of the node.
    ledger: Arc<Mutex<Ledger<N>>>,
    /// The list of peers for the node.
    peers: Arc<Mutex<Peers<N, E>>>,
    /// The list of tasks spawned by the node.
    tasks: Tasks<task::JoinHandle<()>>,
}

impl<N: Network, E: Environment> Initialize<N, E> {
    ///
    /// Starts the connection listener for peers.
    ///
    pub(crate) async fn initialize(port: u16, miner: Option<Address<N>>) -> Result<Self> {
        // Initialize a new TCP listener at the given IP.
        let (local_ip, listener) = match TcpListener::bind(&format!("127.0.0.1:{}", port)).await {
            Ok(listener) => (listener.local_addr().expect("Failed to fetch the local IP"), listener),
            Err(error) => panic!("Failed to bind listener: {:?}. Check if another Aleo node is running", error),
        };

        // Initialize the tasks handler.
        let mut tasks = Tasks::new();

        // Initialize a new instance for managing peers.
        let (peers, peers_router) = Self::initialize_peers(&mut tasks, local_ip);

        // Initialize the connection listener for new peers.
        Self::initialize_listener(&mut tasks, local_ip, listener, peers_router.clone());

        peers_router
            .send(PeersRequest::ConnectNewPeer(
                "127.0.0.1:4133".parse().unwrap(),
                peers_router.clone(),
            ))
            .await?;

        // Initialize a new instance for managing the ledger.
        let (ledger, ledger_router) = Self::initialize_ledger(&mut tasks, local_ip, peers_router.clone(), miner)?;

        // // Initialize a new instance of the miner.
        // let (miner, miner_router) = match E::NODE_TYPE == NodeType::Miner {
        //     true => Self::initialize_miner(&mut tasks, ledger_router),
        //     false => return Err(anyhow!("Node is not a mining node")),
        // };

        Ok(Self { ledger, peers, tasks })
        // Ok(Self { peers, tasks })
    }

    ///
    /// Initialize a new instance for managing peers.
    ///
    fn initialize_peers(tasks: &mut Tasks<task::JoinHandle<()>>, local_ip: SocketAddr) -> (Arc<Mutex<Peers<N, E>>>, PeersRouter<N, E>) {
        let peers = Arc::new(Mutex::new(Peers::new(local_ip)));

        // Initialize an mpsc channel for sending requests to the `Peers` struct.
        let (peers_router, mut peers_handler) = mpsc::channel(1024);

        // Initialize the peers router process.
        let peers_clone = peers.clone();
        tasks.append(task::spawn(async move {
            // Asynchronously wait for a peers request.
            // while let Some(request) = peers_handler.recv().await {
            //     // Hold the peers mutex briefly, to update the state of the peers.
            //     peers_clone.lock().await.update(request).await;
            // }
            loop {
                tokio::select! {
                    // Channel is routing a request to peers.
                    Some(request) = peers_handler.recv() => {
                        // Hold the peers mutex briefly, to update the state of the peers.
                        peers_clone.lock().await.update(request).await;
                    }
                }
            }
        }));

        // // Initialize a process to maintain an adequate number of peers.
        // let peers_router_clone = peers_router.clone();
        // task::spawn(async move {
        //     loop {
        //         // Sleep for 30 seconds.
        //         tokio::time::sleep(Duration::from_secs(30)).await;
        //         // Transmit the heartbeat request.
        //         let request = PeersRequest::Heartbeat(peers_router_clone.clone());
        //         if let Err(error) = peers_router_clone.send(request).await {
        //             error!("Failed to send request to peers: {}", error)
        //         }
        //     }
        // });

        (peers, peers_router)
    }

    ///
    /// Initialize the connection listener for new peers.
    ///
    fn initialize_listener(
        tasks: &mut Tasks<task::JoinHandle<()>>,
        local_ip: SocketAddr,
        listener: TcpListener,
        peers_router: PeersRouter<N, E>,
    ) {
        tasks.append(task::spawn(async move {
            info!("Listening for peers at {}", local_ip);
            loop {
                // Asynchronously wait for an inbound TcpStream.
                match listener.accept().await {
                    // Process the inbound connection request.
                    Ok((stream, peer_ip)) => {
                        let request = PeersRequest::HandleNewPeer(stream, peer_ip, peers_router.clone());
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
    /// Initialize a new instance for managing the ledger.
    ///
    fn initialize_ledger(
        tasks: &mut Tasks<task::JoinHandle<()>>,
        local_ip: SocketAddr,
        peers_router: PeersRouter<N, E>,
        miner: Option<Address<N>>,
    ) -> Result<(Arc<Mutex<Ledger<N>>>, LedgerRouter<N, E>)> {
        // Open the ledger from storage.
        let ledger = Ledger::<N>::open::<RocksDB, _>(&format!(".ledger-{}", thread_rng().gen::<u8>()))?;
        let ledger = Arc::new(Mutex::new(ledger));

        // Initialize an mpsc channel for sending requests to the `Ledger` struct.
        let (ledger_router, mut ledger_handler) = mpsc::channel(1024);

        // Initialize the ledger router process.
        let ledger_clone = ledger.clone();
        let peers_router_clone = peers_router.clone();
        tasks.append(task::spawn(async move {
            // Asynchronously wait for a ledger request.
            while let Some(request) = ledger_handler.recv().await {
                // trace!("{:?}", request);
                // Hold the ledger mutex briefly, to update the state of the ledger.
                if let Err(error) = ledger_clone.lock().await.update::<E>(request).await {
                    error!("{}", error);
                }
            }
        }));

        if let Some(recipient) = miner {
            if E::NODE_TYPE == NodeType::Miner {
                let ledger_router = ledger_router.clone();
                let peers_router_clone = peers_router.clone();
                tasks.append(task::spawn(async move {
                    loop {
                        // Start the mining process.
                        if let Err(error) = ledger_router.send(LedgerRequest::Mine(recipient, peers_router_clone.clone())).await {
                            error!("Miner error: {}", error);
                        }
                        // Sleep for 1 seconds.
                        tokio::time::sleep(Duration::from_secs(1)).await;

                        // // Ensure the miner did not error.
                        // if let Err(error) = result {
                        //     // Sleep for 10 seconds.
                        //     tokio::time::sleep(Duration::from_secs(10)).await;
                        //     warn!("{}", error);
                        // }
                    }
                }));
            }
        }

        Ok((ledger, ledger_router))
    }

    // ///
    // /// Initialize a new instance for the miner.
    // ///
    // fn initialize_miner(tasks: &mut Tasks<task::JoinHandle<()>>, ledger_router: LedgerRouter<N, E>) -> Result<(
    //     Arc<Mutex<Miner<N>>>,
    //     MinerHandler<N, E>,
    // )> {
    //     // If the node is a mining node, initialize a miner.
    //     let miner = match E::NODE_TYPE == NodeType::Miner {
    //         true => Miner::spawn(self.clone(), miner_address),
    //         false => return Err(anyhow!("Node is not a mining node")),
    //     };
    //
    //     let miner = Arc::new(Mutex::new(miner));
    //
    //     // Initialize an mpsc channel for sending requests to the `Miner` struct.
    //     let (miner_handler, mut miner_router) = mpsc::channel(128);
    //
    //     // Initialize the miner router process.
    //     let miner_clone = miner.clone();
    //     tasks.append(tokio::spawn(async move {
    //         // Asynchronously wait for a miner request.
    //         while let Some(event) = miner_router.next().await {
    //             // Hold the miner threaded mutex briefly, to update the state of the miner.
    //             miner_clone.lock().expect("Mutex was poisoned").update(event);
    //         }
    //     }));
    //
    //     Ok((miner, miner_handler))
    // }

    // ///
    // /// Initiates a connection request to the given IP address.
    // ///
    // pub(crate) async fn connect_to(ledger: Arc<RwLock<Ledger<N>>>, peers: Arc<RwLock<Self>>, peer_ip: SocketAddr) -> Result<()> {
    //     // The local IP address must be known by now.
    //     let local_ip = peers.read().await.local_ip()?;
    //
    //     // Ensure the remote IP is not this node.
    //     if peer_ip == local_ip || (peer_ip.ip().is_unspecified() || peer_ip.ip().is_loopback()) && peer_ip.port() == local_ip.port() {
    //         debug!("Skipping connection request to {} (attempted to self-connect)", peer_ip);
    //         Ok(())
    //     }
    //     // Ensure the node does not surpass the maximum number of peer connections.
    //     else if peers.read().await.num_connected_peers() >= E::MAXIMUM_NUMBER_OF_PEERS {
    //         debug!("Skipping connection request to {} (maximum peers reached)", peer_ip);
    //         Ok(())
    //     }
    //     // Ensure the peer is a new connection.
    //     else if peers.read().await.is_connected_to(peer_ip) {
    //         debug!("Skipping connection request to {} (already connected)", peer_ip);
    //         Ok(())
    //     }
    //     // Attempt to open a TCP stream.
    //     else {
    //         debug!("Connecting to {}...", peer_ip);
    //         let stream = match timeout(Duration::from_secs(E::CONNECTION_TIMEOUT_SECS), TcpStream::connect(peer_ip)).await {
    //             Ok(stream) => match stream {
    //                 Ok(stream) => stream,
    //                 Err(error) => return Err(anyhow!("Failed to connect to '{}': '{:?}'", peer_ip, error)),
    //             },
    //             Err(error) => return Err(anyhow!("Unable to reach '{}': '{:?}'", peer_ip, error)),
    //         };
    //
    //         Self::spawn_handler(ledger.clone(), peers, peer_ip, stream).await;
    //         Ok(())
    //     }
    // }
}
