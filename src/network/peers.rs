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

use crate::{Environment, Message};
use snarkos_ledger::ledger::Ledger;
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

/// Shorthand for the parent half of the message channel.
type Outbound<N, E> = mpsc::Sender<Message<N, E>>;
/// Shorthand for the child half of the message channel.
type OutboundRouter<N, E> = mpsc::Receiver<Message<N, E>>;

///
/// A map of peers connected to the node server.
///
pub(crate) struct Peers<N: Network, E: Environment> {
    /// The local address of this node.
    local_ip: OnceCell<SocketAddr>,
    /// The set of connected peer IPs.
    connected_peers: HashMap<SocketAddr, Outbound<N, E>>,
    /// The set of candidate peer IPs.
    candidate_peers: HashSet<SocketAddr>,
}

impl<N: Network, E: Environment> Peers<N, E> {
    ///
    /// Initializes a new instance of `Peers`.
    ///
    pub(crate) fn new() -> Self {
        Self {
            local_ip: OnceCell::new(),
            connected_peers: HashMap::new(),
            candidate_peers: HashSet::new(),
        }
    }

    ///
    /// Returns the local IP address of the node.
    ///
    pub(crate) fn local_ip(&self) -> Result<SocketAddr> {
        match self.local_ip.get() {
            Some(local_ip) => Ok(*local_ip),
            None => return Err(anyhow!("Local IP is unknown")),
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
    /// Adds the given peer IPs to the set of candidate peers.
    ///
    /// This method skips adding any given peers if the combined size exceeds the threshold,
    /// as the peer providing this list could be subverting the protocol.
    ///
    pub(crate) fn add_candidate_peers(&mut self, peers: &[SocketAddr]) -> Result<()> {
        // The local IP address must be known by now.
        let local_ip = self.local_ip()?;

        // Ensure the combined number of peers does not surpass the threshold.
        if self.candidate_peers.len() + peers.len() < E::MAXIMUM_CANDIDATE_PEERS {
            // Proceed to insert each new candidate peer IP.
            for ip in peers.iter().take(E::MAXIMUM_CANDIDATE_PEERS) {
                // Ensure the peer is a new candidate.
                if *ip != local_ip && !self.connected_peers.contains_key(ip) && !self.candidate_peers.contains(ip) {
                    self.candidate_peers.insert(*ip);
                }
            }
        }
        Ok(())
    }

    ///
    /// Sends the given message to specified peer.
    ///
    pub(crate) async fn send(&mut self, peer: SocketAddr, message: &Message<N, E>) {
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
    pub(crate) async fn broadcast(&mut self, message: &Message<N, E>) {
        for peer in self.connected_peers() {
            self.send(peer, message).await;
        }
    }

    ///
    /// Sends the given message to every connected peer, except for the sender.
    ///
    pub(crate) async fn propagate(&mut self, sender: SocketAddr, message: &Message<N, E>) {
        for peer in self.connected_peers() {
            trace!("Preparing to propagate '{}' to {}", message.name(), peer);
            if peer != sender {
                self.send(peer, message).await;
            }
        }
    }

    ///
    /// Initiates a connection request to the given IP address.
    ///
    pub(crate) async fn listen(ledger: Arc<RwLock<Ledger<N>>>, peers: Arc<RwLock<Self>>, port: u16) -> Result<JoinHandle<()>> {
        let listener = TcpListener::bind(&format!("127.0.0.1:{}", port)).await?;

        // Update the local IP address of the node.
        let local_ip = listener.local_addr()?;
        peers.read().await.local_ip.set(local_ip).expect("Overwriting the local IP");

        // Initialize a process to maintain an adequate number of peers.
        let ledger_clone = ledger.clone();
        let peers_clone = peers.clone();
        task::spawn(async move {
            loop {
                // Sleep for 10 seconds.
                tokio::time::sleep(Duration::from_secs(10)).await;

                // Skip if the number of connected peers is above the minimum threshold.
                match peers_clone.read().await.num_connected_peers() < E::MINIMUM_NUMBER_OF_PEERS {
                    true => trace!("Attempting to find new peer connections"),
                    false => continue,
                };

                // Attempt to connect to more peers if the number of connected peers is below the minimum threshold.
                for peer_ip in peers_clone.read().await.candidate_peers().iter().take(E::MINIMUM_NUMBER_OF_PEERS) {
                    trace!("Attempting connection to {}...", peer_ip);
                    if let Err(error) = Peers::connect_to(ledger_clone.clone(), peers_clone.clone(), *peer_ip).await {
                        peers_clone.write().await.candidate_peers.remove(peer_ip);
                        trace!("Failed to connect to {}: {}", peer_ip, error);
                    }
                }

                // Request more peers if the number of connected peers is below the threshold.
                peers_clone.write().await.broadcast(&Message::PeerRequest).await;
            }
        });

        // Initialize the connection listener.
        debug!("Initializing the connection listener...");
        Ok(task::spawn(async move {
            info!("Listening for peers at {}", local_ip);
            loop {
                // Asynchronously wait for an inbound TcpStream.
                match listener.accept().await {
                    Ok((stream, remote_ip)) => {
                        // Process the inbound connection request.
                        Peers::spawn_handler(ledger.clone(), peers.clone(), remote_ip, stream).await;
                        // Add a small delay to avoid connecting above the limit.
                        tokio::time::sleep(Duration::from_millis(1)).await;
                    }
                    Err(error) => error!("Failed to accept a connection: {}", error),
                }
            }
        }))
    }

    ///
    /// Initiates a connection request to the given IP address.
    ///
    pub(crate) async fn connect_to(ledger: Arc<RwLock<Ledger<N>>>, peers: Arc<RwLock<Self>>, peer_ip: SocketAddr) -> Result<()> {
        // The local IP address must be known by now.
        let local_ip = peers.read().await.local_ip()?;

        // Ensure the remote IP is not this node.
        if peer_ip == local_ip || (peer_ip.ip().is_unspecified() || peer_ip.ip().is_loopback()) && peer_ip.port() == local_ip.port() {
            debug!("Skipping connection request to {} (attempted to self-connect)", peer_ip);
            Ok(())
        }
        // Ensure the node does not surpass the maximum number of peer connections.
        else if peers.read().await.num_connected_peers() >= E::MAXIMUM_NUMBER_OF_PEERS {
            debug!("Skipping connection request to {} (maximum peers reached)", peer_ip);
            Ok(())
        }
        // Ensure the peer is a new connection.
        else if peers.read().await.is_connected_to(peer_ip) {
            debug!("Skipping connection request to {} (already connected)", peer_ip);
            Ok(())
        }
        // Attempt to open a TCP stream.
        else {
            debug!("Connecting to {}...", peer_ip);
            let stream = match timeout(Duration::from_secs(E::CONNECTION_TIMEOUT_SECS), TcpStream::connect(peer_ip)).await {
                Ok(stream) => match stream {
                    Ok(stream) => stream,
                    Err(error) => return Err(anyhow!("Failed to connect to '{}': '{:?}'", peer_ip, error)),
                },
                Err(error) => return Err(anyhow!("Unable to reach '{}': '{:?}'", peer_ip, error)),
            };

            Self::spawn_handler(ledger.clone(), peers, peer_ip, stream).await;
            Ok(())
        }
    }

    ///
    /// Handles a new peer connection.
    ///
    async fn spawn_handler(ledger: Arc<RwLock<Ledger<N>>>, peers: Arc<RwLock<Self>>, peer_ip: SocketAddr, stream: TcpStream) {
        // Ensure the node does not surpass the maximum number of peer connections.
        if peers.read().await.num_connected_peers() >= E::MAXIMUM_NUMBER_OF_PEERS {
            debug!("Dropping connection request from {} (maximum peers reached)", peer_ip);
        }
        // Ensure the node is not already connected to this peer.
        else if peers.read().await.is_connected_to(peer_ip) {
            debug!("Dropping connection request from {} (already connected)", peer_ip);
        }
        // Spawn a handler to be run asynchronously.
        else {
            tokio::spawn(async move {
                debug!("Received a connection request from {}", peer_ip);
                if let Err(error) = Peer::handler(ledger.clone(), peers.clone(), stream).await {
                    trace!("{}", error);
                }
            });
        }
    }
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
    /// The TCP socket that handles sending and receiving data with this peer.
    socket: Framed<TcpStream, Message<N, E>>,
    /// The `router` half of the MPSC message channel, used to receive messages from peers.
    /// When a message is received on this `OutboundRouter`, it will be written to the socket.
    router: OutboundRouter<N, E>,
}

impl<N: Network, E: Environment> Peer<N, E> {
    /// Create a new instance of `Peer`.
    async fn new(peers: Arc<RwLock<Peers<N, E>>>, stream: TcpStream) -> Result<Self> {
        // Construct the socket.
        let mut socket = Framed::new(stream, Message::<N, E>::Pong);

        // The local IP address must be known by now.
        let local_ip = peers.read().await.local_ip()?;

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
                                Ok(stream) => {
                                    // Error if the stream is not open.
                                    if let Err(error) = stream {
                                        return Err(anyhow!("Failed to connect to '{}': '{:?}'", peer_ip, error));
                                    }
                                }
                                Err(error) => return Err(anyhow!("Unable to reach '{}': '{:?}'", peer_ip, error)),
                            };
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

        // Create a channel for this peer.
        let (outbound, router) = mpsc::channel(1024);

        // Add an entry for this `Peer` in the connected peers.
        peers.write().await.connected_peers.insert(peer_ip, outbound);
        // Remove an entry for this `Peer` in the candidate peers, if it exists.
        peers.write().await.candidate_peers.remove(&peer_ip);

        Ok(Peer {
            listener_ip: peer_ip,
            last_seen: Instant::now(),
            block_height: 0u32,
            socket,
            router,
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
        self.socket.flush().await?;
        Ok(())
    }

    /// A handler to process an individual peer.
    async fn handler(ledger: Arc<RwLock<Ledger<N>>>, peers: Arc<RwLock<Peers<N, E>>>, stream: TcpStream) -> Result<(), Box<dyn Error>> {
        // Register our peer with state which internally sets up some channels.
        let mut peer = Peer::new(peers.clone(), stream).await?;
        let peer_ip = peer.peer_ip();

        info!("Connected to {}", peer_ip);

        // Process incoming messages until this stream is disconnected.
        loop {
            tokio::select! {
                // Message channel is routing a message outbound to the peer.
                Some(message) = peer.router.recv() => {
                    // Disconnect if the peer has not communicated back in 4 minutes.
                    if peer.last_seen.elapsed() > Duration::from_secs(240) {
                        warn!("Peer {} has not communicated in {} seconds", peer_ip, peer.last_seen.elapsed().as_secs());
                        break;
                    } else {
                        trace!("Routing a message outbound to {}", peer_ip);
                        peer.send(message).await?;
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
                                if let Err(error) = peer.send(Message::PeerResponse(peers.read().await.connected_peers())).await {
                                    warn!("[PeerRequest] {}", error);
                                }
                            }
                            Message::PeerResponse(peer_ips) => {
                                // Add the given peer IPs to the list of candidate peers.
                                if let Err(error) = peers.write().await.add_candidate_peers(peer_ips) {
                                    warn!("[PeerResponse] {}", error);
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
                                }
                            },
                            Message::Pong => {
                                // Sleep for 10 seconds.
                                tokio::time::sleep(Duration::from_secs(10)).await;
                                // Send a `Ping` message.
                                if let Err(error) = peer.send(Message::Ping(E::MESSAGE_VERSION, 1)).await {
                                    warn!("[Pong] {}", error);
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

                                let latest_block_height = ledger.read().await.latest_block_height();
                                if *block_height == latest_block_height + 1 {
                                    ledger.write().await.add_next_block(block)?;
                                }

                                // Propagate the unconfirmed block to the connected peers.
                                peers.write().await.propagate(peer_ip, &message).await;
                            }
                            Message::UnconfirmedTransaction(_transaction) => {
                                // If peer is syncing, reject this message.

                                // TODO (howardwu) - Add to the ledger memory pool.
                                // Propagate the unconfirmed transaction to the connected peers.
                                peers.write().await.propagate(peer_ip, &message).await;
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
        peers.write().await.connected_peers.remove(&peer_ip);
        info!("Disconnecting from {}", peer_ip);

        Ok(())
    }
}
