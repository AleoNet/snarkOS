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

use crate::{Environment, NetworkError};
use snarkvm::prelude::*;

use ::bytes::Bytes;
use anyhow::{anyhow, Result};
use futures::SinkExt;
use once_cell::sync::OnceCell;
use std::{collections::HashMap, net::SocketAddr, sync::Arc, time::Duration};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::{mpsc, Mutex},
    task,
    task::JoinHandle,
    time::timeout,
};
use tokio_stream::StreamExt;
use tokio_util::codec::{BytesCodec, Framed};

/// Shorthand for the inbound half of the message channel.
type Inbound = mpsc::Receiver<Vec<u8>>;

/// Shorthand for the outbound half of the message channel.
type Outbound = mpsc::Sender<Vec<u8>>;

/// A map of peers connected to the node server.
pub(crate) struct Peers {
    peers: HashMap<SocketAddr, Outbound>,
    /// The local address of this node.
    local_ip: OnceCell<SocketAddr>,
}

impl Peers {
    /// Initializes a new instance of `Peers`.
    pub(crate) fn new() -> Self {
        Self {
            peers: HashMap::new(),
            local_ip: OnceCell::new(),
        }
    }

    /// Returns `true` if the node is connected to the given IP.
    pub(crate) fn is_connected_to(&self, ip: SocketAddr) -> bool {
        self.peers.contains_key(&ip)
    }

    /// Returns the number of connected peers.
    pub(crate) fn num_connected_peers(&self) -> usize {
        self.peers.len()
    }

    /// Sends the given message to every connected peer, except for the sender.
    pub(crate) async fn broadcast(&mut self, sender: SocketAddr, message: &[u8]) {
        for peer in self.peers.iter_mut() {
            if *peer.0 != sender {
                let _ = peer.1.send(message.into());
            }
        }
    }

    /// Initiates a connection request to the given IP address.
    pub(crate) async fn listen<E: Environment>(peers: Arc<Mutex<Self>>, port: &str) -> Result<JoinHandle<()>> {
        let listener = TcpListener::bind(&format!("127.0.0.1:{}", port)).await?;

        // Update the local IP address of the node.
        let discovered_local_ip = listener.local_addr()?;
        peers
            .lock()
            .await
            .local_ip
            .set(discovered_local_ip)
            .expect("The local IP address was set more than once!");

        info!("Initializing the listener...");
        Ok(task::spawn(async move {
            info!("Listening for peers at {}", discovered_local_ip);
            loop {
                // Asynchronously wait for an inbound TcpStream.
                match listener.accept().await {
                    Ok((stream, ip)) => {
                        // Process the inbound connection request.
                        Peers::process::<E>(peers.clone(), ip, stream).await;
                        // Add a small delay to avoid connecting above the limit.
                        tokio::time::sleep(Duration::from_millis(1)).await;
                    }
                    Err(error) => error!("Failed to accept a connection: {}", error),
                }
            }
        }))
    }

    /// Initiates a connection request to the given IP address.
    pub(crate) async fn connect<E: Environment>(peers: Arc<Mutex<Self>>, remote_ip: SocketAddr) -> Result<()> {
        debug!("Connecting to {}...", remote_ip);

        // The local IP address must be known by now.
        let local_ip = peers
            .lock()
            .await
            .local_ip
            .get()
            .copied()
            .expect("Local IP must be known in order to connect");

        // Ensure the remote IP is not this node.
        let is_self = (remote_ip.ip().is_unspecified() || remote_ip.ip().is_loopback()) && remote_ip.port() == local_ip.port();
        if remote_ip == local_ip || is_self {
            return Err(NetworkError::SelfConnectAttempt.into());
        }

        Self::process::<E>(
            peers,
            remote_ip,
            match timeout(Duration::from_secs(E::CONNECTION_TIMEOUT_SECS), TcpStream::connect(remote_ip)).await {
                Ok(stream) => match stream {
                    Ok(stream) => stream,
                    Err(error) => {
                        error!("Failed to connect to {}: {}", remote_ip, error); // self.set_connecting_failed();
                        return Err(anyhow!("Failed to send outgoing connection to '{}': '{:?}'", remote_ip, error));
                    }
                },
                Err(_) => {
                    error!("Unable to reach {}", remote_ip); // self.set_connecting_failed();
                    return Err(std::io::Error::new(std::io::ErrorKind::TimedOut, "connection timed out").into());
                }
            },
        )
        .await;
        Ok(())
    }

    /// Handles a new peer connection.
    async fn process<E: Environment>(peers: Arc<Mutex<Self>>, ip: SocketAddr, stream: TcpStream) {
        // Ensure the node does not surpass the maximum number of peer connections.
        if peers.lock().await.num_connected_peers() >= E::MAXIMUM_NUMBER_OF_PEERS {
            trace!("Dropping a connection request from {} (maximum peers reached)", ip);
        }
        // Ensure the node is not already connected to this peer.
        else if peers.lock().await.is_connected_to(ip) {
            trace!("Dropping a connection request from {} (peer is already connected)", ip);
        }
        // Spawn a handler to be run asynchronously.
        else {
            tokio::spawn(async move {
                debug!("Received a connection request from {}", ip);
                if let Err(error) = Peer::handler(peers, stream, ip).await {
                    error!("Failed to receive a connection from {}: {}", ip, error);
                }
            });
        }
    }
}

