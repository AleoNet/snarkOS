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

use crate::{environment::Environment, Data, LedgerRouter, Message, OutboundRouter, Peer};

use snarkvm::prelude::Network;

use anyhow::Result;
use indexmap::IndexMap;
use std::{
    marker::PhantomData,
    net::SocketAddr,
    sync::Arc,
    time::{Duration, Instant, SystemTime},
};
use tokio::{
    net::TcpStream,
    sync::{mpsc, oneshot, RwLock},
    task,
    time::timeout,
};

/// Shorthand for the parent half of the `Peers` message channel.
pub(crate) type PeersRouter<N> = mpsc::Sender<PeersRequest<N>>;
#[allow(unused)]
/// Shorthand for the child half of the `Peers` message channel.
type PeersHandler<N> = mpsc::Receiver<PeersRequest<N>>;

/// Shorthand for the parent half of the connection result channel.
pub(crate) type ConnectionResult = oneshot::Sender<Result<()>>;

///
/// An enum of requests that the `Peers` struct processes.
///
#[derive(Debug)]
pub enum PeersRequest<N: Network> {
    /// Connect := (peer_ip, ledger_reader, ledger_router, operator_router, prover_router, connection_result)
    Connect(SocketAddr, LedgerRouter<N>, ConnectionResult),
    /// Heartbeat
    Heartbeat,
    /// MessagePropagate := (peer_ip, message)
    MessagePropagate(SocketAddr, Message<N>),
    /// MessageSend := (peer_ip, message)
    MessageSend(SocketAddr, Message<N>),
    /// PeerConnecting := (stream, peer_ip, ledger_router)
    PeerConnecting(TcpStream, SocketAddr, LedgerRouter<N>),
    /// PeerConnected := (peer_ip, outbound_router)
    PeerConnected(SocketAddr, OutboundRouter<N>),
    /// PeerDisconnected := (peer_ip)
    PeerDisconnected(SocketAddr),
    /// PeerRestricted := (peer_ip)
    PeerRestricted(SocketAddr),
}

///
/// A list of peers connected to the node server.
///
pub struct Peers<N: Network, E: Environment> {
    /// The peers router of the node.
    peers_router: PeersRouter<N>,
    /// The local address of this node.
    local_ip: SocketAddr,
    /// The map connected peer IPs to their nonce and outbound message router.
    connected_peers: RwLock<IndexMap<SocketAddr, OutboundRouter<N>>>,
    /// The set of restricted peer IPs.
    restricted_peers: RwLock<IndexMap<SocketAddr, Instant>>,
    /// The map of peers to their first-seen port number, number of attempts, and timestamp of the last inbound connection request.
    seen_inbound_connections: RwLock<IndexMap<SocketAddr, ((u16, u32), SystemTime)>>,
    /// The map of peers to the timestamp of their last outbound connection request.
    seen_outbound_connections: RwLock<IndexMap<SocketAddr, SystemTime>>,
    _phantom: PhantomData<E>,
}

impl<N: Network, E: Environment> Peers<N, E> {
    ///
    /// Initializes a new instance of `Peers`.
    ///
    pub async fn new(local_ip: SocketAddr) -> Arc<Self> {
        // Initialize an mpsc channel for sending requests to the `Peers` struct.
        let (peers_router, mut peers_handler) = mpsc::channel(1024);

        // Initialize the peers.
        let peers = Arc::new(Self {
            peers_router,
            local_ip,
            connected_peers: Default::default(),
            restricted_peers: Default::default(),
            seen_inbound_connections: Default::default(),
            seen_outbound_connections: Default::default(),
            _phantom: PhantomData,
        });

        // Initialize the peers router process.
        {
            let peers = peers.clone();
            let (router, handler) = oneshot::channel();
            E::resources().register_task(
                None,
                task::spawn(async move {
                    // Notify the outer function that the task is ready.
                    let _ = router.send(());
                    // Asynchronously wait for a peers request.
                    while let Some(request) = peers_handler.recv().await {
                        let peers = peers.clone();
                        // Asynchronously process a peers request.
                        E::resources().register_task(
                            None,
                            task::spawn(async move {
                                // Hold the peers write lock briefly, to update the state of the peers.
                                peers.update(request).await;
                            }),
                        );
                    }
                }),
            );
            // Wait until the peers router task is ready.
            let _ = handler.await;
        }

        peers
    }

    /// Returns an instance of the peers router.
    pub fn router(&self) -> PeersRouter<N> {
        self.peers_router.clone()
    }

    /// Returns the IP address of this node.
    pub const fn local_ip(&self) -> &SocketAddr {
        &self.local_ip
    }

    ///
    /// Returns `true` if the node is connected to the given IP.
    ///
    pub async fn is_connected_to(&self, ip: &SocketAddr) -> bool {
        self.connected_peers.read().await.contains_key(ip)
    }

