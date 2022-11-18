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

#![forbid(unsafe_code)]

#[macro_use]
extern crate async_trait;
#[macro_use]
extern crate tracing;

mod handshake;
pub use handshake::*;

mod inbound;
pub use inbound::*;

mod outbound;
pub use outbound::*;

mod peer;
pub use peer::*;

use snarkos_node_executor::{spawn_task, spawn_task_loop, Executor, NodeType};
use snarkos_node_messages::*;
use snarkvm::prelude::{Address, Network, PuzzleCommitment};

use anyhow::Result;
use indexmap::{IndexMap, IndexSet};
use rand::{prelude::IteratorRandom, rngs::OsRng, Rng};
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{atomic::AtomicU8, Arc},
    time::{Duration, Instant, SystemTime},
};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::{mpsc, mpsc::error::SendError, RwLock},
    time::timeout,
};
use tokio_stream::StreamExt;
use tokio_util::codec::Framed;

// TODO (raychu86): Move this declaration.
const ALEO_MAXIMUM_FORK_DEPTH: u32 = 4096;

/// Shorthand for the parent half of the `Router` channel.
pub type RouterSender<N> = mpsc::Sender<RouterRequest<N>>;
/// Shorthand for the child half of the `Router` channel.
pub type RouterReceiver<N> = mpsc::Receiver<RouterRequest<N>>;

/// The first-seen port number, number of attempts, and timestamp of the last inbound connection request.
type ConnectionStats = ((u16, u32), SystemTime);

/// An enum of requests that the `Router` processes.
pub enum RouterRequest<N: Network> {
    /// Heartbeat
    Heartbeat,
    /// MessagePropagate := (message, \[ excluded_peers \])
    MessagePropagate(Message<N>, Vec<SocketAddr>),
    /// MessagePropagateBeacon := (message, \[ excluded_beacons \])
    MessagePropagateBeacon(Message<N>, Vec<SocketAddr>),
    /// MessageSend := (peer_ip, message)
    MessageSend(SocketAddr, Message<N>),
    /// PeerConnect := (peer_ip)
    PeerConnect(SocketAddr),
    /// PeerConnecting := (stream, peer_ip)
    PeerConnecting(TcpStream, SocketAddr),
    /// PeerConnected := (peer, outbound_socket, peer_handler)
    PeerConnected(Peer<N>, Framed<TcpStream, MessageCodec<N>>, PeerHandler<N>),
    /// PeerDisconnected := (peer_ip)
    PeerDisconnected(SocketAddr),
    /// PeerRestricted := (peer_ip)
    PeerRestricted(SocketAddr),
    /// SendPeerResponse := (peer_ip)
    SendPeerResponse(SocketAddr),
    /// ReceivePeerResponse := (\[peer_ip\])
    ReceivePeerResponse(Vec<SocketAddr>),
}

#[derive(Clone, Debug)]
pub struct Router<N: Network> {
    /// The router sender.
    router_sender: RouterSender<N>,
    /// The local IP of the node.
    local_ip: SocketAddr,
    /// The address of the node.
    address: Address<N>,
    /// The set of trusted peers.
    trusted_peers: Arc<IndexSet<SocketAddr>>,
    /// The map of connected peer IPs to their peer handlers.
    connected_peers: Arc<RwLock<IndexMap<SocketAddr, Peer<N>>>>,
    /// The set of candidate peer IPs.
    candidate_peers: Arc<RwLock<IndexSet<SocketAddr>>>,
    /// The set of restricted peer IPs.
    restricted_peers: Arc<RwLock<IndexMap<SocketAddr, Instant>>>,
    /// The map of peers to their first-seen port number, number of attempts, and timestamp of the last inbound connection request.
    seen_inbound_connections: Arc<RwLock<IndexMap<SocketAddr, ConnectionStats>>>,
    /// The map of peers to the timestamp of their last outbound connection request.
    seen_outbound_connections: Arc<RwLock<IndexMap<SocketAddr, SystemTime>>>,
    /// The map of peer IPs to the number of puzzle requests.
    pub seen_inbound_puzzle_requests: Arc<RwLock<IndexMap<SocketAddr, Arc<AtomicU8>>>>,
    /// The map of block hashes to their last seen timestamp.
    pub seen_inbound_blocks: Arc<RwLock<IndexMap<N::BlockHash, SystemTime>>>,
    /// The map of solution commitments to their last seen timestamp.
    pub seen_inbound_solutions: Arc<RwLock<IndexMap<PuzzleCommitment<N>, SystemTime>>>,
    /// The map of transaction IDs to their last seen timestamp.
    pub seen_inbound_transactions: Arc<RwLock<IndexMap<N::TransactionID, SystemTime>>>,
    /// The map of block hashes to their last seen timestamp.
    pub seen_outbound_blocks: Arc<RwLock<IndexMap<N::BlockHash, SystemTime>>>,
    /// The map of solution commitments to their last seen timestamp.
    pub seen_outbound_solutions: Arc<RwLock<IndexMap<PuzzleCommitment<N>, SystemTime>>>,
    /// The map of transaction IDs to their last seen timestamp.
    pub seen_outbound_transactions: Arc<RwLock<IndexMap<N::TransactionID, SystemTime>>>,
}

