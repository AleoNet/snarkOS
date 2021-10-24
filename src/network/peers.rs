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
    ledger::{Ledger, LedgerRouter},
    Environment,
    Message,
};
use snarkvm::prelude::*;

use anyhow::{anyhow, Result};
use futures::SinkExt;
use once_cell::sync::OnceCell;
use std::{
    collections::{HashMap, HashSet},
    net::SocketAddr,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::{mpsc, RwLock},
    task,
    task::JoinHandle,
    time::timeout,
};
use tokio_stream::StreamExt;
use tokio_util::codec::Framed;

#[derive(Debug)]
pub enum PeersRequest<N: Network, E: Environment> {
    AddCandidatePeers(Vec<SocketAddr>),
    AddConnectedPeer(SocketAddr, OutboundRouter<N, E>),
    RemoveCandidatePeer(SocketAddr),
    RemoveConnectedPeer(SocketAddr),
    Propagate(SocketAddr, Message<N, E>),
    Broadcast(Message<N, E>),
    SendPeerRequest(SocketAddr),
    SendPeerResponse(SocketAddr),
    HandleNewPeer(TcpStream, SocketAddr, PeersRouter<N, E>, LedgerRouter<N, E>),
    ConnectNewPeer(SocketAddr, PeersRouter<N, E>, LedgerRouter<N, E>),
    Heartbeat(PeersRouter<N, E>, LedgerRouter<N, E>),
}

/// Shorthand for the parent half of the message channel.
pub(crate) type PeersRouter<N, E> = mpsc::Sender<PeersRequest<N, E>>;
/// Shorthand for the child half of the message channel.
type PeersHandler<N, E> = mpsc::Receiver<PeersRequest<N, E>>;

/// Shorthand for the parent half of the message channel.
pub(crate) type OutboundRouter<N, E> = mpsc::Sender<Message<N, E>>;
/// Shorthand for the child half of the message channel.
type OutboundHandler<N, E> = mpsc::Receiver<Message<N, E>>;

///
/// A map of peers connected to the node server.
///
pub(crate) struct Peers<N: Network, E: Environment> {
    /// The local address of this node.
    local_ip: SocketAddr,
    /// The set of connected peer IPs.
    connected_peers: HashMap<SocketAddr, OutboundRouter<N, E>>,
    /// The set of candidate peer IPs.
    candidate_peers: HashSet<SocketAddr>,
}

impl<N: Network, E: Environment> Peers<N, E> {
    ///
    /// Initializes a new instance of `Peers`.
    ///
    pub(crate) fn new(local_ip: SocketAddr) -> Self {
        Self {
            local_ip,
            connected_peers: HashMap::new(),
            candidate_peers: HashSet::new(),
        }
    }

    ///
    /// Returns the local IP address of the node.
    ///
    pub(crate) fn local_ip(&self) -> SocketAddr {
        self.local_ip
    }

    ///
    /// Returns `true` if the node is connected to the given IP.
    ///
    pub(crate) fn is_connected_to(&self, ip: SocketAddr) -> bool {
        self.connected_peers.contains_key(&ip)
    }

    ///
    /// Returns the list of connected peers.
    ///
    pub(crate) fn connected_peers(&self) -> Vec<SocketAddr> {
        self.connected_peers.keys().cloned().collect()
    }

    ///
    /// Returns the list of candidate peers.
    ///
    pub(crate) fn candidate_peers(&self) -> &HashSet<SocketAddr> {
        &self.candidate_peers
    }

    ///
    /// Returns the number of connected peers.
    ///
    pub(crate) fn num_connected_peers(&self) -> usize {
        self.connected_peers.len()
    }

    ///
    /// Returns the number of candidate peers.
    ///
    pub(crate) fn num_candidate_peers(&self) -> usize {
        self.candidate_peers.len()
    }