    ///
    /// Returns `true` if the given IP is restricted.
    ///
    pub async fn is_restricted(&self, ip: &SocketAddr) -> bool {
        match self.restricted_peers.read().await.get(ip) {
            Some(timestamp) => timestamp.elapsed().as_secs() < E::RADIO_SILENCE_IN_SECS,
            None => false,
        }
    }

    ///
    /// Returns the list of connected peers.
    ///
    pub async fn connected_peers(&self) -> Vec<SocketAddr> {
        self.connected_peers.read().await.keys().copied().collect()
    }

    ///
    /// Returns the number of connected peers.
    ///
    pub async fn number_of_connected_peers(&self) -> usize {
        self.connected_peers.read().await.len()
    }

    ///
    /// Performs the given `request` to the peers.
    /// All requests must go through this `update`, so that a unified view is preserved.
    ///
    pub(super) async fn update(&self, request: PeersRequest<N>) {
        match request {
            PeersRequest::Connect(peer_ip, ledger_router, _connection_result) => {
                // Ensure the peer IP is not this node.
                if peer_ip == self.local_ip
                    || (peer_ip.ip().is_unspecified() || peer_ip.ip().is_loopback()) && peer_ip.port() == self.local_ip.port()
                {
                    debug!("Skipping connection request to {} (attempted to self-connect)", peer_ip);
                }
                // Ensure the node does not surpass the maximum number of peer connections.
                else if self.number_of_connected_peers().await >= E::MAXIMUM_NUMBER_OF_PEERS {
                    debug!("Skipping connection request to {} (maximum peers reached)", peer_ip);
                }
                // Ensure the peer is a new connection.
                else if self.is_connected_to(&peer_ip).await {
                    debug!("Skipping connection request to {} (already connected)", peer_ip);
                }
                // Ensure the peer is not restricted.
                else if self.is_restricted(&peer_ip).await {
                    debug!("Skipping connection request to {} (restricted)", peer_ip);
                }
                // Attempt to open a TCP stream.
                else {
                    // Lock seen_outbound_connections for further processing.
                    let mut seen_outbound_connections = self.seen_outbound_connections.write().await;

                    // Ensure the node respects the connection frequency limit.
                    let last_seen = seen_outbound_connections.entry(peer_ip).or_insert(SystemTime::UNIX_EPOCH);
                    let elapsed = last_seen.elapsed().unwrap_or(Duration::MAX).as_secs();
                    if elapsed < E::RADIO_SILENCE_IN_SECS {
                        trace!("Skipping connection request to {} (tried {} secs ago)", peer_ip, elapsed);
                    } else {
                        debug!("Connecting to {}...", peer_ip);
                        // Update the last seen timestamp for this peer.
                        seen_outbound_connections.insert(peer_ip, SystemTime::now());

                        // Release the lock over seen_outbound_connections.
                        drop(seen_outbound_connections);

                        // Initialize the peer handler.
                        match timeout(Duration::from_millis(E::CONNECTION_TIMEOUT_IN_MILLIS), TcpStream::connect(peer_ip)).await {
                            Ok(stream) => match stream {
                                Ok(stream) => Peer::<N, E>::handler(stream, peer_ip, self.router(), ledger_router).await,
                                Err(error) => {
                                    trace!("Failed to connect to '{}': '{:?}'", peer_ip, error);
                                }
                            },
                            Err(error) => {
                                error!("Unable to reach '{}': '{:?}'", peer_ip, error);
                            }
                        };
                    }
                }
            }
            PeersRequest::Heartbeat => {
                // TODO (raychu86): Implement this.
            }
            PeersRequest::MessagePropagate(sender, message) => {
                self.propagate(sender, message).await;
            }
            PeersRequest::MessageSend(sender, message) => {
                self.send(sender, message).await;
            }
            PeersRequest::PeerConnected(peer_ip, outbound) => {
                // Add an entry for this `Peer` in the connected peers.
                self.connected_peers.write().await.insert(peer_ip, outbound);
            }
            PeersRequest::PeerConnecting(stream, peer_ip, ledger_router) => {
                // Ensure the peer IP is not this node.
                if peer_ip == self.local_ip
                    || (peer_ip.ip().is_unspecified() || peer_ip.ip().is_loopback()) && peer_ip.port() == self.local_ip.port()
                {
                    debug!("Skipping connection request to {} (attempted to self-connect)", peer_ip);
                }
                // Ensure the node does not surpass the maximum number of peer connections.
                else if self.number_of_connected_peers().await >= E::MAXIMUM_NUMBER_OF_PEERS {
                    debug!("Dropping connection request from {} (maximum peers reached)", peer_ip);
                }
                // Ensure the node is not already connected to this peer.
                else if self.is_connected_to(&peer_ip).await {
                    debug!("Dropping connection request from {} (already connected)", peer_ip);
                }
                // Ensure the peer is not restricted.
                else if self.is_restricted(&peer_ip).await {
                    debug!("Dropping connection request from {} (restricted)", peer_ip);
                } else {
                    // TODO (raychu86): Implement this.

                    // Sanitize the port from the peer, if it is a remote IP address.
                    let (peer_lookup, peer_port) = match peer_ip.ip().is_loopback() {
                        // Loopback case - Do not sanitize, merely pass through.
                        true => (peer_ip, peer_ip.port()),
                        // Remote case - Sanitize, storing u16::MAX for the peer IP address to dedup the peer next time.
                        false => (SocketAddr::new(peer_ip.ip(), u16::MAX), peer_ip.port()),
                    };

                    // Lock seen_inbound_connections for further processing.
                    let mut seen_inbound_connections = self.seen_inbound_connections.write().await;

                    // Fetch the inbound tracker entry for this peer.
                    let ((initial_port, num_attempts), last_seen) = seen_inbound_connections
                        .entry(peer_lookup)
                        .or_insert(((peer_port, 0), SystemTime::UNIX_EPOCH));
                    let elapsed = last_seen.elapsed().unwrap_or(std::time::Duration::MAX).as_secs();

                    // Reset the inbound tracker entry for this peer, if the predefined elapsed time has passed.
                    if elapsed > E::RADIO_SILENCE_IN_SECS {
                        // Reset the initial port for this peer.
                        *initial_port = peer_port;
                        // Reset the number of attempts for this peer.
                        *num_attempts = 0;
                        // Reset the last seen timestamp for this peer.
                        *last_seen = SystemTime::now();
                    }

                    // Ensure the connecting peer has not surpassed the connection attempt limit.
                    if *initial_port < peer_port && *num_attempts > E::MAXIMUM_CONNECTION_FAILURES {
                        trace!("Dropping connection request from {} (tried {} secs ago)", peer_ip, elapsed);
                        // Add an entry for this `Peer` in the restricted peers.
                        self.restricted_peers.write().await.insert(peer_ip, Instant::now());
                    } else {
                        debug!("Received a connection request from {}", peer_ip);
                        // Update the number of attempts for this peer.
                        *num_attempts += 1;

                        // Release the lock over seen_inbound_connections.
                        drop(seen_inbound_connections);

                        Peer::<N, E>::handler(stream, peer_ip, self.peers_router.clone(), ledger_router).await;
                    }
                }
            }
            PeersRequest::PeerDisconnected(peer_ip) => {
                // Remove an entry for this `Peer` in the connected peers, if it exists.
                self.connected_peers.write().await.remove(&peer_ip);
            }
            PeersRequest::PeerRestricted(peer_ip) => {
                // Remove an entry for this `Peer` in the connected peers, if it exists.
                self.connected_peers.write().await.remove(&peer_ip);
                // Add an entry for this `Peer` in the restricted peers.
                self.restricted_peers.write().await.insert(peer_ip, Instant::now());
            }
        }
    }