#[rustfmt::skip]
impl<N: Network> Router<N> {
    /// The maximum duration in seconds permitted for establishing a connection with a node, before dropping the connection.
    const CONNECTION_TIMEOUT_IN_MILLIS: u64 = 1000;
    /// The duration in seconds to sleep in between heartbeat executions.
    const HEARTBEAT_IN_SECS: u64 = 9; // 9 seconds
    /// The frequency at which the node sends a puzzle request.
    const PUZZLE_REQUEST_IN_SECS: u64 = N::ANCHOR_TIME as u64;
    /// The maximum number of puzzle requests per interval.
    const MAXIMUM_PUZZLE_REQUESTS_PER_INTERVAL: u8 = 10;
    /// The maximum number of candidate peers permitted to be stored in the node.
    const MAXIMUM_CANDIDATE_PEERS: usize = 10_000;
    /// The maximum number of connection failures permitted by an inbound connecting peer.
    const MAXIMUM_CONNECTION_FAILURES: u32 = 3;
    /// The duration in seconds to sleep in between ping requests with a connected peer.
    const PING_SLEEP_IN_SECS: u64 = 60; // 1 minute
    /// The duration in seconds after which a connected peer is considered inactive or
    /// disconnected if no message has been received in the meantime.
    const RADIO_SILENCE_IN_SECS: u64 = 180; // 3 minutes
}

impl<N: Network> Router<N> {
    /// Initializes a new `Router` instance.
    pub async fn new<E: Handshake + Inbound<N> + Outbound>(
        node_ip: SocketAddr,
        address: Address<N>,
        trusted_peers: &[SocketAddr],
    ) -> Result<(Self, RouterReceiver<N>)> {
        // Initialize a new TCP listener at the given IP.
        let (local_ip, listener) = match TcpListener::bind(node_ip).await {
            Ok(listener) => (listener.local_addr().expect("Failed to fetch the local IP"), listener),
            Err(error) => panic!("Failed to bind listener: {error:?}. Check if another Aleo node is running"),
        };

        // Initialize an MPSC channel for sending requests to the `Router` struct.
        let (router_sender, router_receiver) = mpsc::channel(1024);

        // Initialize the router.
        let router = Self {
            router_sender,
            local_ip,
            address,
            trusted_peers: Arc::new(trusted_peers.iter().copied().collect()),
            connected_peers: Default::default(),
            candidate_peers: Default::default(),
            restricted_peers: Default::default(),
            seen_inbound_puzzle_requests: Default::default(),
            seen_inbound_connections: Default::default(),
            seen_outbound_connections: Default::default(),
            seen_inbound_blocks: Default::default(),
            seen_inbound_solutions: Default::default(),
            seen_inbound_transactions: Default::default(),
            seen_outbound_blocks: Default::default(),
            seen_outbound_solutions: Default::default(),
            seen_outbound_transactions: Default::default(),
        };

        // Initialize the listener.
        router.initialize_listener::<E>(listener).await;
        // Initialize the heartbeat.
        router.initialize_heartbeat::<E>().await;
        // Initialize the puzzle request.
        router.initialize_puzzle_request::<E>().await;
        // Initialize the report.
        router.initialize_report::<E>().await;
        // Initialize the GC.
        router.initialize_gc::<E>().await;

        Ok((router, router_receiver))
    }

    /// Returns `true` if the given IP is this node.
    pub fn is_local_ip(&self, ip: &SocketAddr) -> bool {
        *ip == self.local_ip || (ip.ip().is_unspecified() || ip.ip().is_loopback()) && ip.port() == self.local_ip.port()
    }

    /// Returns the IP address of this node.
    pub const fn local_ip(&self) -> &SocketAddr {
        &self.local_ip
    }

    /// Returns `true` if the node is connected to the given IP.
    pub async fn is_connected_to(&self, ip: SocketAddr) -> bool {
        self.connected_peers.read().await.contains_key(&ip)
    }

    /// Returns `true` if the given IP is restricted.
    pub async fn is_restricted(&self, ip: SocketAddr) -> bool {
        match self.restricted_peers.read().await.get(&ip) {
            Some(timestamp) => timestamp.elapsed().as_secs() < Self::RADIO_SILENCE_IN_SECS,
            None => false,
        }
    }

    /// Returns the list of trusted peers.
    pub fn trusted_peers(&self) -> &IndexSet<SocketAddr> {
        &self.trusted_peers
    }

