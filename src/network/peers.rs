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
    helpers::{State, Status, Tasks},
    Data,
    Environment,
    LedgerReader,
    LedgerRequest,
    LedgerRouter,
    Message,
    NodeType,
    OperatorRequest,
    OperatorRouter,
    ProverRequest,
    ProverRouter,
};
use snarkvm::dpc::prelude::*;

use anyhow::{anyhow, Result};
use futures::SinkExt;
use rand::{prelude::IteratorRandom, rngs::OsRng, thread_rng, Rng};
use std::{
    collections::{HashMap, HashSet},
    net::SocketAddr,
    sync::Arc,
    time::{Duration, Instant, SystemTime},
};
use tokio::{
    net::TcpStream,
    sync::{mpsc, oneshot, RwLock},
    task,
    task::JoinHandle,
    time::timeout,
};
use tokio_stream::StreamExt;
use tokio_util::codec::Framed;

/// Shorthand for the parent half of the `Peer` outbound message channel.
pub(crate) type OutboundRouter<N, E> = mpsc::Sender<Message<N, E>>;
/// Shorthand for the child half of the `Peer` outbound message channel.
type OutboundHandler<N, E> = mpsc::Receiver<Message<N, E>>;

/// Shorthand for the parent half of the `Peers` message channel.
pub(crate) type PeersRouter<N, E> = mpsc::Sender<PeersRequest<N, E>>;
#[allow(unused)]
/// Shorthand for the child half of the `Peers` message channel.
type PeersHandler<N, E> = mpsc::Receiver<PeersRequest<N, E>>;

/// Shorthand for the parent half of the connection result channel.
type ConnectionResult = oneshot::Sender<Result<()>>;

///
/// An enum of requests that the `Peers` struct processes.
///
#[derive(Debug)]
pub enum PeersRequest<N: Network, E: Environment> {
    /// Connect := (peer_ip, ledger_reader, ledger_router, operator_router, prover_router, connection_result)
    Connect(
        SocketAddr,
        LedgerReader<N>,
        LedgerRouter<N>,
        OperatorRouter<N>,
        ProverRouter<N>,
        ConnectionResult,
    ),
    /// Heartbeat := (ledger_reader, ledger_router, operator_router, prover_router)
    Heartbeat(LedgerReader<N>, LedgerRouter<N>, OperatorRouter<N>, ProverRouter<N>),
    /// MessagePropagate := (peer_ip, message)
    MessagePropagate(SocketAddr, Message<N, E>),
    /// MessageSend := (peer_ip, message)
    MessageSend(SocketAddr, Message<N, E>),
    /// PeerConnecting := (stream, peer_ip, ledger_reader, ledger_router, operator_router, prover_router)
    PeerConnecting(
        TcpStream,
        SocketAddr,
        LedgerReader<N>,
        LedgerRouter<N>,
        OperatorRouter<N>,
        ProverRouter<N>,
    ),
    /// PeerConnected := (peer_ip, peer_nonce, outbound_router)
    PeerConnected(SocketAddr, u64, OutboundRouter<N, E>),
    /// PeerDisconnected := (peer_ip)
    PeerDisconnected(SocketAddr),
    /// PeerRestricted := (peer_ip)
    PeerRestricted(SocketAddr),
    /// SendPeerResponse := (peer_ip)
    SendPeerResponse(SocketAddr),
    /// ReceivePeerResponse := (\[peer_ip\])
    ReceivePeerResponse(Vec<SocketAddr>),
}

///
/// A list of peers connected to the node server.
///
pub struct Peers<N: Network, E: Environment> {
    /// The peers router of the node.
    peers_router: PeersRouter<N, E>,
    /// The local address of this node.
    local_ip: SocketAddr,
    /// The local nonce for this node session.
    local_nonce: u64,
    /// The local status of this node.
    local_status: Status,
    /// The map connected peer IPs to their nonce and outbound message router.
    connected_peers: RwLock<HashMap<SocketAddr, (u64, OutboundRouter<N, E>)>>,
    /// The set of candidate peer IPs.
    candidate_peers: RwLock<HashSet<SocketAddr>>,
    /// The set of restricted peer IPs.
    restricted_peers: RwLock<HashMap<SocketAddr, Instant>>,
    /// The map of peers to their first-seen port number, number of attempts, and timestamp of the last inbound connection request.
    seen_inbound_connections: RwLock<HashMap<SocketAddr, ((u16, u32), SystemTime)>>,
    /// The map of peers to the timestamp of their last outbound connection request.
    seen_outbound_connections: RwLock<HashMap<SocketAddr, SystemTime>>,
}

impl<N: Network, E: Environment> Peers<N, E> {
    ///
    /// Initializes a new instance of `Peers`.
    ///
    pub(crate) async fn new(
        tasks: Tasks<JoinHandle<()>>,
        local_ip: SocketAddr,
        local_nonce: Option<u64>,
        local_status: &Status,
    ) -> Arc<Self> {
        // Initialize an mpsc channel for sending requests to the `Peers` struct.
        let (peers_router, mut peers_handler) = mpsc::channel(1024);

        // Sample the nonce.
        let local_nonce = match local_nonce {
            Some(nonce) => nonce,
            None => thread_rng().gen(),
        };

        // Initialize the peers.
        let peers = Arc::new(Self {
            peers_router,
            local_ip,
            local_nonce,
            local_status: local_status.clone(),
            connected_peers: Default::default(),
            candidate_peers: Default::default(),
            restricted_peers: Default::default(),
            seen_inbound_connections: Default::default(),
            seen_outbound_connections: Default::default(),
        });

        // Initialize the peers router process.
        {
            let peers = peers.clone();
            let tasks_clone = tasks.clone();
            let (router, handler) = oneshot::channel();
            tasks.append(task::spawn(async move {
                // Notify the outer function that the task is ready.
                let _ = router.send(());
                // Asynchronously wait for a peers request.
                while let Some(request) = peers_handler.recv().await {
                    let peers = peers.clone();
                    let tasks = tasks_clone.clone();
                    // Asynchronously process a peers request.
                    tasks_clone.append(task::spawn(async move {
                        // Hold the peers write lock briefly, to update the state of the peers.
                        peers.update(request, &tasks).await;
                    }));
                }
            }));
            // Wait until the peers router task is ready.
            let _ = handler.await;
        }

        peers
    }