    ///
    /// Sends the given message to specified peer.
    ///
    async fn send(&self, peer: SocketAddr, message: Message<N>) {
        let target_peer = self.connected_peers.read().await.get(&peer).cloned();
        match target_peer {
            Some(outbound) => {
                if let Err(error) = outbound.send(message).await {
                    trace!("Outbound channel failed: {}", error);
                    self.connected_peers.write().await.remove(&peer);
                }
            }
            None => warn!("Attempted to send to a non-connected peer {}", peer),
        }
    }

    ///
    /// Sends the given message to every connected peer, excluding the sender.
    ///
    async fn propagate(&self, sender: SocketAddr, mut message: Message<N>) {
        // Perform ahead-of-time, non-blocking serialization just once for applicable objects.
        if let Message::BlockBroadcast(ref mut data) = message {
            let serialized_block = Data::serialize(data.clone()).await.expect("Block serialization is bugged");
            let _ = std::mem::replace(data, Data::Buffer(serialized_block));
        }

        // Iterate through all peers that are not the sender, sync node, or beacon node.
        for peer in self
            .connected_peers()
            .await
            .iter()
            .filter(|peer_ip| *peer_ip != &sender && !E::beacon_nodes().contains(peer_ip))
            .copied()
            .collect::<Vec<_>>()
        {
            self.send(peer, message.clone()).await;
        }
    }

    ///
    /// Removes the addresses of all known peers.
    ///
    #[cfg(feature = "test")]
    pub async fn reset_known_peers(&self) {
        self.restricted_peers.write().await.clear();
        self.seen_inbound_connections.write().await.clear();
        self.seen_outbound_connections.write().await.clear();
    }
}