    /// Returns the list of metrics for the connected peers.
    pub async fn connected_metrics(&self) -> Vec<(SocketAddr, NodeType)> {
        let mut connected_metrics = Vec::new();
        for (ip, peer) in self.connected_peers.read().await.iter() {
            connected_metrics.push((*ip, peer.node_type().await));
        }
        connected_metrics
    }

    /// Returns the list of connected peers.
    pub async fn connected_peers(&self) -> Vec<SocketAddr> {
        self.connected_peers.read().await.keys().copied().collect()
    }

    /// Returns the list of connected peers that are beacons.
    pub async fn connected_beacons(&self) -> Vec<SocketAddr> {
        let mut connected_beacons = Vec::new();
        for (ip, peer) in self.connected_peers.read().await.iter() {
            if peer.is_beacon().await {
                connected_beacons.push(*ip);
            }
        }
        connected_beacons
    }

    /// Returns the list of reliable peers.
    pub async fn reliable_peers(&self) -> Vec<SocketAddr> {
        let mut connected_peers: Vec<_> = self.connected_peers.read().await.keys().copied().collect();
        connected_peers.retain(|ip| self.trusted_peers.contains(ip));
        connected_peers
    }

    /// Returns the list of candidate peers.
    pub async fn candidate_peers(&self) -> IndexSet<SocketAddr> {
        self.candidate_peers.read().await.clone()
    }

    /// Returns the list of restricted peers.
    pub async fn restricted_peers(&self) -> Vec<SocketAddr> {
        self.restricted_peers.read().await.keys().copied().collect()
    }

    /// Returns the number of connected peers.
    pub async fn number_of_connected_peers(&self) -> usize {
        self.connected_peers.read().await.len()
    }

    /// Returns the number of candidate peers.
    pub async fn number_of_candidate_peers(&self) -> usize {
        self.candidate_peers.read().await.len()
    }

    /// Returns the number of restricted peers.
    pub async fn number_of_restricted_peers(&self) -> usize {
        self.restricted_peers.read().await.len()
    }

    /// Sends a "PuzzleRequest" to a reliable peer.
    pub async fn send_puzzle_request(&self, node_type: NodeType) {
        // Retrieve a reliable peer.
        let reliable_peer = match node_type.is_validator() {
            true => self.connected_beacons().await.first().copied(),
            false => self.reliable_peers().await.first().copied(),
        };
        // If a reliable peer exists, send a "PuzzleRequest" to it.
        if let Some(reliable_peer) = reliable_peer {
            // Send the "PuzzleRequest" to the reliable peer.
            let request = RouterRequest::MessageSend(reliable_peer, Message::PuzzleRequest(PuzzleRequest));
            if let Err(error) = self.process(request).await {
                warn!("[PuzzleRequest] {error}");
            }
        } else {
            warn!("[PuzzleRequest] There are no reliable peers available yet");
        }
    }
}

impl<N: Network> Router<N> {
    /// Initialize the handler for router requests.
    pub async fn initialize_handler<E: Handshake + Inbound<N> + Outbound>(
        &self,
        executor: E,
        mut router_receiver: RouterReceiver<N>,
    ) {
        let router = self.clone();
        spawn_task_loop!(E, {
            // Asynchronously wait for a router request.
            while let Some(request) = router_receiver.recv().await {
                let router = router.clone();
                let executor_clone = executor.clone();
                spawn_task!(E, {
                    // Update the router.
                    router.handler::<E>(executor_clone, request).await;
                });
            }
        });
    }

    /// Initialize the connection listener for new peers.
    async fn initialize_listener<E: Handshake>(&self, listener: TcpListener) {
        let router = self.clone();
        spawn_task_loop!(E, {
            info!("Listening for peers at {}", router.local_ip);
            loop {
                // Don't accept connections if the node is breaching the configured peer limit.
                if router.number_of_connected_peers().await < E::MAXIMUM_NUMBER_OF_PEERS {
                    // Asynchronously wait for an inbound TcpStream.
                    match listener.accept().await {
                        // Process the inbound connection request.
                        Ok((stream, peer_ip)) => {
                            if let Err(error) = router.process(RouterRequest::PeerConnecting(stream, peer_ip)).await {
                                error!("Failed to send request to peers: {error}")
                            }
                        }
                        Err(error) => error!("Failed to accept a connection: {error}"),
                    }
                    // Add a small delay to prevent overloading the network from handshakes.
                    tokio::time::sleep(Duration::from_millis(150)).await;
                } else {
                    // Add a sleep delay as the node has reached peer capacity.
                    tokio::time::sleep(Duration::from_secs(10)).await;
                }
            }
        });
    }