    /// Returns an instance of the peers router.
    pub fn router(&self) -> PeersRouter<N, E> {
        self.peers_router.clone()
    }

    ///
    /// Returns `true` if the node is connected to the given IP.
    ///
    pub async fn is_connected_to(&self, ip: SocketAddr) -> bool {
        self.connected_peers.read().await.contains_key(&ip)
    }

    ///
    /// Returns `true` if the given IP is restricted.
    ///
    pub async fn is_restricted(&self, ip: SocketAddr) -> bool {
        match self.restricted_peers.read().await.get(&ip) {
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
    /// Returns the list of candidate peers.
    ///
    pub async fn candidate_peers(&self) -> HashSet<SocketAddr> {
        self.candidate_peers.read().await.clone()
    }

    ///
    /// TODO (howardwu): Make this operation more efficient.
    /// Returns the number of connected sync nodes.
    ///
    pub async fn connected_sync_nodes(&self) -> HashSet<SocketAddr> {
        let connected_peers: HashSet<SocketAddr> = self.connected_peers.read().await.keys().into_iter().copied().collect();
        let sync_nodes: HashSet<SocketAddr> = E::SYNC_NODES.iter().map(|ip| ip.parse().unwrap()).collect();
        connected_peers.intersection(&sync_nodes).copied().collect()
    }

    ///
    /// TODO (howardwu): Make this operation more efficient.
    /// Returns the number of connected sync nodes.
    ///
    pub async fn number_of_connected_sync_nodes(&self) -> usize {
        let connected_peers: HashSet<SocketAddr> = self.connected_peers.read().await.keys().into_iter().copied().collect();
        let sync_nodes: HashSet<SocketAddr> = E::SYNC_NODES.iter().map(|ip| ip.parse().unwrap()).collect();
        connected_peers.intersection(&sync_nodes).count()
    }

    ///
    /// Returns the number of connected peers.
    ///
    pub async fn number_of_connected_peers(&self) -> usize {
        self.connected_peers.read().await.len()
    }

    ///
    /// Returns the number of candidate peers.
    ///
    pub async fn number_of_candidate_peers(&self) -> usize {
        self.candidate_peers.read().await.len()
    }

    ///
    /// Returns the list of nonces for the connected peers.
    ///
    pub(crate) async fn connected_nonces(&self) -> Vec<u64> {
        self.connected_peers
            .read()
            .await
            .values()
            .map(|(peer_nonce, _)| *peer_nonce)
            .collect()
    }

    ///
    /// Performs the given `request` to the peers.
    /// All requests must go through this `update`, so that a unified view is preserved.
    ///
    pub(super) async fn update(&self, request: PeersRequest<N, E>, tasks: &Tasks<JoinHandle<()>>) {
        match request {
            PeersRequest::Connect(peer_ip, ledger_reader, ledger_router, operator_router, prover_router, connection_result) => {
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
                else if self.is_connected_to(peer_ip).await {
                    debug!("Skipping connection request to {} (already connected)", peer_ip);
                }
                // Ensure the peer is not restricted.
                else if self.is_restricted(peer_ip).await {
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
                                Ok(stream) => {
                                    Peer::handler(
                                        stream,
                                        self.local_ip,
                                        self.local_nonce,
                                        self.local_status.clone(),
                                        &self.peers_router,
                                        ledger_reader,
                                        ledger_router,
                                        prover_router,
                                        operator_router,
                                        self.connected_nonces().await,
                                        Some(connection_result),
                                        tasks.clone(),
                                    )
                                    .await
                                }
                                Err(error) => {
                                    trace!("Failed to connect to '{}': '{:?}'", peer_ip, error);
                                    self.candidate_peers.write().await.remove(&peer_ip);
                                }
                            },
                            Err(error) => {
                                error!("Unable to reach '{}': '{:?}'", peer_ip, error);
                                self.candidate_peers.write().await.remove(&peer_ip);
                            }
                        };
                    }
                }
            }
            PeersRequest::Heartbeat(ledger_reader, ledger_router, operator_router, prover_router) => {
                // Obtain the number of connected peers.
                let number_of_connected_peers = self.number_of_connected_peers().await;
                // Ensure the number of connected peers is below the maximum threshold.
                if number_of_connected_peers > E::MAXIMUM_NUMBER_OF_PEERS {
                    debug!("Exceeded maximum number of connected peers");

                    // Determine the peers to disconnect from.
                    let num_excess_peers = number_of_connected_peers.saturating_sub(E::MAXIMUM_NUMBER_OF_PEERS);
                    let peer_ips_to_disconnect = self
                        .connected_peers
                        .read()
                        .await
                        .iter()
                        .filter(|(&peer_ip, _)| {
                            let peer_str = peer_ip.to_string();
                            !E::SYNC_NODES.contains(&peer_str.as_str()) && !E::BEACON_NODES.contains(&peer_str.as_str())
                        })
                        .take(num_excess_peers)
                        .map(|(&peer_ip, _)| peer_ip)
                        .collect::<Vec<SocketAddr>>();

                    // Proceed to send disconnect requests to these peers.
                    for peer_ip in peer_ips_to_disconnect {
                        info!("Disconnecting from {} (exceeded maximum connections)", peer_ip);
                        self.send(peer_ip, Message::Disconnect).await;
                        // Add an entry for this `Peer` in the restricted peers.
                        self.restricted_peers.write().await.insert(peer_ip, Instant::now());
                    }
                }

                // TODO (howardwu): This logic can be optimized and unified with the context around it.
                // Determine if the node is connected to more sync nodes than expected.
                let connected_sync_nodes = self.connected_sync_nodes().await;
                let number_of_connected_sync_nodes = connected_sync_nodes.len();
                let num_excess_sync_nodes = number_of_connected_sync_nodes.saturating_sub(1);
                if num_excess_sync_nodes > 0 {
                    // Proceed to send disconnect requests to these peers.
                    for peer_ip in connected_sync_nodes
                        .iter()
                        .copied()
                        .choose_multiple(&mut OsRng::default(), num_excess_sync_nodes)
                    {
                        info!("Disconnecting from {} (exceeded maximum connections)", peer_ip);
                        self.send(peer_ip, Message::Disconnect).await;
                        // Add an entry for this `Peer` in the restricted peers.
                        self.restricted_peers.write().await.insert(peer_ip, Instant::now());
                    }
                }

                // Skip if the number of connected peers is above the minimum threshold.
                match number_of_connected_peers < E::MINIMUM_NUMBER_OF_PEERS {
                    true => {
                        trace!("Sending request for more peer connections");
                        // Request more peers if the number of connected peers is below the threshold.
                        for peer_ip in self.connected_peers().await.iter().choose_multiple(&mut OsRng::default(), 3) {
                            self.send(*peer_ip, Message::PeerRequest).await;
                        }
                    }
                    false => return,
                };

                // Add the sync nodes to the list of candidate peers.
                let sync_nodes: Vec<SocketAddr> = E::SYNC_NODES.iter().map(|ip| ip.parse().unwrap()).collect();
                if number_of_connected_sync_nodes == 0 {
                    self.add_candidate_peers(&sync_nodes).await;
                }

                // Add the beacon nodes to the list of candidate peers.
                let beacon_nodes: Vec<SocketAddr> = E::BEACON_NODES.iter().map(|ip| ip.parse().unwrap()).collect();
                self.add_candidate_peers(&beacon_nodes).await;

                // Attempt to connect to more peers if the number of connected peers is below the minimum threshold.
                // Select the peers randomly from the list of candidate peers.
                let midpoint_number_of_peers = E::MINIMUM_NUMBER_OF_PEERS.saturating_add(E::MAXIMUM_NUMBER_OF_PEERS) / 2;
                for peer_ip in self
                    .candidate_peers()
                    .await
                    .iter()
                    .copied()
                    .choose_multiple(&mut OsRng::default(), midpoint_number_of_peers)
                {
                    // Ensure this node is not connected to more than the permitted number of sync nodes.
                    if sync_nodes.contains(&peer_ip) && number_of_connected_sync_nodes >= 1 {
                        continue;
                    }

                    if !self.is_connected_to(peer_ip).await {
                        trace!("Attempting connection to {}...", peer_ip);

                        // Initialize the connection process.
                        let (router, handler) = oneshot::channel();
                        let request = PeersRequest::Connect(
                            peer_ip,
                            ledger_reader.clone(),
                            ledger_router.clone(),
                            operator_router.clone(),
                            prover_router.clone(),
                            router,
                        );
                        if let Err(error) = self.peers_router.send(request).await {
                            warn!("Failed to transmit the request: '{}'", error);
                        }
                        // Do not wait for the result of each connection.
                        tasks.append(task::spawn(async move {
                            let _ = handler.await;
                        }));
                    }
                }
            }
            PeersRequest::MessagePropagate(sender, message) => {
                self.propagate(sender, message).await;
            }
            PeersRequest::MessageSend(sender, message) => {
                self.send(sender, message).await;
            }
            PeersRequest::PeerConnecting(stream, peer_ip, ledger_reader, ledger_router, operator_router, prover_router) => {
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
                else if self.is_connected_to(peer_ip).await {
                    debug!("Dropping connection request from {} (already connected)", peer_ip);
                }
                // Ensure the peer is not restricted.
                else if self.is_restricted(peer_ip).await {
                    debug!("Dropping connection request from {} (restricted)", peer_ip);
                }
                // Spawn a handler to be run asynchronously.
                else {
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
                    let elapsed = last_seen.elapsed().unwrap_or(Duration::MAX).as_secs();

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

                        // Initialize the peer handler.
                        Peer::handler(
                            stream,
                            self.local_ip,
                            self.local_nonce,
                            self.local_status.clone(),
                            &self.peers_router,
                            ledger_reader,
                            ledger_router,
                            prover_router,
                            operator_router,
                            self.connected_nonces().await,
                            None,
                            tasks.clone(),
                        )
                        .await;
                    }
                }
            }
            PeersRequest::PeerConnected(peer_ip, peer_nonce, outbound) => {
                // Add an entry for this `Peer` in the connected peers.
                self.connected_peers.write().await.insert(peer_ip, (peer_nonce, outbound));
                // Remove an entry for this `Peer` in the candidate peers, if it exists.
                self.candidate_peers.write().await.remove(&peer_ip);
            }
            PeersRequest::PeerDisconnected(peer_ip) => {
                // Remove an entry for this `Peer` in the connected peers, if it exists.
                self.connected_peers.write().await.remove(&peer_ip);
                // Add an entry for this `Peer` in the candidate peers.
                self.candidate_peers.write().await.insert(peer_ip);
            }
            PeersRequest::PeerRestricted(peer_ip) => {
                // Remove an entry for this `Peer` in the connected peers, if it exists.
                self.connected_peers.write().await.remove(&peer_ip);
                // Add an entry for this `Peer` in the restricted peers.
                self.restricted_peers.write().await.insert(peer_ip, Instant::now());
            }
            PeersRequest::SendPeerResponse(recipient) => {
                // Send a `PeerResponse` message.
                let connected_peers = self.connected_peers().await;
                self.send(recipient, Message::PeerResponse(connected_peers)).await;
            }
            PeersRequest::ReceivePeerResponse(peer_ips) => {
                self.add_candidate_peers(&peer_ips).await;
            }
        }
    }

    ///
    /// Adds the given peer IPs to the set of candidate peers.
    ///
    /// This method skips adding any given peers if the combined size exceeds the threshold,
    /// as the peer providing this list could be subverting the protocol.
    ///
    async fn add_candidate_peers(&self, peers: &[SocketAddr]) {
        // Acquire the candidate peers write lock.
        let mut candidate_peers = self.candidate_peers.write().await;
        // Ensure the combined number of peers does not surpass the threshold.
        if candidate_peers.len() + peers.len() < E::MAXIMUM_CANDIDATE_PEERS {
            // Proceed to insert each new candidate peer IP.
            for peer_ip in peers.iter().take(E::MAXIMUM_CANDIDATE_PEERS) {
                // Ensure the peer is not self and is a new candidate peer.
                let is_self = *peer_ip == self.local_ip
                    || (peer_ip.ip().is_unspecified() || peer_ip.ip().is_loopback()) && peer_ip.port() == self.local_ip.port();
                if !is_self && !self.is_connected_to(*peer_ip).await {
                    candidate_peers.insert(*peer_ip);
                }
            }
        }
    }

    ///
    /// Sends the given message to specified peer.
    ///
    async fn send(&self, peer: SocketAddr, message: Message<N, E>) {
        let target_peer = self.connected_peers.read().await.get(&peer).cloned();
        match target_peer {
            Some((_, outbound)) => {
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
    async fn propagate(&self, sender: SocketAddr, mut message: Message<N, E>) {
        // Perform ahead-of-time, non-blocking serialization just once for applicable objects.
        if let Message::UnconfirmedBlock(_, _, ref mut data) = message {
            let serialized_block = Data::serialize(data.clone()).await.expect("Block serialization is bugged");
            let _ = std::mem::replace(data, Data::Buffer(serialized_block));
        }

        // Iterate through all peers that are not the sender, sync node, or beacon node.
        for peer in self
            .connected_peers()
            .await
            .iter()
            .filter(|peer_ip| {
                let peer_str = peer_ip.to_string();
                *peer_ip != &sender && !E::SYNC_NODES.contains(&peer_str.as_str()) && !E::BEACON_NODES.contains(&peer_str.as_str())
            })
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
        self.candidate_peers.write().await.clear();
        self.restricted_peers.write().await.clear();
        self.seen_inbound_connections.write().await.clear();
        self.seen_outbound_connections.write().await.clear();
    }
}

///
/// The state for each connected client.
///
struct Peer<N: Network, E: Environment> {
    /// The IP address of the peer, with the port set to the listener port.
    listener_ip: SocketAddr,
    /// The message version of the peer.
    version: u32,
    /// The node type of the peer.
    node_type: NodeType,
    /// The node type of the peer.
    status: Status,
    /// The block header of the peer.
    block_header: BlockHeader<N>,
    /// The timestamp of the last message received from this peer.
    last_seen: Instant,
    /// The TCP socket that handles sending and receiving data with this peer.
    outbound_socket: Framed<TcpStream, Message<N, E>>,
    /// The `outbound_handler` half of the MPSC message channel, used to receive messages from peers.
    /// When a message is received on this `OutboundHandler`, it will be written to the socket.
    outbound_handler: OutboundHandler<N, E>,
    /// The map of block hashes to their last seen timestamp.
    seen_inbound_blocks: HashMap<N::BlockHash, SystemTime>,
    /// The map of transaction IDs to their last seen timestamp.
    seen_inbound_transactions: HashMap<N::TransactionID, SystemTime>,
    /// The map of peers to a map of block hashes to their last seen timestamp.
    seen_outbound_blocks: HashMap<N::BlockHash, SystemTime>,
    /// The map of peers to a map of transaction IDs to their last seen timestamp.
    seen_outbound_transactions: HashMap<N::TransactionID, SystemTime>,
}

impl<N: Network, E: Environment> Peer<N, E> {
    /// Create a new instance of `Peer`.
    async fn new(
        stream: TcpStream,
        local_ip: SocketAddr,
        local_nonce: u64,
        local_status: &Status,
        peers_router: &PeersRouter<N, E>,
        ledger_reader: &LedgerReader<N>,
        connected_nonces: &[u64],
    ) -> Result<Self> {
        // Construct the socket.
        let mut outbound_socket = Framed::new(stream, Message::<N, E>::PeerRequest);

        // Perform the handshake before proceeding.
        let (peer_ip, peer_nonce, node_type, status) = Peer::handshake(
            &mut outbound_socket,
            local_ip,
            local_nonce,
            local_status,
            ledger_reader.latest_cumulative_weight(),
            connected_nonces,
        )
        .await?;

        // Send the first `Ping` message to the peer.
        let message = Message::Ping(
            E::MESSAGE_VERSION,
            E::MAXIMUM_FORK_DEPTH,
            E::NODE_TYPE,
            local_status.get(),
            ledger_reader.latest_block_hash(),
            Data::Object(ledger_reader.latest_block_header()),
        );
        trace!("Sending '{}' to {}", message.name(), peer_ip);
        outbound_socket.send(message).await?;

        // Create a channel for this peer.
        let (outbound_router, outbound_handler) = mpsc::channel(1024);

        // Add an entry for this `Peer` in the connected peers.
        peers_router
            .send(PeersRequest::PeerConnected(peer_ip, peer_nonce, outbound_router))
            .await?;

        Ok(Peer {
            listener_ip: peer_ip,
            version: 0,
            node_type,
            status,
            block_header: N::genesis_block().header().clone(),
            last_seen: Instant::now(),
            outbound_socket,
            outbound_handler,
            seen_inbound_blocks: Default::default(),
            seen_inbound_transactions: Default::default(),
            seen_outbound_blocks: Default::default(),
            seen_outbound_transactions: Default::default(),
        })
    }

    /// Returns the IP address of the peer, with the port set to the listener port.
    fn peer_ip(&self) -> SocketAddr {
        self.listener_ip
    }

    /// Sends the given message to this peer.
    async fn send(&mut self, message: Message<N, E>) -> Result<()> {
        trace!("Sending '{}' to {}", message.name(), self.peer_ip());
        self.outbound_socket.send(message).await?;
        Ok(())
    }

    /// Performs the handshake protocol, returning the listener IP and nonce of the peer upon success.
    async fn handshake(
        outbound_socket: &mut Framed<TcpStream, Message<N, E>>,
        local_ip: SocketAddr,
        local_nonce: u64,
        local_status: &Status,
        local_cumulative_weight: u128,
        connected_nonces: &[u64],
    ) -> Result<(SocketAddr, u64, NodeType, Status)> {
        // Get the IP address of the peer.
        let mut peer_ip = outbound_socket.get_ref().peer_addr()?;

        // Retrieve the genesis block header.
        let genesis_header = N::genesis_block().header();

        // Send a challenge request to the peer.
        let message = Message::<N, E>::ChallengeRequest(
            E::MESSAGE_VERSION,
            E::MAXIMUM_FORK_DEPTH,
            E::NODE_TYPE,
            local_status.get(),
            local_ip.port(),
            local_nonce,
            local_cumulative_weight,
        );
        trace!("Sending '{}-A' to {}", message.name(), peer_ip);
        outbound_socket.send(message).await?;

        // Wait for the counterparty challenge request to come in.
        let (peer_nonce, node_type, status) = match outbound_socket.next().await {
            Some(Ok(message)) => {
                // Process the message.
                trace!("Received '{}-B' from {}", message.name(), peer_ip);
                match message {
                    Message::ChallengeRequest(
                        version,
                        fork_depth,
                        node_type,
                        peer_status,
                        listener_port,
                        peer_nonce,
                        peer_cumulative_weight,
                    ) => {
                        // Ensure the message protocol version is not outdated.
                        if version < E::MESSAGE_VERSION {
                            warn!("Dropping {} on version {} (outdated)", peer_ip, version);
                            return Err(anyhow!("Dropping {} on version {} (outdated)", peer_ip, version));
                        }
                        // Ensure the maximum fork depth is correct.
                        if fork_depth != E::MAXIMUM_FORK_DEPTH {
                            return Err(anyhow!(
                                "Dropping {} for an incorrect maximum fork depth of {}",
                                peer_ip,
                                fork_depth
                            ));
                        }
                        // If this node is not a sync node and is syncing, the peer is a sync node, and this node is ahead, proceed to disconnect.
                        if E::NODE_TYPE != NodeType::Sync
                            && local_status.is_syncing()
                            && node_type == NodeType::Sync
                            && local_cumulative_weight > peer_cumulative_weight
                        {
                            return Err(anyhow!("Dropping {} as this node is ahead", peer_ip));
                        }
                        // If this node is a sync node, the peer is not a sync node and is syncing, and the peer is ahead, proceed to disconnect.
                        if E::NODE_TYPE == NodeType::Sync
                            && node_type != NodeType::Sync
                            && peer_status == State::Syncing
                            && peer_cumulative_weight > local_cumulative_weight
                        {
                            return Err(anyhow!("Dropping {} as this node is ahead", peer_ip));
                        }
                        // Ensure the peer is not this node.
                        if local_nonce == peer_nonce {
                            return Err(anyhow!("Attempted to connect to self (nonce = {})", peer_nonce));
                        }
                        // Ensure the peer is not already connected to this node.
                        if connected_nonces.contains(&peer_nonce) {
                            return Err(anyhow!("Already connected to a peer with nonce {}", peer_nonce));
                        }
                        // Verify the listener port.
                        if peer_ip.port() != listener_port {
                            // Update the peer IP to the listener port.
                            peer_ip.set_port(listener_port);
                            // Ensure the claimed listener port is open.
                            let stream =
                                match timeout(Duration::from_millis(E::CONNECTION_TIMEOUT_IN_MILLIS), TcpStream::connect(peer_ip)).await {
                                    Ok(stream) => stream,
                                    Err(error) => return Err(anyhow!("Unable to reach '{}': '{:?}'", peer_ip, error)),
                                };
                            // Error if the stream is not open.
                            if let Err(error) = stream {
                                return Err(anyhow!("Unable to reach '{}': '{}'", peer_ip, error));
                            }
                        }
                        // Send the challenge response.
                        let message = Message::ChallengeResponse(Data::Object(genesis_header.clone()));
                        trace!("Sending '{}-B' to {}", message.name(), peer_ip);
                        outbound_socket.send(message).await?;

                        // Initialize a status variable.
                        let status = Status::new();
                        status.update(peer_status);

                        (peer_nonce, node_type, status)
                    }
                    message => {
                        return Err(anyhow!(
                            "Expected challenge request, received '{}' from {}",
                            message.name(),
                            peer_ip
                        ));
                    }
                }
            }
            // An error occurred.
            Some(Err(error)) => return Err(anyhow!("Failed to get challenge request from {}: {:?}", peer_ip, error)),
            // Did not receive anything.
            None => return Err(anyhow!("Dropped prior to challenge request of {}", peer_ip)),
        };

        // Wait for the challenge response to come in.
        match outbound_socket.next().await {
            Some(Ok(message)) => {
                // Process the message.
                trace!("Received '{}-A' from {}", message.name(), peer_ip);
                match message {
                    Message::ChallengeResponse(block_header) => {
                        // Perform the deferred non-blocking deserialization of the block header.
                        let block_header = block_header.deserialize().await?;
                        match &block_header == genesis_header {
                            true => Ok((peer_ip, peer_nonce, node_type, status)),
                            false => Err(anyhow!("Challenge response from {} failed, received '{}'", peer_ip, block_header)),
                        }
                    }
                    message => Err(anyhow!(
                        "Expected challenge response, received '{}' from {}",
                        message.name(),
                        peer_ip
                    )),
                }
            }
            // An error occurred.
            Some(Err(error)) => Err(anyhow!("Failed to get challenge response from {}: {:?}", peer_ip, error)),
            // Did not receive anything.
            None => Err(anyhow!("Failed to get challenge response from {}, peer has disconnected", peer_ip)),
        }
    }

    /// A handler to process an individual peer.
    #[allow(clippy::too_many_arguments)]
    async fn handler(
        stream: TcpStream,
        local_ip: SocketAddr,
        local_nonce: u64,
        local_status: Status,
        peers_router: &PeersRouter<N, E>,
        ledger_reader: LedgerReader<N>,
        ledger_router: LedgerRouter<N>,
        prover_router: ProverRouter<N>,
        operator_router: OperatorRouter<N>,
        connected_nonces: Vec<u64>,
        connection_result: Option<ConnectionResult>,
        tasks: Tasks<task::JoinHandle<()>>,
    ) {
        let peers_router = peers_router.clone();

        let tasks_clone = tasks.clone();
        tasks.append(task::spawn(async move {
            // Register our peer with state which internally sets up some channels.
            let mut peer = match Peer::new(
                stream,
                local_ip,
                local_nonce,
                &local_status,
                &peers_router,
                &ledger_reader,
                &connected_nonces,
            )
            .await
            {
                Ok(peer) => {
                    // If the optional connection result router is given, report a successful connection result.
                    if let Some(router) = connection_result {
                        if router.send(Ok(())).is_err() {
                            warn!("Failed to report a successful connection");
                        }
                    }
                    peer
                }
                Err(error) => {
                    trace!("{}", error);
                    // If the optional connection result router is given, report a failed connection result.
                    if let Some(router) = connection_result {
                        if router.send(Err(error)).is_err() {
                            warn!("Failed to report a failed connection");
                        }
                    }
                    return;
                }
            };

            // Retrieve the peer IP.
            let peer_ip = peer.peer_ip();
            info!("Connected to {}", peer_ip);

            // Process incoming messages until this stream is disconnected.
            loop {
                tokio::select! {
                    // Message channel is routing a message outbound to the peer.
                    Some(mut message) = peer.outbound_handler.recv() => {
                        // Disconnect if the peer has not communicated back within the predefined time.
                        if peer.last_seen.elapsed() > Duration::from_secs(E::RADIO_SILENCE_IN_SECS) {
                            warn!("Peer {} has not communicated in {} seconds", peer_ip, peer.last_seen.elapsed().as_secs());
                            break;
                        } else {
                            // Ensure sufficient time has passed before needing to send the message.
                            let is_ready_to_send = match message {
                                Message::Ping(_, _, _, _, _, ref mut data) => {
                                    // Perform non-blocking serialisation of the block header.
                                    let serialized_header = Data::serialize(data.clone()).await.expect("Block header serialization is bugged");
                                    let _ = std::mem::replace(data, Data::Buffer(serialized_header));

                                    true
                                }
                                Message::UnconfirmedBlock(block_height, block_hash, ref mut data) => {
                                    // Retrieve the last seen timestamp of this block for this peer.
                                    let last_seen = peer.seen_outbound_blocks.entry(block_hash).or_insert(SystemTime::UNIX_EPOCH);
                                    let is_ready_to_send = last_seen.elapsed().unwrap().as_secs() > E::RADIO_SILENCE_IN_SECS;

                                    // Update the timestamp for the peer and sent block.
                                    peer.seen_outbound_blocks.insert(block_hash, SystemTime::now());
                                    // Report the unconfirmed block height.
                                    if is_ready_to_send {
                                        trace!("Preparing to send 'UnconfirmedBlock {}' to {}", block_height, peer_ip);
                                    }

                                    // Perform non-blocking serialization of the block (if it hasn't been serialized yet).
                                    let serialized_block = Data::serialize(data.clone()).await.expect("Block serialization is bugged");
                                    let _ = std::mem::replace(data, Data::Buffer(serialized_block));

                                    is_ready_to_send
                                }
                                Message::UnconfirmedTransaction(ref transaction) => {
                                    // Retrieve the last seen timestamp of this transaction for this peer.
                                    let last_seen = peer
                                        .seen_outbound_transactions
                                        .entry(transaction.transaction_id())
                                        .or_insert(SystemTime::UNIX_EPOCH);
                                    let is_ready_to_send = last_seen.elapsed().unwrap().as_secs() > E::RADIO_SILENCE_IN_SECS;

                                    // Update the timestamp for the peer and sent transaction.
                                    peer.seen_outbound_transactions.insert(transaction.transaction_id(), SystemTime::now());
                                    // Report the unconfirmed block height.
                                    if is_ready_to_send {
                                        trace!(
                                            "Preparing to send '{} {}' to {}",
                                            message.name(),
                                            transaction.transaction_id(),
                                            peer_ip
                                        );
                                    }
                                    is_ready_to_send
                                }
                                _ => true,
                            };
                            // Send the message if it is ready.
                            if is_ready_to_send {
                                // Route a message to the peer.
                                if let Err(error) = peer.send(message).await {
                                    warn!("[OutboundRouter] {}", error);
                                }
                            }
                        }
                    }
                    result = peer.outbound_socket.next() => match result {
                        // Received a message from the peer.
                        Some(Ok(message)) => {
                            // Disconnect if the peer has not communicated back within the predefined time.
                            match peer.last_seen.elapsed() > Duration::from_secs(E::RADIO_SILENCE_IN_SECS) {
                                true => {
                                    let last_seen = peer.last_seen.elapsed().as_secs();
                                    warn!("Failed to receive a message from {} in {} seconds", peer_ip, last_seen);
                                    break;
                                },
                                false => {
                                    // Update the last seen timestamp.
                                    peer.last_seen = Instant::now();
                                }
                            }
                            // Process the message.
                            trace!("Received '{}' from {}", message.name(), peer_ip);
                            match message {
                                Message::BlockRequest(start_block_height, end_block_height) => {
                                    // Ensure the request is within the accepted limits.
                                    let number_of_blocks = end_block_height.saturating_sub(start_block_height);
                                    if number_of_blocks > E::MAXIMUM_BLOCK_REQUEST {
                                        // Route a `Failure` to the ledger.
                                        let failure = format!("Attempted to request {} blocks", number_of_blocks);
                                        if let Err(error) = ledger_router.send(LedgerRequest::Failure(peer_ip, failure)).await {
                                            warn!("[Failure] {}", error);
                                        }
                                        continue;
                                    }
                                    // Retrieve the requested blocks.
                                    let blocks = match ledger_reader.get_blocks(start_block_height, end_block_height) {
                                        Ok(blocks) => blocks,
                                        Err(error) => {
                                            // Route a `Failure` to the ledger.
                                            if let Err(error) = ledger_router.send(LedgerRequest::Failure(peer_ip, format!("{}", error))).await {
                                                warn!("[Failure] {}", error);
                                            }
                                            continue;
                                        }
                                    };
                                    // Send a `BlockResponse` message for each block to the peer.
                                    for block in blocks {
                                        debug!("Sending 'BlockResponse {}' to {}", block.height(), peer_ip);
                                        if let Err(error) = peer.outbound_socket.send(Message::BlockResponse(Data::Object(block))).await {
                                            warn!("[BlockResponse] {}", error);
                                            break;
                                        }
                                    }
                                },
                                Message::BlockResponse(block) => {
                                    // Perform the deferred non-blocking deserialization of the block.
                                    match block.deserialize().await {
                                        // Route the `BlockResponse` to the ledger.
                                        Ok(block) => if let Err(error) = ledger_router.send(LedgerRequest::BlockResponse(peer_ip, block, prover_router.clone())).await {
                                            warn!("[BlockResponse] {}", error);
                                        },
                                        // Route the `Failure` to the ledger.
                                        Err(error) => if let Err(error) = ledger_router.send(LedgerRequest::Failure(peer_ip, format!("{}", error))).await {
                                            warn!("[Failure] {}", error);
                                        }
                                    }
                                }
                                Message::ChallengeRequest(..) | Message::ChallengeResponse(..) => {
                                    // Peer is not following the protocol.
                                    warn!("Peer {} is not following the protocol", peer_ip);
                                    break;
                                },
                                Message::Disconnect => break,
                                Message::PeerRequest => {
                                    // Send a `PeerResponse` message.
                                    if let Err(error) = peers_router.send(PeersRequest::SendPeerResponse(peer_ip)).await {
                                        warn!("[PeerRequest] {}", error);
                                    }
                                }
                                Message::PeerResponse(peer_ips) => {
                                    // Adds the given peer IPs to the list of candidate peers.
                                    if let Err(error) = peers_router.send(PeersRequest::ReceivePeerResponse(peer_ips)).await {
                                        warn!("[PeerResponse] {}", error);
                                    }
                                }
                                Message::Ping(version, fork_depth, node_type, status, block_hash, block_header) => {
                                    // Ensure the message protocol version is not outdated.
                                    if version < E::MESSAGE_VERSION {
                                        warn!("Dropping {} on version {} (outdated)", peer_ip, version);
                                        break;
                                    }
                                    // Ensure the maximum fork depth is correct.
                                    if fork_depth != E::MAXIMUM_FORK_DEPTH {
                                        warn!("Dropping {} for an incorrect maximum fork depth of {}", peer_ip, fork_depth);
                                        break;
                                    }
                                    // Perform the deferred non-blocking deserialization of the block header.
                                    match block_header.deserialize().await {
                                        Ok(block_header) => {
                                            // If this node is not a sync node and is syncing, the peer is a sync node, and this node is ahead, proceed to disconnect.
                                            if E::NODE_TYPE != NodeType::Sync
                                                && local_status.is_syncing()
                                                && node_type == NodeType::Sync
                                                && ledger_reader.latest_cumulative_weight() > block_header.cumulative_weight()
                                            {
                                                trace!("Disconnecting from {} (ahead of sync node)", peer_ip);
                                                break;
                                            }
                                            // Update the block header of the peer.
                                            peer.block_header = block_header;
                                        }
                                        Err(error) => warn!("[Ping] {}", error),
                                    }

                                    // Update the version of the peer.
                                    peer.version = version;
                                    // Update the node type of the peer.
                                    peer.node_type = node_type;
                                    // Update the status of the peer.
                                    peer.status.update(status);

                                    // Determine if the peer is on a fork (or unknown).
                                    let is_fork = match ledger_reader.get_block_hash(peer.block_header.height()) {
                                        Ok(expected_block_hash) => Some(expected_block_hash != block_hash),
                                        Err(_) => None,
                                    };
                                    // Send a `Pong` message to the peer.
                                    if let Err(error) = peer.send(Message::Pong(is_fork, Data::Object(ledger_reader.latest_block_locators()))).await {
                                        warn!("[Pong] {}", error);
                                    }
                                },
                                Message::Pong(is_fork, block_locators) => {
                                    // Perform the deferred non-blocking deserialization of block locators.
                                    let request = match block_locators.deserialize().await {
                                        // Route the `Pong` to the ledger.
                                        Ok(block_locators) => LedgerRequest::Pong(peer_ip, peer.node_type, peer.status.get(), is_fork, block_locators),
                                        // Route the `Failure` to the ledger.
                                        Err(error) => LedgerRequest::Failure(peer_ip, format!("{}", error)),
                                    };

                                    // Route the request to the ledger.
                                    if let Err(error) = ledger_router.send(request).await {
                                        warn!("[Pong] {}", error);
                                    }

                                    // Spawn an asynchronous task for the `Ping` request.
                                    let local_status = local_status.clone();
                                    let peers_router = peers_router.clone();
                                    let ledger_reader = ledger_reader.clone();
                                    tasks_clone.append(task::spawn(async move {
                                        // Sleep for the preset time before sending a `Ping` request.
                                        tokio::time::sleep(Duration::from_secs(E::PING_SLEEP_IN_SECS)).await;

                                        // Retrieve the latest ledger state.
                                        let latest_block_hash = ledger_reader.latest_block_hash();
                                        let latest_block_header = ledger_reader.latest_block_header();

                                        // Send a `Ping` request to the peer.
                                        let message = Message::Ping(E::MESSAGE_VERSION, E::MAXIMUM_FORK_DEPTH, E::NODE_TYPE, local_status.get(), latest_block_hash, Data::Object(latest_block_header));
                                        if let Err(error) = peers_router.send(PeersRequest::MessageSend(peer_ip, message)).await {
                                            warn!("[Ping] {}", error);
                                        }
                                    }));
                                }
                                Message::PoolRegister(address) => {
                                    if E::NODE_TYPE != NodeType::Operator {
                                        trace!("Skipping 'PoolRegister' from {}", peer_ip);
                                    } else if let Err(error) = operator_router.send(OperatorRequest::PoolRegister(peer_ip, address)).await {
                                        warn!("[PoolRegister] {}", error);
                                    }
                                }
                                Message::PoolRequest(share_difficulty, block_template) => {
                                    if E::NODE_TYPE != NodeType::Prover {
                                        trace!("Skipping 'PoolRequest' from {}", peer_ip);
                                    } else if let Ok(block_template) = block_template.deserialize().await {
                                        if let Err(error) = prover_router.send(ProverRequest::PoolRequest(peer_ip, share_difficulty, block_template)).await {
                                            warn!("[PoolRequest] {}", error);
                                        }
                                    } else {
                                        warn!("[PoolRequest] could not deserialize block template");
                                    }
                                }
                                Message::PoolResponse(address, block) => {
                                    if E::NODE_TYPE != NodeType::Operator {
                                        trace!("Skipping 'PoolResponse' from {}", peer_ip);
                                    } else if let Ok(block) = block.deserialize().await {
                                        if let Err(error) = operator_router.send(OperatorRequest::PoolResponse(peer_ip, block, address)).await {
                                            warn!("[PoolResponse] {}", error);
                                        }
                                    } else {
                                        warn!("[PoolResponse] could not deserialize block");
                                    }
                                }
                                Message::UnconfirmedBlock(block_height, block_hash, block) => {
                                    // Drop the peer, if they have sent more than 5 unconfirmed blocks in the last 5 seconds.
                                    let frequency = peer.seen_inbound_blocks.values().filter(|t| t.elapsed().unwrap().as_secs() <= 5).count();
                                    if frequency >= 10 {
                                        warn!("Dropping {} for spamming unconfirmed blocks (frequency = {})", peer_ip, frequency);
                                        // Send a `PeerRestricted` message.
                                        if let Err(error) = peers_router.send(PeersRequest::PeerRestricted(peer_ip)).await {
                                            warn!("[PeerRestricted] {}", error);
                                        }
                                        break;
                                    }

                                    // Retrieve the last seen timestamp of the received block.
                                    let last_seen = peer.seen_inbound_blocks.entry(block_hash).or_insert(SystemTime::UNIX_EPOCH);
                                    let is_router_ready = last_seen.elapsed().unwrap().as_secs() > E::RADIO_SILENCE_IN_SECS;

                                    // Update the timestamp for the received block.
                                    peer.seen_inbound_blocks.insert(block_hash, SystemTime::now());

                                    // Ensure the unconfirmed block is at least within 2 blocks of the latest block height,
                                    // and no more that 2 blocks ahead of the latest block height.
                                    // If it is stale, skip the routing of this unconfirmed block to the ledger.
                                    let latest_block_height = ledger_reader.latest_block_height();
                                    let lower_bound = latest_block_height.saturating_sub(2);
                                    let upper_bound = latest_block_height.saturating_add(2);
                                    let is_within_range = block_height >= lower_bound && block_height <= upper_bound;

                                    // Ensure the node is not peering.
                                    let is_node_ready = !local_status.is_peering();

                                    // If this node is a beacon or sync node, skip this message, after updating the timestamp.
                                    if E::NODE_TYPE == NodeType::Beacon || E::NODE_TYPE == NodeType::Sync || !is_router_ready || !is_within_range || !is_node_ready {
                                        trace!("Skipping 'UnconfirmedBlock {}' from {}", block_height, peer_ip)
                                    } else {
                                        // Perform the deferred non-blocking deserialization of the block.
                                        let request = match block.deserialize().await {
                                            // Ensure the claimed block height and block hash matches in the deserialized block.
                                            Ok(block) => match block_height == block.height() && block_hash == block.hash() {
                                                // Route the `UnconfirmedBlock` to the ledger.
                                                true => LedgerRequest::UnconfirmedBlock(peer_ip, block, prover_router.clone()),
                                                // Route the `Failure` to the ledger.
                                                false => LedgerRequest::Failure(peer_ip, "Malformed UnconfirmedBlock message".to_string())
                                            },
                                            // Route the `Failure` to the ledger.
                                            Err(error) => LedgerRequest::Failure(peer_ip, format!("{}", error)),
                                        };

                                        // Route the request to the ledger.
                                        if let Err(error) = ledger_router.send(request).await {
                                            warn!("[UnconfirmedBlock] {}", error);
                                        }
                                    }
                                }
                                Message::UnconfirmedTransaction(transaction) => {
                                    // Drop the peer, if they have sent more than 500 unconfirmed transactions in the last 5 seconds.
                                    let frequency = peer.seen_inbound_transactions.values().filter(|t| t.elapsed().unwrap().as_secs() <= 5).count();
                                    if frequency >= 500 {
                                        warn!("Dropping {} for spamming unconfirmed transactions (frequency = {})", peer_ip, frequency);
                                        // Send a `PeerRestricted` message.
                                        if let Err(error) = peers_router.send(PeersRequest::PeerRestricted(peer_ip)).await {
                                            warn!("[PeerRestricted] {}", error);
                                        }
                                        break;
                                    }

                                    // Retrieve the last seen timestamp of the received transaction.
                                    let last_seen = peer.seen_inbound_transactions.entry(transaction.transaction_id()).or_insert(SystemTime::UNIX_EPOCH);
                                    let is_router_ready = last_seen.elapsed().unwrap().as_secs() > E::RADIO_SILENCE_IN_SECS;

                                    // Update the timestamp for the received transaction.
                                    peer.seen_inbound_transactions.insert(transaction.transaction_id(), SystemTime::now());

                                    // Ensure the node is not peering.
                                    let is_node_ready = !local_status.is_peering();

                                    // If this node is a beacon or sync node, skip this message, after updating the timestamp.
                                    if E::NODE_TYPE == NodeType::Beacon || E::NODE_TYPE == NodeType::Sync || !is_router_ready || !is_node_ready {
                                        trace!("Skipping 'UnconfirmedTransaction {}' from {}", transaction.transaction_id(), peer_ip);
                                    } else {
                                        // Route the `UnconfirmedTransaction` to the prover.
                                        if let Err(error) = prover_router.send(ProverRequest::UnconfirmedTransaction(peer_ip, transaction)).await {
                                            warn!("[UnconfirmedTransaction] {}", error);
                                        }
                                    }
                                }
                                Message::Unused(_) => break, // Peer is not following the protocol.
                            }
                        }
                        // An error occurred.
                        Some(Err(error)) => error!("Failed to read message from {}: {}", peer_ip, error),
                        // The stream has been disconnected.
                        None => break,
                    },
                }
            }

            // When this is reached, it means the peer has disconnected.
            // Route a `Disconnect` to the ledger.
            if let Err(error) = ledger_router
                .send(LedgerRequest::Disconnect(peer_ip, "peer has disconnected".to_string()))
                .await
            {
                warn!("[Peer::Disconnect] {}", error);
            }
        }));
    }
}
