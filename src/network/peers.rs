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

use crate::{Environment, Message, NetworkError};
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
    sync::{mpsc, Mutex},
    task,
    task::JoinHandle,
    time::timeout,
};
use tokio_stream::StreamExt;
use tokio_util::codec::Framed;

/// Shorthand for the parent half of the message channel.
type Outbound<N> = mpsc::Sender<Message<N>>;

/// Shorthand for the child half of the message channel.
type Router<N> = mpsc::Receiver<Message<N>>;

/// A map of peers connected to the node server.
pub(crate) struct Peers<N: Network> {
    peers: HashMap<SocketAddr, Outbound<N>>,
    /// The local address of this node.
    local_ip: OnceCell<SocketAddr>,
    /// A set of candidate peer IPs.
    candidate_peers: HashSet<SocketAddr>,
}

impl<N: Network> Peers<N> {
    /// Initializes a new instance of `Peers`.
    pub(crate) fn new() -> Self {
        Self {
            peers: HashMap::new(),
            local_ip: OnceCell::new(),
            candidate_peers: HashSet::new(),
        }
    }

    /// Returns `true` if the node is connected to the given IP.
    pub(crate) fn is_connected_to(&self, ip: SocketAddr) -> bool {
        self.peers.contains_key(&ip)
    }

    /// Returns the list of connected peers.
    pub(crate) fn connected_peers(&self) -> Vec<SocketAddr> {
        self.peers.keys().cloned().collect()
    }

    /// Returns the number of connected peers.
    pub(crate) fn num_connected_peers(&self) -> usize {
        self.peers.len()
    }

    /// Adds the given peer IPs to the set of candidate peers.
    pub(crate) fn add_candidate_peers<E: Environment>(&mut self, peers: Vec<SocketAddr>) {
        for ip in peers.iter().take(E::MAXIMUM_CANDIDATE_PEERS) {
            if self.candidate_peers.len() < E::MAXIMUM_CANDIDATE_PEERS {
                // Ensure the peer is a new candidate.
                if !self.peers.contains_key(ip) && !self.candidate_peers.contains(ip) {
                    self.candidate_peers.insert(*ip);
                }
            }
        }
    }

    /// Returns the local IP address of the node.
    pub(crate) fn local_ip(&self) -> Result<SocketAddr> {
        match self.local_ip.get() {
            Some(local_ip) => Ok(*local_ip),
            None => return Err(anyhow!("Local IP is unknown")),
        }
    }

    /// Sends the given message to specified peer.
    pub(crate) async fn send(&mut self, peer: SocketAddr, message: &Message<N>) {
        match self.peers.get(&peer) {
            Some(outbound) => {
                debug!("Sending '{}' to {}", message.name(), peer);
                if let Err(error) = outbound.send(message.clone()).await {
                    error!("{}", error);
                    self.peers.remove(&peer);
                }
            }
            None => error!("Attempted to send to a non-connected peer {}", peer),
        }
    }

    /// Sends the given message to every connected peer.
    pub(crate) async fn broadcast(&mut self, message: &Message<N>) {
        for peer in self.connected_peers() {
            self.send(peer, message).await;
        }
    }

    /// Sends the given message to every connected peer, except for the sender.
    pub(crate) async fn propagate(&mut self, sender: SocketAddr, message: &Message<N>) {
        for peer in self.connected_peers() {
            trace!("Preparing to propagate '{}' to {}", message.name(), peer);
            if peer != sender {
                self.send(peer, message).await;
            }
        }
    }

    /// Initiates a connection request to the given IP address.
    pub(crate) async fn listen<E: Environment>(peers: Arc<Mutex<Self>>, port: u16) -> Result<JoinHandle<()>> {
        let listener = TcpListener::bind(&format!("127.0.0.1:{}", port)).await?;

        // Update the local IP address of the node.
        let discovered_local_ip = listener.local_addr()?;
        peers
            .lock()
            .await
            .local_ip
            .set(discovered_local_ip)
            .expect("The local IP address was set more than once!");

        debug!("Initializing the listener...");
        Ok(task::spawn(async move {
            info!("Listening for peers at {}", discovered_local_ip);
            loop {
                // Asynchronously wait for an inbound TcpStream.
                match listener.accept().await {
                    Ok((stream, remote_ip)) => {
                        // Process the inbound connection request.
                        Peers::process::<E>(peers.clone(), remote_ip, stream).await;
                        // Add a small delay to avoid connecting above the limit.
                        tokio::time::sleep(Duration::from_millis(1)).await;
                    }
                    Err(error) => error!("Failed to accept a connection: {}", error),
                }
            }
        }))
    }

