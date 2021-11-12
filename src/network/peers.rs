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

use crate::{Environment, LedgerRequest, LedgerRouter, Message};
use snarkvm::prelude::*;

use anyhow::{anyhow, Result};
use futures::SinkExt;
use std::{
    collections::{HashMap, HashSet},
    net::SocketAddr,
    time::{Duration, Instant},
};
use tokio::{net::TcpStream, sync::mpsc, task, time::timeout};
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

///
/// An enum of requests that the `Peers` struct processes.
///
#[derive(Debug)]
pub enum PeersRequest<N: Network, E: Environment> {
    /// Connect := (peer_ip, ledger_router)
    Connect(SocketAddr, LedgerRouter<N, E>),
    /// Heartbeat := (ledger_router)
    Heartbeat(LedgerRouter<N, E>),
    /// MessageBroadcast := (message)
    MessageBroadcast(Message<N, E>),
    /// MessagePropagate := (peer_ip, message)
    MessagePropagate(SocketAddr, Message<N, E>),
    /// MessageSend := (peer_ip, message)
    MessageSend(SocketAddr, Message<N, E>),
    /// PeerConnecting := (stream, peer_ip, ledger_router)
    PeerConnecting(TcpStream, SocketAddr, LedgerRouter<N, E>),
    /// PeerConnected := (peer_ip, outbound_router)
    PeerConnected(SocketAddr, OutboundRouter<N, E>),
    /// PeerDisconnected := (peer_ip)
    PeerDisconnected(SocketAddr),
    /// SendPeerResponse := (peer_ip)
    SendPeerResponse(SocketAddr),
    /// ReceivePeerResponse := (\[peer_ip\])
    ReceivePeerResponse(Vec<SocketAddr>),
}