    /// Initialize a new instance of the heartbeat.
    async fn initialize_heartbeat<E: Executor>(&self) {
        let router = self.clone();
        spawn_task_loop!(E, {
            loop {
                // Transmit a heartbeat request to the router.
                if let Err(error) = router.process(RouterRequest::Heartbeat).await {
                    error!("Failed to send heartbeat to router: {error}")
                }
                // Sleep for `Self::HEARTBEAT_IN_SECS` seconds.
                tokio::time::sleep(Duration::from_secs(Self::HEARTBEAT_IN_SECS)).await;
            }
        });
    }

    /// Initialize a new instance of the puzzle request.
    async fn initialize_puzzle_request<E: Executor>(&self) {
        if !E::node_type().is_beacon() {
            let router = self.clone();
            spawn_task_loop!(E, {
                loop {
                    // Send a "PuzzleRequest".
                    router.send_puzzle_request(E::node_type()).await;
                    // Sleep for `Self::PUZZLE_REQUEST_IN_SECS` seconds.
                    tokio::time::sleep(Duration::from_secs(Self::PUZZLE_REQUEST_IN_SECS)).await;
                }
            });
        }
    }

    /// Initialize a new instance of the report.
    async fn initialize_report<E: Executor>(&self) {
        let router = self.clone();
        spawn_task_loop!(E, {
            let url = "https://vm.aleo.org/testnet3/report";
            loop {
                // Prepare the report.
                let mut report = HashMap::new();
                report.insert("node_address".to_string(), router.address.to_string());
                report.insert("node_type".to_string(), E::node_type().to_string());
                // Transmit the report.
                if reqwest::Client::new().post(url).json(&report).send().await.is_err() {
                    warn!("Failed to send report");
                }
                // Sleep for a fixed duration in seconds.
                tokio::time::sleep(Duration::from_secs(3600 * 6)).await;
            }
        });
    }

    /// Initialize a new instance of the garbage collector.
    async fn initialize_gc<E: Executor>(&self) {
        let router = self.clone();
        spawn_task_loop!(E, {
            loop {
                // Sleep for the interval.
                tokio::time::sleep(Duration::from_secs(Self::RADIO_SILENCE_IN_SECS)).await;

                // Clear the seen puzzle requests.
                router.seen_inbound_puzzle_requests.write().await.clear();
                // Clear the seen unconfirmed blocks.
                router.seen_inbound_blocks.write().await.retain(|_, timestamp| {
                    timestamp.elapsed().unwrap_or_default().as_secs() <= Self::RADIO_SILENCE_IN_SECS
                });
                // Clear the seen unconfirmed solutions.
                router.seen_inbound_solutions.write().await.retain(|_, timestamp| {
                    timestamp.elapsed().unwrap_or_default().as_secs() <= Self::RADIO_SILENCE_IN_SECS
                });
                // Clear the seen unconfirmed transactions.
                router.seen_inbound_transactions.write().await.retain(|_, timestamp| {
                    timestamp.elapsed().unwrap_or_default().as_secs() <= Self::RADIO_SILENCE_IN_SECS
                });
                // Clear the seen unconfirmed blocks.
                router.seen_outbound_blocks.write().await.retain(|_, timestamp| {
                    timestamp.elapsed().unwrap_or_default().as_secs() <= Self::RADIO_SILENCE_IN_SECS
                });
                // Clear the seen unconfirmed solutions.
                router.seen_outbound_solutions.write().await.retain(|_, timestamp| {
                    timestamp.elapsed().unwrap_or_default().as_secs() <= Self::RADIO_SILENCE_IN_SECS
                });
                // Clear the seen unconfirmed transactions.
                router.seen_outbound_transactions.write().await.retain(|_, timestamp| {
                    timestamp.elapsed().unwrap_or_default().as_secs() <= Self::RADIO_SILENCE_IN_SECS
                });
            }
        });
    }
}

impl<N: Network> Router<N> {
    /// Routes the given request to the router to process during `Self::handler`.
    pub async fn process(&self, request: RouterRequest<N>) -> Result<(), SendError<RouterRequest<N>>> {
        self.router_sender.send(request).await
    }

