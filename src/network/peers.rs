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

use crate::{Data, Environment, LedgerReader, LedgerRouter, Message, OperatorRouter, OutboundRouter, Peer, ProverRouter};
use snarkvm::dpc::prelude::*;

use anyhow::Result;
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
    time::timeout,
};

/// Shorthand for the parent half of the `Peers` message channel.
pub(crate) type PeersRouter<N, E> = mpsc::Sender<PeersRequest<N, E>>;
#[allow(unused)]
/// Shorthand for the child half of the `Peers` message channel.
type PeersHandler<N, E> = mpsc::Receiver<PeersRequest<N, E>>;

/// Shorthand for the parent half of the connection result channel.
pub(crate) type ConnectionResult = oneshot::Sender<Result<()>>;

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
    pub(crate) async fn new(local_ip: SocketAddr, local_nonce: Option<u64>) -> Arc<Self> {
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
            connected_peers: Default::default(),
            candidate_peers: Default::default(),
            restricted_peers: Default::default(),
            seen_inbound_connections: Default::default(),
            seen_outbound_connections: Default::default(),
        });

        // Initialize the peers router process.
        {
            let peers = peers.clone();
            let (router, handler) = oneshot::channel();
            E::tasks().append(task::spawn(async move {
                // Notify the outer function that the task is ready.
                let _ = router.send(());
                // Asynchronously wait for a peers request.
                while let Some(request) = peers_handler.recv().await {
                    let peers = peers.clone();
                    // Asynchronously process a peers request.
                    E::tasks().append(task::spawn(async move {
                        // Hold the peers write lock briefly, to update the state of the peers.
                        peers.update(request).await;
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
        connected_peers.intersection(E::sync_nodes()).copied().collect()
    }

    ///
    /// TODO (howardwu): Make this operation more efficient.
    /// Returns the number of connected sync nodes.
    ///
    pub async fn number_of_connected_sync_nodes(&self) -> usize {
        let connected_peers: HashSet<SocketAddr> = self.connected_peers.read().await.keys().into_iter().copied().collect();
        connected_peers.intersection(E::sync_nodes()).count()
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
    pub(super) async fn update(&self, request: PeersRequest<N, E>) {
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
                                        &self.peers_router,
                                        ledger_reader,
                                        ledger_router,
                                        prover_router,
                                        operator_router,
                                        self.connected_nonces().await,
                                        Some(connection_result),
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
                        .filter(|(peer_ip, _)| !E::sync_nodes().contains(peer_ip) && !E::beacon_nodes().contains(peer_ip))
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
                    debug!("Exceeded maximum number of sync nodes");

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
                if number_of_connected_sync_nodes == 0 {
                    self.add_candidate_peers(E::sync_nodes().iter()).await;
                }

                // Add the beacon nodes to the list of candidate peers.
                self.add_candidate_peers(E::beacon_nodes().iter()).await;

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
                    if E::sync_nodes().contains(&peer_ip) && number_of_connected_sync_nodes >= 1 {
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
                        E::tasks().append(task::spawn(async move {
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
                            &self.peers_router,
                            ledger_reader,
                            ledger_router,
                            prover_router,
                            operator_router,
                            self.connected_nonces().await,
                            None,
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
                self.add_candidate_peers(peer_ips.iter()).await;
            }
        }
    }

    ///
    /// Adds the given peer IPs to the set of candidate peers.
    ///
    /// This method skips adding any given peers if the combined size exceeds the threshold,
    /// as the peer providing this list could be subverting the protocol.
    ///
    async fn add_candidate_peers<'a, T: ExactSizeIterator<Item = &'a SocketAddr> + IntoIterator>(&self, peers: T) {
        // Acquire the candidate peers write lock.
        let mut candidate_peers = self.candidate_peers.write().await;
        // Ensure the combined number of peers does not surpass the threshold.
        for peer_ip in peers.take(E::MAXIMUM_CANDIDATE_PEERS.saturating_sub(candidate_peers.len())) {
            // Ensure the peer is not self and is a new candidate peer.
            let is_self = *peer_ip == self.local_ip
                || (peer_ip.ip().is_unspecified() || peer_ip.ip().is_loopback()) && peer_ip.port() == self.local_ip.port();
            if !is_self && !self.is_connected_to(*peer_ip).await {
                // Proceed to insert each new candidate peer IP.
                candidate_peers.insert(*peer_ip);
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
            .filter(|peer_ip| *peer_ip != &sender && !E::sync_nodes().contains(peer_ip) && !E::beacon_nodes().contains(peer_ip))
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
