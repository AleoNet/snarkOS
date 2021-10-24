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

use crate::{helpers::Tasks, network::peers::Peer, peers::Outbound, Environment, Message, NodeType, Peers};
use snarkos_ledger::{ledger::Ledger, storage::rocksdb::RocksDB};
use snarkvm::prelude::*;

use anyhow::{anyhow, Result};
use futures::SinkExt;
use once_cell::sync::OnceCell;
// use rand::{thread_rng, Rng};
use std::{
    collections::{HashMap, HashSet},
    time::{Duration, Instant},
};

use std::{net::SocketAddr, sync::Arc};

use tokio::{
    net::{TcpListener, TcpStream},
    sync::{mpsc, Mutex},
    task,
    task::JoinHandle,
    time::timeout,
};
use tokio_stream::StreamExt;
use tokio_util::codec::Framed;

#[derive(Debug)]
pub enum PeersOperation<N: Network, E: Environment> {
    AddCandidatePeer(SocketAddr),
    AddConnectedPeer(SocketAddr, Outbound<N, E>),
    RemoveCandidatePeer(SocketAddr),
    RemoveConnectedPeer(SocketAddr),
    Propagate(SocketAddr, Message<N, E>),
    Broadcast(Message<N, E>),
    SendPeerResponse(SocketAddr),
    HandleNewPeer(TcpStream, SocketAddr, PeersHandler<N, E>),
    ConnectNewPeer(SocketAddr, PeersHandler<N, E>),
}

pub enum LedgerEvent<N: Network, E: Environment> {
    UnconfirmedBlock(Block<N>),
    Unused(std::marker::PhantomData<E>),
}

/// Shorthand for the parent half of the message channel.
pub(crate) type PeersHandler<N, E> = mpsc::Sender<PeersOperation<N, E>>;
/// Shorthand for the child half of the message channel.
pub(crate) type PeersRouter<N, E> = mpsc::Receiver<PeersOperation<N, E>>;

/// Shorthand for the parent half of the message channel.
pub(crate) type LedgerHandler<N, E> = mpsc::Sender<LedgerEvent<N, E>>;
/// Shorthand for the child half of the message channel.
pub(crate) type LedgerRouter<N, E> = mpsc::Receiver<LedgerEvent<N, E>>;

///
/// A set of operations to initialize the node server.
///
pub(crate) struct Initialize<N: Network, E: Environment> {
    /// The list of peers for the node.
    peers: Arc<Mutex<Peers<N, E>>>,
    // /// The ledger state of the node.
    // ledger: Arc<Mutex<Ledger<N>>>,
    /// The list of tasks spawned by the node.
    tasks: Tasks<task::JoinHandle<()>>,
}

impl<N: Network, E: Environment> Initialize<N, E> {
    ///
    /// Starts the connection listener for peers.
    ///
    pub(crate) async fn initialize(port: u16) -> Result<Self> {
        // Initialize a new TCP listener at the given IP.
        let (listener, local_ip) = match TcpListener::bind(&format!("127.0.0.1:{}", port)).await {
            Ok(listener) => {
                let local_ip = listener.local_addr().expect("Failed to fetch the local IP");
                (listener, local_ip)
            }
            Err(error) => panic!("Failed to bind listener: {:?}. Check if another Aleo node is running", error),
        };

        // Initialize the tasks handler.
        let mut tasks = Tasks::new();

        // Initialize a new instance for managing peers.
        let (peers, peers_handler, peers_task) = Self::initialize_peers(local_ip);
        tasks.append(peers_task);

        // Initialize the connection listener for new peers.
        let listener_task = Self::initialize_listener(listener, local_ip, peers_handler.clone());
        tasks.append(listener_task);

        peers_handler
            .send(PeersOperation::ConnectNewPeer(
                "127.0.0.1:4133".parse().unwrap(),
                peers_handler.clone(),
            ))
            .await?;

        // // Initialize a new instance for managing the ledger.
        // let (ledger, ledger_handler, ledger_task) = Self::initialize_ledger();
        // tasks.append(ledger_task);

        // // Initialize a new instance of the miner.
        // let (miner, miner_handler, miner_task) = Self::initialize_miner(ledger_handler);
        // tasks.append(miner_task);

        // // Initialize an mpsc channel for peer changes, with a generous buffer.
        // let (peerset_tx, peerset_rx) = mpsc::channel::<PeerChange>(128);
        //
        // // Initialize an mpsc channel for peers demand signaling.
        // let (mut demand_tx, demand_rx) = mpsc::channel::<MorePeers>(128);
        //

        // // Initialize a process to maintain an adequate number of peers.
        // let ledger_clone = ledger.clone();
        // let peers_clone = peers.clone();
        // task::spawn(async move {
        //     loop {
        //         // Sleep for 30 seconds.
        //         tokio::time::sleep(Duration::from_secs(30)).await;
        //
        //         // Skip if the number of connected peers is above the minimum threshold.
        //         match peers_clone.read().await.num_connected_peers() < E::MINIMUM_NUMBER_OF_PEERS {
        //             true => trace!("Attempting to find new peer connections"),
        //             false => continue,
        //         };
        //
        //         // Attempt to connect to more peers if the number of connected peers is below the minimum threshold.
        //         for peer_ip in peers_clone.read().await.candidate_peers().iter().take(E::MINIMUM_NUMBER_OF_PEERS) {
        //             trace!("Attempting connection to {}...", peer_ip);
        //             if let Err(error) = Peers::connect_to(ledger_clone.clone(), peers_clone.clone(), *peer_ip).await {
        //                 peers_clone.write().await.candidate_peers.remove(peer_ip);
        //                 trace!("Failed to connect to {}: {}", peer_ip, error);
        //             }
        //         }
        //
        //         // Request more peers if the number of connected peers is below the threshold.
        //         peers_clone.write().await.broadcast(&Message::PeerRequest).await;
        //     }
        // });

        // Ok(Self { peers, ledger, tasks })
        Ok(Self { peers, tasks })
    }