    /// Performs the given `request` to the peers.
    /// All requests must go through this `handler`, so that a unified view is preserved.
    pub(crate) async fn handler<E: Handshake + Inbound<N> + Outbound>(&self, executor: E, request: RouterRequest<N>) {
        match request {
            RouterRequest::Heartbeat => self.handle_heartbeat::<E>().await,
            RouterRequest::MessagePropagate(message, excluded_peers) => {
                self.handle_propagate(message, excluded_peers).await
            }
            RouterRequest::MessagePropagateBeacon(message, excluded_beacons) => {
                self.handle_propagate_beacon(message, excluded_beacons).await
            }
            RouterRequest::MessageSend(sender, message) => self.handle_send(sender, message).await,
            RouterRequest::PeerConnect(peer_ip) => self.handle_peer_connect::<E>(peer_ip).await,
            RouterRequest::PeerConnecting(stream, peer_ip) => self.handle_peer_connecting::<E>(stream, peer_ip).await,
            RouterRequest::PeerConnected(peer, outbound_socket, peer_handler) => {
                // Add an entry for this `Peer` in the connected peers.
                self.connected_peers.write().await.insert(*peer.ip(), peer.clone());
                // Remove an entry for this `Peer` in the candidate peers, if it exists.
                self.candidate_peers.write().await.remove(peer.ip());
                // Handle the peer connection.
                self.handle_peer_connected::<E>(executor, peer, outbound_socket, peer_handler).await;
            }
            RouterRequest::PeerDisconnected(peer_ip) => {
                // Remove an entry for this `Peer` in the connected peers, if it exists.
                self.connected_peers.write().await.remove(&peer_ip);
                // Add an entry for this `Peer` in the candidate peers.
                self.candidate_peers.write().await.insert(peer_ip);
            }
            RouterRequest::PeerRestricted(peer_ip) => {
                // Remove an entry for this `Peer` in the connected peers, if it exists.
                self.connected_peers.write().await.remove(&peer_ip);
                // Add an entry for this `Peer` in the restricted peers.
                self.restricted_peers.write().await.insert(peer_ip, Instant::now());
            }
            RouterRequest::SendPeerResponse(recipient) => {
                // Send a `PeerResponse` message.
                let connected_peers = self.connected_peers().await;
                self.handle_send(recipient, Message::PeerResponse(PeerResponse { peers: connected_peers })).await;
            }
            RouterRequest::ReceivePeerResponse(peer_ips) => {
                self.add_candidate_peers(peer_ips.iter()).await;
            }
        }
    }

    /// Handles the heartbeat request.
    async fn handle_heartbeat<E: Handshake>(&self) {
        debug!("Peers: {:?}", self.connected_peers().await);

        // TODO (howardwu): Remove this in Phase 3.
        if E::node_type().is_beacon() {
            // Proceed to send disconnect requests to these peers.
            for peer_ip in self.connected_peers().await {
                if !self.trusted_peers().contains(&peer_ip) {
                    info!("Disconnecting from '{peer_ip}' (exceeded maximum connections)");
                    self.handle_send(peer_ip, Message::Disconnect(DisconnectReason::TooManyPeers.into())).await;
                    // Add an entry for this `Peer` in the restricted peers.
                    self.restricted_peers.write().await.insert(peer_ip, Instant::now());
                }
            }
        }

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
                .keys()
                .filter(
                    |peer_ip| /* !E::beacon_nodes().contains(peer_ip) && */ !self.trusted_peers().contains(*peer_ip),
                )
                .take(num_excess_peers)
                .copied()
                .collect::<Vec<SocketAddr>>();

            // Proceed to send disconnect requests to these peers.
            for peer_ip in peer_ips_to_disconnect {
                info!("Disconnecting from '{peer_ip}' (exceeded maximum connections)");
                self.handle_send(peer_ip, Message::Disconnect(DisconnectReason::TooManyPeers.into())).await;
                // Add an entry for this `Peer` in the restricted peers.
                self.restricted_peers.write().await.insert(peer_ip, Instant::now());
            }
        }

        // TODO (howardwu): This logic can be optimized and unified with the context around it.
        // Determine if the node is connected to more sync nodes than allowed.
        let connected_beacons = self.connected_beacons().await;
        let number_of_connected_beacons = connected_beacons.len();
        let num_excess_beacons = number_of_connected_beacons.saturating_sub(1);
        if num_excess_beacons > 0 {
            debug!("Exceeded maximum number of beacons");

            // Proceed to send disconnect requests to these peers.
            for peer_ip in connected_beacons.iter().copied().choose_multiple(&mut OsRng::default(), num_excess_beacons)
            {
                info!("Disconnecting from 'beacon' {peer_ip} (exceeded maximum connections)");
                self.handle_send(peer_ip, Message::Disconnect(DisconnectReason::TooManyPeers.into())).await;
                // Add an entry for this `Peer` in the restricted peers.
                self.restricted_peers.write().await.insert(peer_ip, Instant::now());
            }
        }

        // Ensure that the trusted nodes are connected.
        if !self.trusted_peers().is_empty() {
            let connected_peers = self.connected_peers().await.into_iter().collect::<IndexSet<_>>();
            let disconnected_trusted_nodes = self.trusted_peers().difference(&connected_peers).copied();
            for peer_ip in disconnected_trusted_nodes {
                if let Err(error) = self.process(RouterRequest::PeerConnect(peer_ip)).await {
                    warn!("Failed to transmit the request: '{error}'");
                }
            }
        }