    ///
    /// Performs the given `request` to the peers.
    /// All requests must go through this `update`, so that a unified view is preserved.
    ///
    pub(crate) async fn update(&mut self, request: PeersRequest<N, E>) {
        match request {
            PeersRequest::AddCandidatePeers(peer_ips) => {
                self.add_candidate_peers(&peer_ips);
            }
            PeersRequest::AddConnectedPeer(peer_ip, outbound) => {
                // Add an entry for this `Peer` in the connected peers.
                self.connected_peers.insert(peer_ip, outbound);
                // Remove an entry for this `Peer` in the candidate peers, if it exists.
                self.candidate_peers.remove(&peer_ip);
            }
            PeersRequest::RemoveCandidatePeer(peer_ip) => {
                self.candidate_peers.remove(&peer_ip);
            }
            PeersRequest::RemoveConnectedPeer(peer_ip) => {
                self.connected_peers.remove(&peer_ip);
                self.candidate_peers.insert(peer_ip);
            }
            PeersRequest::Propagate(sender, message) => {
                self.propagate(sender, &message).await;
            }
            PeersRequest::Broadcast(message) => {
                self.broadcast(&message).await;
            }
            PeersRequest::SendPeerRequest(recipient) => {
                // Send a `PeerResponse` message.
                self.send(recipient, &Message::PeerRequest).await;
            }
            PeersRequest::SendPeerResponse(recipient) => {
                // Send a `PeerResponse` message.
                self.send(recipient, &Message::PeerResponse(self.connected_peers())).await;
            }
            PeersRequest::HandleNewPeer(stream, peer_ip, peers_router, ledger_router) => {
                // Ensure the node does not surpass the maximum number of peer connections.
                if self.num_connected_peers() >= E::MAXIMUM_NUMBER_OF_PEERS {
                    debug!("Dropping connection request from {} (maximum peers reached)", peer_ip);
                }
                // Ensure the node is not already connected to this peer.
                else if self.is_connected_to(peer_ip) {
                    debug!("Dropping connection request from {} (already connected)", peer_ip);
                }
                // Spawn a handler to be run asynchronously.
                else {
                    debug!("Received a connection request from {}", peer_ip);
                    Peer::handler(stream, self.local_ip, peers_router).await;
                }
            }

            PeersRequest::ConnectNewPeer(peer_ip, peers_router, ledger_router) => {
                // Ensure the remote IP is not this node.
                if peer_ip == self.local_ip
                    || (peer_ip.ip().is_unspecified() || peer_ip.ip().is_loopback()) && peer_ip.port() == self.local_ip.port()
                {
                    debug!("Skipping connection request to {} (attempted to self-connect)", peer_ip);
                }
                // Ensure the node does not surpass the maximum number of peer connections.
                else if self.num_connected_peers() >= E::MAXIMUM_NUMBER_OF_PEERS {
                    debug!("Skipping connection request to {} (maximum peers reached)", peer_ip);
                }
                // Ensure the peer is a new connection.
                else if self.is_connected_to(peer_ip) {
                    debug!("Skipping connection request to {} (already connected)", peer_ip);
                }
                // Attempt to open a TCP stream.
                else {
                    debug!("Connecting to {}...", peer_ip);
                    match timeout(Duration::from_secs(E::CONNECTION_TIMEOUT_SECS), TcpStream::connect(peer_ip)).await {
                        Ok(stream) => match stream {
                            Ok(stream) => Peer::handler(stream, self.local_ip, peers_router).await,
                            Err(error) => {
                                error!("Failed to connect to '{}': '{:?}'", peer_ip, error);
                                self.candidate_peers.remove(&peer_ip);
                            }
                        },
                        Err(error) => {
                            error!("Unable to reach '{}': '{:?}'", peer_ip, error);
                            self.candidate_peers.remove(&peer_ip);
                        }
                    };
                }
            }
            PeersRequest::Heartbeat(peers_router, ledger_router) => {
                // Skip if the number of connected peers is above the minimum threshold.
                match self.num_connected_peers() < E::MINIMUM_NUMBER_OF_PEERS {
                    true => trace!("Attempting to discover new peers"),
                    false => return,
                };

                // Attempt to connect to more peers if the number of connected peers is below the minimum threshold.
                for peer_ip in self.candidate_peers().iter().take(E::MINIMUM_NUMBER_OF_PEERS) {
                    trace!("Attempting connection to {}...", peer_ip);
                    if let Err(error) = peers_router
                        .send(PeersRequest::ConnectNewPeer(*peer_ip, peers_router.clone(), ledger_router.clone()))
                        .await
                    {
                        error!("Failed to transmit the request: '{}'", error);
                    }
                }

                // Request more peers if the number of connected peers is below the threshold.
                self.broadcast(&Message::PeerRequest).await;
            }
        }
    }