    ///
    /// Initialize a new instance for managing peers.
    ///
    fn initialize_peers(local_ip: SocketAddr) -> (Arc<Mutex<Peers<N, E>>>, PeersHandler<N, E>, task::JoinHandle<()>) {
        let peers = Arc::new(Mutex::new(Peers::new(local_ip)));

        // Initialize an mpsc channel for sending requests to the `Peers` struct.
        let (peers_handler, mut peers_router) = mpsc::channel(1024);

        // Initialize the peers router process.
        let peers_clone = peers.clone();
        let peers_task = task::spawn(async move {
            // Asynchronously wait for a peers operation.

            // while let Some(operation) = peers_router.recv().await {
            //     // Hold the peers threaded mutex briefly, to update the state of a single peer.
            //     peers_clone.lock().await.update(operation).await;
            // }

            loop {
                tokio::select! {
                    // Channel is routing an operation to peers.
                    Some(operation) = peers_router.recv() => {
                        // Hold the peers threaded mutex briefly, to update the state of a single peer.
                        peers_clone.lock().await.update(operation).await;
                    }
                    // result = peers_router.next().await => match result {
                    //     Some(operation) {
                    //         // Hold the peers threaded mutex briefly, to update the state of a single peer.
                    //         peers_clone.lock().expect("Mutex was poisoned").update(operation).await;
                    //     }
                    //     None => error("Failed to parse operation for peers")
                    // }
                }
            }
        });

        (peers, peers_handler, peers_task)
    }

    ///
    /// Initialize the connection listener for new peers.
    ///
    fn initialize_listener(listener: TcpListener, local_ip: SocketAddr, peers_handler: PeersHandler<N, E>) -> task::JoinHandle<()> {
        task::spawn(async move {
            info!("Listening for peers at {}", local_ip);
            loop {
                // Asynchronously wait for an inbound TcpStream.
                match listener.accept().await {
                    // Process the inbound connection request.
                    Ok((stream, peer_ip)) => {
                        if let Err(error) = peers_handler
                            .send(PeersOperation::HandleNewPeer(stream, peer_ip, peers_handler.clone()))
                            .await
                        {
                            error!("Failed to send operation to peers: {}", error)
                        }
                    }
                    Err(error) => error!("Failed to accept a connection: {}", error),
                }
                // Add a small delay to prevent overloading the network from handshakes.
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        })
    }

    // ///
    // /// Initialize a new instance for managing the ledger.
    // ///
    // fn initialize_ledger() -> (Arc<Mutex<Ledger<N>>>, LedgerHandler<N, E>, task::JoinHandle<()>) {
    //     // Open the ledger from storage.
    //     let ledger = Ledger::<N>::open::<RocksDB, _>(&format!(".ledger-{}", thread_rng().gen::<u8>()))?;
    //
    //     let ledger = Arc::new(Mutex::new(ledger));
    //
    //     // Initialize an mpsc channel for sending requests to the `Ledger` struct.
    //     let (ledger_handler, mut ledger_router) = mpsc::channel(1024);
    //
    //     // Initialize the ledger router process.
    //     let ledger_clone = ledger.clone();
    //     let ledger_task = tokio::spawn(async move {
    //         // Asynchronously wait for a ledger operation.
    //         while let Some(event) = ledger_router.next().await {
    //             // Hold the ledger threaded mutex briefly, to update the state of the ledger.
    //             ledger_clone.lock().expect("Mutex was poisoned").update(event);
    //         }
    //     });
    //
    //     (ledger, ledger_handler, ledger_task)
    // }

    // ///
    // /// Initialize a new instance for the miner.
    // ///
    // fn initialize_miner(ledger_handler: LedgerHandler<N, E>)-> (
    //     Arc<Mutex<Miner<N>>>,
    //     MinerHandler<N, E>,
    //     task::JoinHandle<()>,
    // ) {
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
    //     let ledger_task = tokio::spawn(async move {
    //         // Asynchronously wait for a miner operation.
    //         while let Some(event) = miner_router.next().await {
    //             // Hold the miner threaded mutex briefly, to update the state of the miner.
    //             miner_clone.lock().expect("Mutex was poisoned").update(event);
    //         }
    //     });
    //
    //     (miner, miner_handler, ledger_task)
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