///
/// A list of peers connected to the node server.
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
    pub(super) async fn update(&mut self, request: PeersRequest<N, E>, peers_router: &PeersRouter<N, E>) {
        match request {
            PeersRequest::Connect(peer_ip, ledger_router) => {
                // Ensure the peer IP is not this node.
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
                    match timeout(Duration::from_secs(E::CONNECTION_TIMEOUT_IN_SECS), TcpStream::connect(peer_ip)).await {
                        Ok(stream) => match stream {
                            Ok(stream) => Peer::handler(stream, self.local_ip, peers_router, ledger_router).await,
                            Err(error) => {
                                trace!("Failed to connect to '{}': '{:?}'", peer_ip, error);
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
            PeersRequest::Heartbeat(ledger_router) => {
                // Skip if the number of connected peers is above the minimum threshold.
                match self.num_connected_peers() < E::MINIMUM_NUMBER_OF_PEERS {
                    true => trace!("Sending request for more peer connections"),
                    false => return,
                };

                // Add the sync nodes to the list of candidate peers.
                let sync_nodes: Vec<SocketAddr> = E::SYNC_NODES.iter().map(|ip| ip.parse().unwrap()).collect();
                self.add_candidate_peers(&sync_nodes);

                // Add the peer nodes to the list of candidate peers.
                let peer_nodes: Vec<SocketAddr> = E::PEER_NODES.iter().map(|ip| ip.parse().unwrap()).collect();
                self.add_candidate_peers(&peer_nodes);

                // Attempt to connect to more peers if the number of connected peers is below the minimum threshold.
                for peer_ip in self.candidate_peers().iter().take(E::MINIMUM_NUMBER_OF_PEERS) {
                    trace!("Attempting connection to {}...", peer_ip);
                    let request = PeersRequest::Connect(*peer_ip, ledger_router.clone());
                    if let Err(error) = peers_router.send(request).await {
                        error!("Failed to transmit the request: '{}'", error);
                    }
                }
                // Request more peers if the number of connected peers is below the threshold.
                self.broadcast(&Message::PeerRequest).await;
            }
            PeersRequest::MessageBroadcast(message) => {
                self.broadcast(&message).await;
            }
            PeersRequest::MessagePropagate(sender, message) => {
                self.propagate(sender, &message).await;
            }
            PeersRequest::MessageSend(sender, message) => {
                self.send(sender, &message).await;
            }
            PeersRequest::PeerConnecting(stream, peer_ip, ledger_router) => {
                // Ensure the peer IP is not this node.
                if peer_ip == self.local_ip
                    || (peer_ip.ip().is_unspecified() || peer_ip.ip().is_loopback()) && peer_ip.port() == self.local_ip.port()
                {
                    debug!("Skipping connection request to {} (attempted to self-connect)", peer_ip);
                }
                // Ensure the node does not surpass the maximum number of peer connections.
                else if self.num_connected_peers() >= E::MAXIMUM_NUMBER_OF_PEERS {
                    debug!("Dropping connection request from {} (maximum peers reached)", peer_ip);
                }
                // Ensure the node is not already connected to this peer.
                else if self.is_connected_to(peer_ip) {
                    debug!("Dropping connection request from {} (already connected)", peer_ip);
                }
                // Spawn a handler to be run asynchronously.
                else {
                    debug!("Received a connection request from {}", peer_ip);
                    Peer::handler(stream, self.local_ip, peers_router, ledger_router).await;
                }
            }
            PeersRequest::PeerConnected(peer_ip, outbound) => {
                // Add an entry for this `Peer` in the connected peers.
                self.connected_peers.insert(peer_ip, outbound);
                // Remove an entry for this `Peer` in the candidate peers, if it exists.
                self.candidate_peers.remove(&peer_ip);
            }
            PeersRequest::PeerDisconnected(peer_ip) => {
                self.connected_peers.remove(&peer_ip);
                self.candidate_peers.insert(peer_ip);
            }
            PeersRequest::SendPeerResponse(recipient) => {
                // Send a `PeerResponse` message.
                self.send(recipient, &Message::PeerResponse(self.connected_peers())).await;
            }
            PeersRequest::ReceivePeerResponse(peer_ips) => {
                self.add_candidate_peers(&peer_ips);
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
            for peer_ip in peers.iter().take(E::MAXIMUM_CANDIDATE_PEERS) {
                // Ensure the peer is not self and is a new candidate peer.
                let is_self = *peer_ip == self.local_ip
                    || (peer_ip.ip().is_unspecified() || peer_ip.ip().is_loopback()) && peer_ip.port() == self.local_ip.port();
                if !is_self && !self.connected_peers.contains_key(peer_ip) && !self.candidate_peers.contains(peer_ip) {
                    self.candidate_peers.insert(*peer_ip);
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
                if let Err(error) = outbound.send(message.clone()).await {
                    trace!("Outbound channel failed: {}", error);
                    self.connected_peers.remove(&peer);
                }
            }
            None => warn!("Attempted to send to a non-connected peer {}", peer),
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
}

// TODO (howardwu): Consider changing this to a random challenge height.
//  The tradeoff is that checking genesis ensures your peer is starting at the same genesis block.
//  Choosing a random height also requires knowing upfront the height of the peer.
//  As such, leaving it at the genesis block height may be the best option here.
const CHALLENGE_HEIGHT: u32 = 0;

///
/// The state for each connected client.
///
struct Peer<N: Network, E: Environment> {
    /// The IP address of the peer, with the port set to the listener port.
    listener_ip: SocketAddr,
    /// The message version of the peer.
    version: u32,
    /// The timestamp of the last message received from this peer.
    last_seen: Instant,
    /// The TCP socket that handles sending and receiving data with this peer.
    outbound_socket: Framed<TcpStream, Message<N, E>>,
    /// The `outbound_handler` half of the MPSC message channel, used to receive messages from peers.
    /// When a message is received on this `OutboundHandler`, it will be written to the socket.
    outbound_handler: OutboundHandler<N, E>,
}

impl<N: Network, E: Environment> Peer<N, E> {
    /// Create a new instance of `Peer`.
    async fn new(stream: TcpStream, local_ip: SocketAddr, peers_router: &PeersRouter<N, E>) -> Result<Self> {
        // Construct the socket.
        let mut outbound_socket = Framed::new(stream, Message::<N, E>::PeerRequest);

        // Perform the handshake before proceeding.
        let peer_ip = Peer::handshake(&mut outbound_socket, local_ip).await?;

        // Create a channel for this peer.
        let (outbound_router, outbound_handler) = mpsc::channel(1024);

        // Add an entry for this `Peer` in the connected peers.
        peers_router.send(PeersRequest::PeerConnected(peer_ip, outbound_router)).await?;

        Ok(Peer {
            listener_ip: peer_ip,
            version: 0,
            last_seen: Instant::now(),
            outbound_socket,
            outbound_handler,
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

    /// Performs the handshake protocol, returning the listener IP of the peer upon success.
    async fn handshake(outbound_socket: &mut Framed<TcpStream, Message<N, E>>, local_ip: SocketAddr) -> Result<SocketAddr> {
        // Get the IP address of the peer.
        let mut peer_ip = outbound_socket.get_ref().peer_addr()?;

        // Retrieve the genesis block header.
        let genesis_block_header = N::genesis_block().header();

        // Send a challenge request to the peer.
        let message = Message::<N, E>::ChallengeRequest(local_ip.port(), CHALLENGE_HEIGHT);
        trace!("Sending '{}-A' to {}", message.name(), peer_ip);
        outbound_socket.send(message).await?;

        // Wait for the counterparty challenge request to come in.
        match outbound_socket.next().await {
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
                            let stream =
                                match timeout(Duration::from_secs(E::CONNECTION_TIMEOUT_IN_SECS), TcpStream::connect(peer_ip)).await {
                                    Ok(stream) => stream,
                                    Err(error) => return Err(anyhow!("Unable to reach '{}': '{:?}'", peer_ip, error)),
                                };
                            // Error if the stream is not open.
                            if let Err(error) = stream {
                                return Err(anyhow!("Unable to reach '{}': '{}'", peer_ip, error));
                            }
                        }
                        // Send the challenge response.
                        let message = Message::ChallengeResponse(genesis_block_header.clone());
                        trace!("Sending '{}-B' to {}", message.name(), peer_ip);
                        outbound_socket.send(message).await?;
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
        match outbound_socket.next().await {
            Some(Ok(message)) => {
                // Process the message.
                trace!("Received '{}-A' from {}", message.name(), peer_ip);
                match message {
                    Message::ChallengeResponse(block_header) => {
                        match block_header.height() == CHALLENGE_HEIGHT && &block_header == genesis_block_header && block_header.is_valid()
                        {
                            true => {
                                // Send the first ping sequence.
                                let message = Message::<N, E>::Ping(E::MESSAGE_VERSION);
                                trace!("Sending '{}' to {}", message.name(), peer_ip);
                                outbound_socket.send(message).await?;
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
    async fn handler(stream: TcpStream, local_ip: SocketAddr, peers_router: &PeersRouter<N, E>, ledger_router: LedgerRouter<N, E>) {
        let peers_router = peers_router.clone();
        task::spawn(async move {
            // Register our peer with state which internally sets up some channels.
            let mut peer = match Peer::new(stream, local_ip, &peers_router).await {
                Ok(peer) => peer,
                Err(error) => {
                    trace!("{}", error);
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
                    Some(message) = peer.outbound_handler.recv() => {
                        // Disconnect if the peer has not communicated back within the predefined time.
                        if peer.last_seen.elapsed() > Duration::from_secs(E::MAXIMUM_RADIO_SILENCE_IN_SECS) {
                            warn!("Peer {} has not communicated in {} seconds", peer_ip, peer.last_seen.elapsed().as_secs());
                            break;
                        } else {
                            // Route a message to the peer.
                            if let Err(error) = peer.send(message).await {
                                warn!("[OutboundRouter] {}", error);
                            }
                        }
                    }
                    result = peer.outbound_socket.next() => match result {
                        // Received a message from the peer.
                        Some(Ok(message)) => {
                            // Disconnect if the peer has not communicated back within the predefined time.
                            match peer.last_seen.elapsed() > Duration::from_secs(E::MAXIMUM_RADIO_SILENCE_IN_SECS) {
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
                                    // Route the `BlockRequest` to the ledger.
                                    if let Err(error) = ledger_router.send(LedgerRequest::BlockRequest(peer_ip, start_block_height, end_block_height)).await {
                                        warn!("[BlockRequest] {}", error);
                                    }
                                },
                                Message::BlockResponse(block) => {
                                    // Route the `BlockResponse` to the ledger.
                                    if let Err(error) = ledger_router.send(LedgerRequest::BlockResponse(peer_ip, block)).await {
                                        warn!("[BlockResponse] {}", error);
                                    }
                                }
                                Message::ChallengeRequest(..) | Message::ChallengeResponse(..) => {
                                    // Peer is not following the protocol.
                                    warn!("Peer {} is not following the protocol", peer_ip);
                                    break;
                                },
                                Message::Disconnect => {
                                    // Route the `Disconnect` to the ledger.
                                    if let Err(error) = ledger_router.send(LedgerRequest::Disconnect(peer_ip)).await {
                                        warn!("[Disconnect] {}", error);
                                    }
                                    break;
                                }
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
                                Message::Ping(version) => {
                                    // Ensure the message protocol version is not outdated.
                                    if version < E::MESSAGE_VERSION {
                                        warn!("Dropping {} on version {} (outdated)", peer_ip, version);
                                        break;
                                    }
                                    // Update the version of the peer.
                                    peer.version = version;
                                    // Route the `Ping` to the ledger.
                                    if let Err(error) = ledger_router.send(LedgerRequest::Ping(peer_ip)).await {
                                        warn!("[Ping] {}", error);
                                    }
                                },
                                Message::Pong(block_locators) => {
                                    // Route the `Pong` to the ledger.
                                    if let Err(error) = ledger_router.send(LedgerRequest::Pong(peer_ip, block_locators)).await {
                                        warn!("[Pong] {}", error);
                                    }
                                    // Sleep for the preset time before sending a `Ping` request.
                                    tokio::time::sleep(Duration::from_secs(E::PING_SLEEP_IN_SECS)).await;
                                    // Send a `Ping` request to the peer.
                                    let request = PeersRequest::MessageSend(peer_ip, Message::Ping(E::MESSAGE_VERSION));
                                    if let Err(error) = peers_router.send(request).await {
                                        warn!("[Ping] {}", error);
                                    }
                                }
                                Message::UnconfirmedBlock(block) => {
                                    // Route the `UnconfirmedBlock` to the ledger.
                                    if let Err(error) = ledger_router.send(LedgerRequest::UnconfirmedBlock(peer_ip, block)).await {
                                        warn!("[UnconfirmedBlock] {}", error);
                                    }
                                }
                                Message::UnconfirmedTransaction(transaction) => {
                                    // Route the `UnconfirmedTransaction` to the ledger.
                                    if let Err(error) = ledger_router.send(LedgerRequest::UnconfirmedTransaction(peer_ip, transaction)).await {
                                        warn!("[UnconfirmedTransaction] {}", error);
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
            if let Err(error) = ledger_router.send(LedgerRequest::Disconnect(peer_ip)).await {
                warn!("[Peer::Disconnect] {}", error);
            }
        });
    }
}