    ///
    /// Adds the given peer IPs to the set of candidate peers.
    ///
    /// This method skips adding any given peers if the combined size exceeds the threshold,
    /// as the peer providing this list could be subverting the protocol.
    ///
    fn add_candidate_peers(&mut self, peers: &[SocketAddr]) {
        // Ensure the combined number of peers does not surpass the threshold.
        if self.candidate_peers.len() + peers.len() < E::MAXIMUM_CANDIDATE_PEERS {
            // Proceed to insert each new candidate peer IP.
            for ip in peers.iter().take(E::MAXIMUM_CANDIDATE_PEERS) {
                // Ensure the peer is a new candidate.
                if *ip != self.local_ip() && !self.connected_peers.contains_key(ip) && !self.candidate_peers.contains(ip) {
                    self.candidate_peers.insert(*ip);
                }
            }
        }
    }

    ///
    /// Sends the given message to specified peer.
    ///
    async fn send(&mut self, peer: SocketAddr, message: &Message<N, E>) {
        match self.connected_peers.get(&peer) {
            Some(outbound) => {
                trace!("Sending '{}' to {}", message.name(), peer);
                if let Err(error) = outbound.send(message.clone()).await {
                    error!("{}", error);
                    self.connected_peers.remove(&peer);
                }
            }
            None => error!("Attempted to send to a non-connected peer {}", peer),
        }
    }

    ///
    /// Sends the given message to every connected peer.
    ///
    async fn broadcast(&mut self, message: &Message<N, E>) {
        for peer in self.connected_peers() {
            self.send(peer, message).await;
        }
    }

    ///
    /// Sends the given message to every connected peer, except for the sender.
    ///
    async fn propagate(&mut self, sender: SocketAddr, message: &Message<N, E>) {
        for peer in self.connected_peers() {
            trace!("Preparing to propagate '{}' to {}", message.name(), peer);
            if peer != sender {
                self.send(peer, message).await;
            }
        }
    }

    // ///
    // /// Starts the connection listener for peers.
    // ///
    // pub(crate) async fn listen(ledger: Arc<RwLock<Ledger<N>>>, peers: Arc<RwLock<Self>>, port: u16) -> Result<JoinHandle<()>> {
    //     let listener = TcpListener::bind(&format!("127.0.0.1:{}", port)).await?;
    //
    //     // Update the local IP address of the node.
    //     let local_ip = listener.local_addr()?;
    //
    //     // Initialize a process to maintain an adequate number of peers.
    //     let ledger_clone = ledger.clone();
    //     let peers_clone = peers.clone();
    //     task::spawn(async move {
    //         loop {
    //             // Sleep for 30 seconds.
    //             tokio::time::sleep(Duration::from_secs(30)).await;
    //
    //             // Skip if the number of connected peers is above the minimum threshold.
    //             match peers_clone.read().await.num_connected_peers() < E::MINIMUM_NUMBER_OF_PEERS {
    //                 true => trace!("Attempting to find new peer connections"),
    //                 false => continue,
    //             };
    //
    //             // Attempt to connect to more peers if the number of connected peers is below the minimum threshold.
    //             for peer_ip in peers_clone.read().await.candidate_peers().iter().take(E::MINIMUM_NUMBER_OF_PEERS) {
    //                 trace!("Attempting connection to {}...", peer_ip);
    //                 if let Err(error) = Peers::connect_to(ledger_clone.clone(), peers_clone.clone(), *peer_ip).await {
    //                     peers_clone.write().await.candidate_peers.remove(peer_ip);
    //                     trace!("Failed to connect to {}: {}", peer_ip, error);
    //                 }
    //             }
    //
    //             // Request more peers if the number of connected peers is below the threshold.
    //             peers_clone.write().await.broadcast(&Message::PeerRequest).await;
    //         }
    //     });
    //
    //     // Initialize the connection listener.
    //     debug!("Initializing the connection listener...");
    //     Ok(task::spawn(async move {
    //         info!("Listening for peers at {}", local_ip);
    //         loop {
    //             // Asynchronously wait for an inbound TcpStream.
    //             match listener.accept().await {
    //                 Ok((stream, remote_ip)) => {
    //                     // Process the inbound connection request.
    //                     Peers::spawn_handler(ledger.clone(), peers.clone(), remote_ip, stream).await;
    //                     // Add a small delay to avoid connecting above the limit.
    //                     tokio::time::sleep(Duration::from_millis(1)).await;
    //                 }
    //                 Err(error) => error!("Failed to accept a connection: {}", error),
    //             }
    //         }
    //     }))
    // }