    /// Initiates a connection request to the given IP address.
    pub(crate) async fn connect_to<E: Environment>(peers: Arc<Mutex<Self>>, peer_ip: SocketAddr) -> Result<()> {
        debug!("Connecting to {}...", peer_ip);

        // The local IP address must be known by now.
        let local_ip = peers.lock().await.local_ip()?;

        // Ensure the remote IP is not this node.
        let is_self = (peer_ip.ip().is_unspecified() || peer_ip.ip().is_loopback()) && peer_ip.port() == local_ip.port();
        if peer_ip == local_ip || is_self {
            return Err(NetworkError::SelfConnectAttempt.into());
        }

        // Attempt to open a TCP stream.
        let stream = match timeout(Duration::from_secs(E::CONNECTION_TIMEOUT_SECS), TcpStream::connect(peer_ip)).await {
            Ok(stream) => match stream {
                Ok(stream) => stream,
                Err(error) => return Err(anyhow!("Failed to connect to '{}': '{:?}'", peer_ip, error)),
            },
            Err(error) => return Err(anyhow!("Unable to reach '{}': '{:?}'", peer_ip, error)),
        };

        Self::process::<E>(peers, peer_ip, stream).await;
        Ok(())
    }

    /// Handles a new peer connection.
    async fn process<E: Environment>(peers: Arc<Mutex<Self>>, peer_ip: SocketAddr, stream: TcpStream) {
        // Ensure the node does not surpass the maximum number of peer connections.
        if peers.lock().await.num_connected_peers() >= E::MAXIMUM_NUMBER_OF_PEERS {
            trace!("Dropping a connection request from {} (maximum peers reached)", peer_ip);
        }
        // Ensure the node is not already connected to this peer.
        else if peers.lock().await.is_connected_to(peer_ip) {
            trace!("Dropping a connection request from {} (peer is already connected)", peer_ip);
        }
        // Spawn a handler to be run asynchronously.
        else {
            let peers_clone = peers.clone();
            tokio::spawn(async move {
                trace!("Received a connection request from {}", peer_ip);
                if let Err(error) = Peer::handler::<E>(peers_clone, stream).await {
                    trace!("{}", error);
                }
            });
        }
    }
}

// TODO (howardwu): Consider changing this.
const CHALLENGE_HEIGHT: u32 = 0;

/// The state for each connected client.
struct Peer<N: Network> {
    /// The IP address of the peer, with the port set to the listener port.
    ip: SocketAddr,
    /// The TCP socket that handles sending and receiving data with this peer.
    socket: Framed<TcpStream, Message<N>>,
    /// The `router` half of the MPSC message channel, used to receive messages from peers.
    /// When a message is received off of this `Router`, it will be written to the socket.
    router: Router<N>,
    /// The timestamp of the last message received from this peer.
    last_seen: Instant,
}