/// The state for each connected client.
struct Peer {
    /// The TCP socket that handles sending and receiving data with this peer.
    socket: Framed<TcpStream, BytesCodec>,

    /// The `inbound` half of the MPSC message channel.
    ///
    /// This is used to receive messages from peers. When a message is received
    /// off of this `Inbound`, it will be written to the socket.
    inbound: Inbound,
}

impl Peer {
    /// Create a new instance of `Peer`.
    async fn new(peers: Arc<Mutex<Peers>>, stream: TcpStream) -> io::Result<Self> {
        let mut socket = Framed::new(stream, BytesCodec::new());

        // // Send a prompt to the client to enter their username.
        // lines.send(Bytes::from(&b"Please enter your username:"[..])).await?;

        // // Read the first line from the `LineCodec` stream to get the username.
        // let username = match lines.next().await {
        //     Some(Ok(line)) => line,
        //     // We didn't get a line so we return early here.
        //     _ => {
        //         tracing::error!("Failed to get username from {}. Client disconnected.", ip);
        //         return Ok(());
        //     }
        // };

        // Get the IP address of the peer.
        let ip = socket.get_ref().peer_addr()?;
        // ip.set_port(E::NODE_PORT);

        // Create a channel for this peer
        let (outbound, inbound) = mpsc::channel(1024);

        // Add an entry for this `Peer` in the peers.
        peers.lock().await.peers.insert(ip, outbound);

        Ok(Peer { socket, inbound })
    }

    /// A handler to process an individual peer.
    async fn handler(peers: Arc<Mutex<Peers>>, stream: TcpStream, ip: SocketAddr) -> Result<(), Box<dyn Error>> {
        // Register our peer with state which internally sets up some channels.
        let mut peer = Peer::new(peers.clone(), stream).await?;

        // // A client has connected, let's let everyone know.
        // {
        //     let mut peers = peers.lock().await;
        //     let msg = format!("{} has joined the chat", String::from_utf8_lossy(&username));
        //     tracing::info!("{}", msg);
        //     peers.broadcast(ip, msg.as_bytes()).await;
        // }

        // Process incoming messages until this stream is disconnected.
        loop {
            tokio::select! {
                // A message was received from a peer. Send it to the current user.
                Some(msg) = peer.inbound.recv() => {
                    info!("First case - {}: {}", ip, String::from_utf8_lossy(&msg));
                    peer.socket.send(Bytes::from(msg)).await?;
                }
                result = peer.socket.next() => match result {
                    // A message was received from the current user, we should
                    // broadcast this message to the other users.
                    Some(Ok(msg)) => {
                        let mut peers = peers.lock().await;
                        let msg = format!("Second case - {}: {}", ip, String::from_utf8_lossy(&msg));
                        info!("{}", msg);
                        peers.broadcast(ip, msg.as_bytes()).await;
                    }
                    // An error occurred.
                    Some(Err(error)) => {
                        error!(
                            "Failed to process message from {}: {:?}",
                            ip,
                            error
                        );
                    }
                    // The stream has been disconnected.
                    None => break,
                },
            }
        }

        // When this is reached, it means the peer has disconnected.
        {
            let mut peers = peers.lock().await;
            peers.peers.remove(&ip);
            tracing::info!("{} has disconnected", ip);
        }

        Ok(())
    }
}

#[derive(Clone, Debug)]
pub enum Message {
    Ping(u32),
    Pong,
}

impl Message {
    pub fn id(&self) -> u16 {
        match self {
            Self::Ping(..) => 0,
            Self::Pong => 1,
        }
    }

    pub fn data(&self) -> Vec<u8> {
        match self {
            Self::Ping(nonce) => nonce.to_le_bytes().to_vec(),
            Self::Pong => vec![],
        }
    }

    pub fn serialize(&self) -> Result<Vec<u8>> {
        let buffer = [self.id().to_le_bytes().to_vec(), self.data()].concat();
        Ok(bincode::serialize(&buffer)?)
    }

    pub fn deserialize(buffer: &[u8]) -> Result<Self> {
        if buffer.len() < 2 {
            return Err(anyhow!("Invalid message"));
        }

        let id = u16::from_le_bytes([buffer[0], buffer[1]]);
        let data = &buffer[2..];

        match id {
            0 => Ok(Self::Ping(bincode::deserialize(data)?)),
            1 => match data.len() == 0 {
                true => Ok(Self::Pong),
                false => unreachable!(),
            },
            _ => unreachable!(),
        }
    }
}