    // ///
    // /// Initiates a connection request to the given IP address.
    // ///
    // pub(crate) async fn connect_to(peers: Arc<RwLock<Self>>, peer_ip: SocketAddr) -> Result<()> {
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
    //         Self::spawn_handler(peers, peer_ip, stream).await;
    //         Ok(())
    //     }
    // }

    // ///
    // /// Handles a new peer connection.
    // ///
    // async fn spawn_handler(peers: Arc<RwLock<Self>>, peer_ip: SocketAddr, stream: TcpStream) {
    //     // Ensure the node does not surpass the maximum number of peer connections.
    //     if peers.read().await.num_connected_peers() >= E::MAXIMUM_NUMBER_OF_PEERS {
    //         debug!("Dropping connection request from {} (maximum peers reached)", peer_ip);
    //     }
    //     // Ensure the node is not already connected to this peer.
    //     else if peers.read().await.is_connected_to(peer_ip) {
    //         debug!("Dropping connection request from {} (already connected)", peer_ip);
    //     }
    //     // Spawn a handler to be run asynchronously.
    //     else {
    //         tokio::spawn(async move {
    //             debug!("Received a connection request from {}", peer_ip);
    //             if let Err(error) = Peer::handler(peers.clone(), stream).await {
    //                 trace!("{}", error);
    //             }
    //         });
    //     }
    // }
}

// TODO (howardwu): Consider changing this.
const CHALLENGE_HEIGHT: u32 = 0;

///
/// The state for each connected client.
///
struct Peer<N: Network, E: Environment> {
    /// The IP address of the peer, with the port set to the listener port.
    listener_ip: SocketAddr,
    /// The timestamp of the last message received from this peer.
    last_seen: Instant,
    /// The block height of this peer.
    block_height: u32,
    /// The number of failures.
    failures: u64,
    /// The TCP socket that handles sending and receiving data with this peer.
    socket: Framed<TcpStream, Message<N, E>>,
    /// The `handler` half of the MPSC message channel, used to receive messages from peers.
    /// When a message is received on this `OutboundHandler`, it will be written to the socket.
    handler: OutboundHandler<N, E>,
}

impl<N: Network, E: Environment> Peer<N, E> {
    /// Create a new instance of `Peer`.
    async fn new(stream: TcpStream, local_ip: SocketAddr, peers_handler: PeersRouter<N, E>) -> Result<Self> {
        // Construct the socket.
        let mut socket = Framed::new(stream, Message::<N, E>::Pong);

        // Perform the handshake before proceeding.
        let peer_ip = Peer::handshake(&mut socket, local_ip).await?;

        // Create a channel for this peer.
        let (outbound_router, handler) = mpsc::channel(1024);

        // Add an entry for this `Peer` in the connected peers.
        peers_handler.send(PeersRequest::AddConnectedPeer(peer_ip, outbound_router)).await?;

        Ok(Peer {
            listener_ip: peer_ip,
            last_seen: Instant::now(),
            block_height: 0u32,
            failures: 0u64,
            socket,
            handler,
        })
    }

    /// Returns the IP address of the peer, with the port set to the listener port.
    fn peer_ip(&self) -> SocketAddr {
        self.listener_ip
    }

    /// Sends the given message to this peer.
    async fn send(&mut self, message: Message<N, E>) -> Result<()> {
        trace!("Sending '{}' to {}", message.name(), self.socket.get_ref().peer_addr()?);
        self.socket.send(message).await?;
        // self.socket.flush().await?;
        Ok(())
    }