impl<N: Network> Peer<N> {
    /// Create a new instance of `Peer`.
    async fn new<E: Environment>(peers: Arc<Mutex<Peers<N>>>, stream: TcpStream) -> Result<Self> {
        // Construct the socket.
        let mut socket = Framed::new(stream, Message::<N>::Pong);

        // The local IP address must be known by now.
        let local_ip = peers.lock().await.local_ip()?;

        // Get the IP address of the peer.
        let mut peer_ip = socket.get_ref().peer_addr()?;

        // Retrieve the genesis block header.
        let genesis_block_header = N::genesis_block().header();

        // Send a challenge request to the peer.
        let message = Message::<N>::ChallengeRequest(local_ip.port(), CHALLENGE_HEIGHT);
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
                            match timeout(Duration::from_secs(E::CONNECTION_TIMEOUT_SECS), TcpStream::connect(peer_ip)).await {
                                Ok(stream) => {
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
                            "Expected a challenge request, received '{}' from {}",
                            message.name(),
                            peer_ip
                        ));
                    }
                }
            }
            // An error occurred.
            Some(Err(error)) => return Err(anyhow!("Failed to get challenge request from {}: {:?}", peer_ip, error)),
            // Did not receive anything.
            None => return Err(anyhow!("Dropped prior to the challenge request of {}", peer_ip)),
        };

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
                                let message = Message::<N>::Ping(0);
                                trace!("Sending '{}' to {}", message.name(), peer_ip);
                                socket.send(message).await?;
                            }
                            false => return Err(anyhow!("Challenge response from {} failed, received '{}'", peer_ip, block_header)),
                        }
                    }
                    message => {
                        return Err(anyhow!(
                            "Expected a challenge response, received '{}' from {}",
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
        };

        // Create a channel for this peer.
        let (outbound, router) = mpsc::channel(1024);

        // Add an entry for this `Peer` in the peers.
        peers.lock().await.peers.insert(peer_ip, outbound);

        Ok(Peer {
            ip: peer_ip,
            socket,
            router,
            last_seen: Instant::now(),
        })
    }

    /// Returns the IP address of the peer, with the port set to the listener port.
    fn ip(&self) -> SocketAddr {
        self.ip
    }

    async fn send(&mut self, message: Message<N>) -> Result<()> {
        trace!("Sending '{}' to {}", message.name(), self.socket.get_ref().peer_addr()?);
        self.socket.send(message).await?;
        Ok(())
    }

    /// A handler to process an individual peer.
    async fn handler<E: Environment>(peers: Arc<Mutex<Peers<N>>>, stream: TcpStream) -> Result<(), Box<dyn Error>> {
        // Register our peer with state which internally sets up some channels.
        let mut peer = Peer::new::<E>(peers.clone(), stream).await?;
        let peer_ip = peer.ip();

        info!("Connected to {}", peer_ip);

        let peers_clone = peers.clone();
        tokio::spawn(async move {
            loop {
                // Update the peers.
                peers_clone.lock().await.broadcast(&Message::PeerRequest).await;
                // Sleep for 120 seconds.
                tokio::time::sleep(Duration::from_secs(120)).await;
            }
        });

        // Process incoming messages until this stream is disconnected.
        loop {
            tokio::select! {
                // Message channel is routing a message outbound to the peer.
                Some(message) = peer.router.recv() => {
                    // Disconnect if the peer has not communicated back in 2 minutes.
                    if peer.last_seen.elapsed() > Duration::from_secs(120) {
                        break;
                    } else {
                        trace!("Routing a message outbound to {}", peer_ip);
                        peer.send(message).await?;
                    }
                }
                result = peer.socket.next() => match result {
                    // Received a message from the peer.
                    Some(Ok(message)) => {
                        // Update the last seen timestamp.
                        peer.last_seen = Instant::now();

                        // Process the message.
                        trace!("Received '{}' from {}", message.name(), peer_ip);
                        match message {
                            Message::ChallengeRequest(..) | Message::ChallengeResponse(..) => break, // Peer is not following the protocol.
                            Message::PeerRequest => {
                                peer.send(Message::PeerResponse(peers.lock().await.connected_peers())).await?;
                            }
                            Message::PeerResponse(peer_ips) => {
                                peers.lock().await.add_candidate_peers::<E>(peer_ips)
                            }
                            Message::Ping(block_height) => {
                                peer.send(Message::Pong).await?;
                            },
                            Message::Pong => {
                                // Sleep for 60 seconds.
                                tokio::time::sleep(Duration::from_secs(60)).await;
                                peer.send(Message::Ping(1)).await?;
                            }
                        }
                    }
                    // An error occurred.
                    Some(Err(error)) => {
                        error!(
                            "Failed to process message from {}: {:?}",
                            peer_ip,
                            error
                        );
                    }
                    // The stream has been disconnected.
                    None => break,
                },
            }
        }

        // When this is reached, it means the peer has disconnected.
        let mut peers = peers.lock().await;
        peers.peers.remove(&peer_ip);
        info!("Disconnecting from {}", peer_ip);

        Ok(())
    }
}