        // Skip if the number of connected peers is above the minimum threshold.
        match number_of_connected_peers < E::MINIMUM_NUMBER_OF_PEERS {
            true => {
                if number_of_connected_peers > 0 {
                    trace!("Sending requests for more peer connections");
                    // Request more peers if the number of connected peers is below the threshold.
                    for peer_ip in self.connected_peers().await.iter().choose_multiple(&mut OsRng::default(), 3) {
                        self.handle_send(*peer_ip, Message::PeerRequest(PeerRequest)).await;
                    }
                }
            }
            false => return,
        };

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
            // TODO (howardwu): This check is skipped because we no longer have a fixed set of beacon nodes.
            //  Introduce network-level connection safety for beacon nodes.
            // // Ensure this node is not connected to more than the permitted number of sync nodes.
            // if E::beacon_nodes().contains(&peer_ip) && number_of_connected_beacons >= 1 {
            //     continue;
            // }

            if !self.is_connected_to(peer_ip).await {
                trace!("Attempting connection to '{peer_ip}'...");
                if let Err(error) = self.process(RouterRequest::PeerConnect(peer_ip)).await {
                    warn!("Failed to transmit the request: '{error}'");
                }
            }
        }
    }

    /// Handles the request to connect to the given IP.
    async fn handle_peer_connect<E: Handshake>(&self, peer_ip: SocketAddr) {
        // Ensure the peer IP is not this node.
        if self.is_local_ip(&peer_ip) {
            debug!("Skipping connection request to '{peer_ip}' (attempted to self-connect)");
        }
        // Ensure the node does not surpass the maximum number of peer connections.
        else if self.number_of_connected_peers().await >= E::MAXIMUM_NUMBER_OF_PEERS {
            debug!("Skipping connection request to '{peer_ip}' (maximum peers reached)");
        }
        // Ensure the peer is a new connection.
        else if self.is_connected_to(peer_ip).await {
            debug!("Skipping connection request to '{peer_ip}' (already connected)");
        }
        // Ensure the peer is not restricted.
        else if self.is_restricted(peer_ip).await {
            debug!("Skipping connection request to '{peer_ip}' (restricted)");
        }
        // Attempt to open a TCP stream.
        else {
            // Lock seen_outbound_connections for further processing.
            let mut seen_outbound_connections = self.seen_outbound_connections.write().await;

            // Ensure the node respects the connection frequency limit.
            let last_seen = seen_outbound_connections.entry(peer_ip).or_insert(SystemTime::UNIX_EPOCH);
            let elapsed = last_seen.elapsed().unwrap_or(Duration::MAX).as_secs();
            if elapsed < Self::RADIO_SILENCE_IN_SECS {
                trace!("Skipping connection request to '{peer_ip}' (tried {elapsed} secs ago)");
            } else {
                debug!("Connecting to '{peer_ip}'...");
                // Update the last seen timestamp for this peer.
                seen_outbound_connections.insert(peer_ip, SystemTime::now());

                // Release the lock over seen_outbound_connections.
                drop(seen_outbound_connections);

                // Initialize the peer.
                match timeout(Duration::from_millis(Self::CONNECTION_TIMEOUT_IN_MILLIS), TcpStream::connect(peer_ip))
                    .await
                {
                    Ok(stream) => match stream {
                        Ok(stream) => {
                            let router = self.clone();
                            spawn_task!(E, {
                                if let Err(error) = E::handshake(router, stream).await {
                                    trace!("{error}");
                                }
                            });
                        }
                        Err(error) => {
                            trace!("Failed to connect to '{peer_ip}': '{:?}'", error);
                            self.candidate_peers.write().await.remove(&peer_ip);
                        }
                    },
                    Err(error) => {
                        error!("Unable to reach '{peer_ip}': '{:?}'", error);
                        self.candidate_peers.write().await.remove(&peer_ip);
                    }
                };
            }
        }
    }

    /// Handles the peer connecting request.
    async fn handle_peer_connecting<E: Handshake>(&self, stream: TcpStream, peer_ip: SocketAddr) {
        // Ensure the peer IP is not this node.
        if self.is_local_ip(&peer_ip) {
            debug!("Dropping connection request from '{peer_ip}' (attempted to self-connect)");
        }
        // Ensure the node does not surpass the maximum number of peer connections.
        else if self.number_of_connected_peers().await >= E::MAXIMUM_NUMBER_OF_PEERS {
            debug!("Dropping connection request from '{peer_ip}' (maximum peers reached)");
        }
        // Ensure the node is not already connected to this peer.
        else if self.is_connected_to(peer_ip).await {
            debug!("Dropping connection request from '{peer_ip}' (already connected)");
        }
        // Ensure the peer is not restricted.
        else if self.is_restricted(peer_ip).await {
            debug!("Dropping connection request from '{peer_ip}' (restricted)");
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
            let ((initial_port, num_attempts), last_seen) =
                seen_inbound_connections.entry(peer_lookup).or_insert(((peer_port, 0), SystemTime::UNIX_EPOCH));
            let elapsed = last_seen.elapsed().unwrap_or(Duration::MAX).as_secs();

            // Reset the inbound tracker entry for this peer, if the predefined elapsed time has passed.
            if elapsed > Self::RADIO_SILENCE_IN_SECS {
                // Reset the initial port for this peer.
                *initial_port = peer_port;
                // Reset the number of attempts for this peer.
                *num_attempts = 0;
                // Reset the last seen timestamp for this peer.
                *last_seen = SystemTime::now();
            }

            // Ensure the connecting peer has not surpassed the connection attempt limit.
            if *num_attempts > Self::MAXIMUM_CONNECTION_FAILURES {
                trace!("Dropping connection request from '{peer_ip}' (tried {elapsed} secs ago)");
                // Add an entry for this `Peer` in the restricted peers.
                self.restricted_peers.write().await.insert(peer_ip, Instant::now());
            } else {
                debug!("Received a connection request from '{peer_ip}'");
                // Update the number of attempts for this peer.
                *num_attempts += 1;

                // Release the lock over seen_inbound_connections.
                drop(seen_inbound_connections);

                // Initialize the peer handler.
                let router = self.clone();
                spawn_task!(E, {
                    if let Err(error) = E::handshake(router, stream).await {
                        trace!("{error}");
                    }
                });
            }
        }
    }

    /// Initialize the handler for the new peer.
    async fn handle_peer_connected<E: Inbound<N> + Outbound>(
        &self,
        executor: E,
        peer: Peer<N>,
        mut outbound_socket: Framed<TcpStream, MessageCodec<N>>,
        mut peer_handler: PeerHandler<N>,
    ) {
        let router = self.clone();
        spawn_task_loop!(E, {
            // Retrieve the peer IP.
            let peer_ip = *peer.ip();

            info!("Connected to '{peer_ip}'");

            // Process incoming messages until this stream is disconnected.
            let executor_clone = executor.clone();
            loop {
                tokio::select! {
                    // Message channel is routing a message outbound to the peer.
                    Some(message) = peer_handler.recv() => {
                        // Disconnect if the peer has not communicated back within the predefined time.
                        let last_seen_elapsed = peer.last_seen.read().await.elapsed().as_secs();
                        if last_seen_elapsed > Self::RADIO_SILENCE_IN_SECS {
                            warn!("Peer {peer_ip} has not communicated in {last_seen_elapsed} seconds");
                            break;
                        }

                        // Determine if this is a disconnect message.
                        let is_disconnect = matches!(message, Message::Disconnect(..));

                        // Handle the outbound message.
                        executor_clone.outbound(&peer, message, &router, &mut outbound_socket).await;

                        // If this was a disconnect message, break this connection.
                        if is_disconnect {
                            break;
                        }
                    },
                    result = outbound_socket.next() => match result {
                        // Received a message from the peer.
                        Some(Ok(message)) => {
                            // Disconnect if the peer has not communicated back within the predefined time.
                            let last_seen_elapsed = peer.last_seen.read().await.elapsed().as_secs();
                            match last_seen_elapsed > Self::RADIO_SILENCE_IN_SECS {
                                true => {
                                    warn!("Failed to receive a message from '{peer_ip}' in {last_seen_elapsed} seconds");
                                    break;
                                }
                                // Update the last seen timestamp.
                                false => *peer.last_seen.write().await = Instant::now(),
                            }

                            // Update the timestamp for the received message.
                            peer.seen_messages.write().await.insert((message.id(), rand::thread_rng().gen()), SystemTime::now());
                            // Drop the peer, if they have sent more than 50 messages in the last 5 seconds.
                            let frequency = peer.seen_messages.read().await.values().filter(|t| t.elapsed().unwrap_or_default().as_secs() <= 5).count();
                            if frequency >= 50 {
                                warn!("Dropping {peer_ip} for spamming messages (frequency = {frequency})");
                                // Send a `PeerRestricted` message.
                                if let Err(error) = router.process(RouterRequest::PeerRestricted(peer_ip)).await {
                                    warn!("[PeerRestricted] {error}");
                                }
                                break;
                            }

                            // Process the message.
                            let success = executor_clone.inbound(&peer, message, &router).await;
                            // Disconnect if the peer violated the protocol.
                            if !success {
                                warn!("Disconnecting from '{peer_ip}' (violated protocol)");
                                break;
                            }
                        },
                        // An error occurred.
                        Some(Err(error)) => error!("Failed to read message from '{peer_ip}': {error}"),
                        // The stream has been disconnected.
                        None => break,
                    },
                }
            }

            warn!("[Peer::Disconnect] Peer {peer_ip} has disconnected");

            // When this is reached, it means the peer has disconnected.
            if let Err(error) = router.process(RouterRequest::PeerDisconnected(peer_ip)).await {
                warn!("[PeerDisconnected] {error}");
            }

            // // Route a `Disconnect` to the ledger.
            // if let Err(error) = state.ledger().router()
            //     .send(LedgerRequest::Disconnect(peer_ip, DisconnectReason::PeerHasDisconnected))
            //     .await
            // {
            //     warn!("[Peer::Disconnect] {}", error);
            // }
        });
    }

    /// Sends the given message to specified peer.
    async fn handle_send(&self, peer_ip: SocketAddr, message: Message<N>) {
        let target_peer = self.connected_peers.read().await.get(&peer_ip).cloned();
        match target_peer {
            Some(peer) => {
                if let Err(error) = peer.send(message).await {
                    trace!("Failed to send message to '{peer_ip}': {error}");
                    self.connected_peers.write().await.remove(&peer_ip);
                }
            }
            None => warn!("Attempted to send to a non-connected peer {peer_ip}"),
        }
    }

    /// Sends the given message to every connected peer, excluding the sender and any specified peer IPs.
    async fn handle_propagate(&self, mut message: Message<N>, excluded_peers: Vec<SocketAddr>) {
        // Perform ahead-of-time, non-blocking serialization just once for applicable objects.
        if let Message::UnconfirmedBlock(ref mut message) = message {
            if let Ok(serialized_block) = Data::serialize(message.block.clone()).await {
                let _ = std::mem::replace(&mut message.block, Data::Buffer(serialized_block));
            } else {
                error!("Block serialization is bugged");
            }
        } else if let Message::UnconfirmedSolution(ref mut message) = message {
            if let Ok(serialized_solution) = Data::serialize(message.solution.clone()).await {
                let _ = std::mem::replace(&mut message.solution, Data::Buffer(serialized_solution));
            } else {
                error!("Solution serialization is bugged");
            }
        } else if let Message::UnconfirmedTransaction(ref mut message) = message {
            if let Ok(serialized_transaction) = Data::serialize(message.transaction.clone()).await {
                let _ = std::mem::replace(&mut message.transaction, Data::Buffer(serialized_transaction));
            } else {
                error!("Transaction serialization is bugged");
            }
        }

        // Iterate through all peers that are not the sender and excluded peers.
        for peer in self
            .connected_peers()
            .await
            .iter()
            .filter(|peer_ip| !self.is_local_ip(peer_ip) && !excluded_peers.contains(peer_ip))
        {
            self.handle_send(*peer, message.clone()).await;
        }
    }

    /// Sends the given message to every connected beacon, excluding the sender and any specified beacon IPs.
    async fn handle_propagate_beacon(&self, mut message: Message<N>, excluded_beacons: Vec<SocketAddr>) {
        // Perform ahead-of-time, non-blocking serialization just once for applicable objects.
        if let Message::UnconfirmedBlock(ref mut message) = message {
            if let Ok(serialized_block) = Data::serialize(message.block.clone()).await {
                let _ = std::mem::replace(&mut message.block, Data::Buffer(serialized_block));
            } else {
                error!("Block serialization is bugged");
            }
        } else if let Message::UnconfirmedSolution(ref mut message) = message {
            if let Ok(serialized_solution) = Data::serialize(message.solution.clone()).await {
                let _ = std::mem::replace(&mut message.solution, Data::Buffer(serialized_solution));
            } else {
                error!("Solution serialization is bugged");
            }
        } else if let Message::UnconfirmedTransaction(ref mut message) = message {
            if let Ok(serialized_transaction) = Data::serialize(message.transaction.clone()).await {
                let _ = std::mem::replace(&mut message.transaction, Data::Buffer(serialized_transaction));
            } else {
                error!("Transaction serialization is bugged");
            }
        }

        // Iterate through all beacons that are not the sender and excluded beacons.
        for peer in self
            .connected_beacons()
            .await
            .iter()
            .filter(|peer_ip| !self.is_local_ip(peer_ip) && !excluded_beacons.contains(peer_ip))
        {
            self.handle_send(*peer, message.clone()).await;
        }
    }

    /// Adds the given peer IPs to the set of candidate peers.
    ///
    /// This method skips adding any given peers if the combined size exceeds the threshold,
    /// as the peer providing this list could be subverting the protocol.
    async fn add_candidate_peers<'a, T: ExactSizeIterator<Item = &'a SocketAddr> + IntoIterator>(&self, peers: T) {
        // Acquire the candidate peers write lock.
        let mut candidate_peers = self.candidate_peers.write().await;
        // Ensure the combined number of peers does not surpass the threshold.
        for peer_ip in peers.take(Self::MAXIMUM_CANDIDATE_PEERS.saturating_sub(candidate_peers.len())) {
            // Ensure the peer is not itself, is not already connected, and is not restricted.
            if !self.is_local_ip(peer_ip)
                && !self.is_connected_to(*peer_ip).await
                && !self.is_restricted(*peer_ip).await
            {
                // Proceed to insert each new candidate peer IP.
                candidate_peers.insert(*peer_ip);
            }
        }
    }
}