    /// Performs the handshake protocol, returning the listener IP of the peer upon success.
    async fn handshake(socket: &mut Framed<TcpStream, Message<N, E>>, local_ip: SocketAddr) -> Result<SocketAddr> {
        // Get the IP address of the peer.
        let mut peer_ip = socket.get_ref().peer_addr()?;

        // Retrieve the genesis block header.
        let genesis_block_header = N::genesis_block().header();

        // Send a challenge request to the peer.
        let message = Message::<N, E>::ChallengeRequest(local_ip.port(), CHALLENGE_HEIGHT);
        trace!("Sending '{}-A' to {}", message.name(), peer_ip);
        socket.send(message).await?;

        // Wait for the counterparty challenge request to come in.
        match socket.next().await {
            Some(Ok(message)) => {
                // Process the message.
                trace!("Received '{}-B' from {}", message.name(), peer_ip);
                match message {
                    Message::ChallengeRequest(listener_port, _block_height) => {
                        // Verify the listener port.
                        if peer_ip.port() != listener_port {
                            // Update the peer IP to the listener port.
                            peer_ip.set_port(listener_port);
                            // Ensure the claimed listener port is open.
                            let stream = match timeout(Duration::from_secs(E::CONNECTION_TIMEOUT_SECS), TcpStream::connect(peer_ip)).await {
                                Ok(stream) => stream,
                                Err(error) => return Err(anyhow!("Unable to reach '{}': '{:?}'", peer_ip, error)),
                            };
                            // Error if the stream is not open.
                            if let Err(error) = stream {
                                return Err(anyhow!("Failed to connect to '{}': '{:?}'", peer_ip, error));
                            }
                        }
                        // Send the challenge response.
                        let message = Message::ChallengeResponse(genesis_block_header.clone());
                        trace!("Sending '{}-B' to {}", message.name(), peer_ip);
                        socket.send(message).await?;
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
        }

        // Wait for the challenge response to come in.
        match socket.next().await {
            Some(Ok(message)) => {
                // Process the message.
                trace!("Received '{}-A' from {}", message.name(), peer_ip);
                match message {
                    Message::ChallengeResponse(block_header) => {
                        match block_header.height() == CHALLENGE_HEIGHT && &block_header == genesis_block_header && block_header.is_valid()
                        {
                            true => {
                                // Send the first ping sequence.
                                let message = Message::<N, E>::Ping(E::MESSAGE_VERSION, 0);
                                trace!("Sending '{}' to {}", message.name(), peer_ip);
                                socket.send(message).await?;
                                Ok(peer_ip)
                            }
                            false => return Err(anyhow!("Challenge response from {} failed, received '{}'", peer_ip, block_header)),
                        }
                    }
                    message => {
                        return Err(anyhow!(
                            "Expected challenge response, received '{}' from {}",
                            message.name(),
                            peer_ip
                        ));
                    }
                }
            }
            // An error occurred.
            Some(Err(error)) => return Err(anyhow!("Failed to get challenge response from {}: {:?}", peer_ip, error)),
            // Did not receive anything.
            None => return Err(anyhow!("Failed to get challenge response from {}, peer has disconnected", peer_ip)),
        }
    }

    /// A handler to process an individual peer.
    async fn handler(stream: TcpStream, local_ip: SocketAddr, peers_router: PeersRouter<N, E>) {
        task::spawn(async move {
            // Register our peer with state which internally sets up some channels.
            let mut peer = match Peer::new(stream, local_ip, peers_router.clone()).await {
                Ok(mut peer) => peer,
                Err(error) => return,
            };

            // Retrieve the peer IP.
            let peer_ip = peer.peer_ip();
            info!("Connected to {}", peer_ip);

            // Process incoming messages until this stream is disconnected.
            loop {
                tokio::select! {
                    // Message channel is routing a message outbound to the peer.
                    Some(message) = peer.handler.recv() => {
                        // Disconnect if the peer has not communicated back in 4 minutes.
                        if peer.last_seen.elapsed() > Duration::from_secs(240) {
                            warn!("Peer {} has not communicated in {} seconds", peer_ip, peer.last_seen.elapsed().as_secs());
                            break;
                        } else {
                            trace!("Routing a message outbound to {}", peer_ip);
                            if let Err(error) = peer.send(message).await {
                                warn!("[OutboundRouter] {}", error);
                                peer.failures += 1;
                            }
                        }
                    }
                    result = peer.socket.next() => match result {
                        // Received a message from the peer.
                        Some(Ok(message)) => {
                            // Disconnect if the peer has not communicated back in 4 minutes.
                            match peer.last_seen.elapsed() > Duration::from_secs(240) {
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
                            match &message {
                                Message::ChallengeRequest(..) | Message::ChallengeResponse(..) => {
                                    // Peer is not following the protocol.
                                    warn!("Peer {} is not following the protocol", peer_ip);
                                    break;
                                },
                                Message::PeerRequest => {
                                    // Send a `PeerResponse` message.
                                    if let Err(error) = peers_router.send(PeersRequest::SendPeerResponse(peer_ip)).await {
                                        warn!("[PeerRequest] {}", error);
                                        peer.failures += 1;
                                    }
                                }
                                Message::PeerResponse(peer_ips) => {
                                    // Add the given peer IPs to the list of candidate peers.
                                    if let Err(error) = peers_router.send(PeersRequest::AddCandidatePeers(peer_ips.to_vec())).await {
                                        warn!("[PeerResponse] {}", error);
                                        peer.failures += 1;
                                    }
                                }
                                Message::Ping(version, block_height) => {
                                    // Ensure the message protocol version is not outdated.
                                    if *version < E::MESSAGE_VERSION {
                                        warn!("Dropping {} with outdated version {}", peer_ip, version);
                                        break;
                                    }
                                    // Set the block height of the peer.
                                    peer.block_height = *block_height;
                                    // Send a `Pong` message.
                                    if let Err(error) = peer.send(Message::Pong).await {
                                        warn!("[PeerRequest] {}", error);
                                        peer.failures += 1;
                                    }
                                },
                                Message::Pong => {
                                    // Sleep for 10 seconds.
                                    tokio::time::sleep(Duration::from_secs(10)).await;
                                    // Send a `Ping` message.
                                    if let Err(error) = peer.send(Message::Ping(E::MESSAGE_VERSION, 1)).await {
                                        warn!("[Pong] {}", error);
                                        peer.failures += 1;
                                    }
                                },
                                Message::RebaseRequest(_block_headers) => {
                                    // TODO (howardwu) - Process the rebase request.
                                    // If peer is syncing, reject this message.
                                },
                                Message::RebaseResponse => {
                                    // TODO (howardwu) - Add logic for this.
                                    // If peer is syncing, reject this message.
                                }
                                Message::SyncRequest(block_height) => {
                                    // TODO (howardwu) - Send a block back.
                                    // peer.send(Message::SyncResponse(block_height, )).await?;
                                },
                                Message::SyncResponse(_block_height, _block) => {
                                    // TODO (howardwu) - Add to the ledger.
                                }
                                Message::UnconfirmedBlock(block_height, block) => {
                                    // TODO (howardwu) - Add to the ledger memory pool.
                                    // If peer is syncing, reject this message.
                                    info!("Received an unconfirmed block {} from {}", block_height, peer_ip);

                                    // let latest_block_height = ledger.read().await.latest_block_height();
                                    // if *block_height == latest_block_height + 1 {
                                    //     ledger.write().await.add_next_block(block)?;
                                    // }

                                    // Propagate the unconfirmed block to the connected peers.
                                    if let Err(error) = peers_router.send(PeersRequest::Propagate(peer_ip, message)).await {
                                        warn!("[UnconfirmedBlock] {}", error);
                                        peer.failures += 1;
                                    }
                                }
                                Message::UnconfirmedTransaction(_transaction) => {
                                    // If peer is syncing, reject this message.

                                    // TODO (howardwu) - Add to the ledger memory pool.
                                    // Propagate the unconfirmed transaction to the connected peers.
                                    if let Err(error) = peers_router.send(PeersRequest::Propagate(peer_ip, message)).await {
                                        warn!("[UnconfirmedTransaction] {}", error);
                                        peer.failures += 1;
                                    }
                                }
                                Message::Unused(_) => {
                                    // Peer is not following the protocol.
                                    break;
                                }
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
            info!("Disconnecting from {}", peer_ip);
            if let Err(error) = peers_router.send(PeersRequest::RemoveConnectedPeer(peer_ip)).await {
                error!("Failed to disconnect from {}: {}", peer_ip, error);
            }
        });
    }
}
