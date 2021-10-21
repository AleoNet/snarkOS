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

use snarkos::{Miner, Node};

use snarkvm::{
    dpc::{prelude::*, testnet2::Testnet2},
    prelude::*,
};

use ::rand::thread_rng;
use anyhow::Result;
use tokio::{task};
use tracing_subscriber::EnvFilter;

use ::bytes::Bytes;
use anyhow::anyhow;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, Mutex};
use std::net::SocketAddr;
use std::collections::HashMap;
use tokio_util::codec::{Framed, BytesCodec};
use tokio_stream::StreamExt;
use futures::SinkExt;

pub fn initialize_logger() {
    let verbosity = 4;

    match verbosity {
        1 => std::env::set_var("RUST_LOG", "info"),
        2 => std::env::set_var("RUST_LOG", "debug"),
        3 | 4 => std::env::set_var("RUST_LOG", "trace"),
        _ => std::env::set_var("RUST_LOG", "info"),
    };

    // disable undesirable logs
    let filter = EnvFilter::from_default_env().add_directive("mio=off".parse().unwrap());

    // initialize tracing
    tracing_subscriber::fmt().with_env_filter(filter).with_target(verbosity == 4).init();
}

#[tokio::main]
async fn main() -> Result<()> {
    // let addr = env::args()
    //     .nth(1)
    //     .unwrap_or_else(|| "127.0.0.1:8080".to_string());
    //
    // let listener = TcpListener::bind(&addr).await?;
    // println!("Listening on: {}", addr);

    initialize_logger();
    tracing::trace!("Hello world");

    let account = Account::<Testnet2>::new(&mut thread_rng());

    // let node = Node::<Testnet2, Miner>::new()?;
    // node.start_listener().await?;
    // node.connect_to("144.126.212.176:4132".parse().unwrap()).await?;
    // node.start_miner(account.address());

    {
        // Create the shared state. This is how all the peers communicate.
        //
        // The server task will hold a handle to this. For every new client, the
        // `state` handle is cloned and passed into the task that processes the
        // client connection.
        let peers = Arc::new(Mutex::new(Peers::new()));

        let addr = env::args()
            .nth(1)
            .unwrap_or_else(|| "127.0.0.1:4132".to_string());

        // Bind a TCP listener to the socket address.
        //
        // Note that this is the Tokio TcpListener, which is fully async.
        let listener = TcpListener::bind(&addr).await?;

        tracing::info!("server running on {}", addr);

        loop {
            // Asynchronously wait for an inbound TcpStream.
            let (stream, ip) = listener.accept().await?;

            // Spawn a handler to be run asynchronously.
            let peers = peers.clone();
            tokio::spawn(async move {
                tracing::debug!("Received a connection from {}", ip);
                if let Err(e) = Peers::handler(peers, stream, ip).await {
                    tracing::info!("an error occurred; error = {:?}", e);
                }
            });
        }
    }

    std::future::pending::<()>().await;
    Ok(())
}

/// Shorthand for the transmit half of the message channel.
type Outbound = mpsc::Sender<Vec<u8>>;

/// Shorthand for the receive half of the message channel.
type Inbound = mpsc::Receiver<Vec<u8>>;

/// Data that is shared between all peers in the chat server.
///
/// This is the set of `Tx` handles for all connected clients. Whenever a
/// message is received from a client, it is broadcasted to all peers by
/// iterating over the `peers` entries and sending a copy of the message on each
/// `Tx`.
struct Peers {
    peers: HashMap<SocketAddr, Outbound>,
}

impl Peers {
    /// Create a new, empty, instance of `Shared`.
    fn new() -> Self {
        Peers {
            peers: HashMap::new(),
        }
    }

    /// Send a `LineCodec` encoded message to every peer, except
    /// for the sender.
    async fn broadcast(&mut self, sender: SocketAddr, message: &[u8]) {
        for peer in self.peers.iter_mut() {
            if *peer.0 != sender {
                let _ = peer.1.send(message.into());
            }
        }
    }

    /// Process an individual chat client
    async fn handler(
        peers: Arc<Mutex<Self>>,
        stream: TcpStream,
        ip: SocketAddr,
    ) -> Result<(), Box<dyn Error>> {
        // Register our peer with state which internally sets up some channels.
        let mut peer = Peer::new(peers.clone(), stream).await?;

        // // A client has connected, let's let everyone know.
        // {
        //     let mut peers = peers.lock().await;
        //     let msg = format!("{} has joined the chat", String::from_utf8_lossy(&username));
        //     tracing::info!("{}", msg);
        //     peers.broadcast(ip, msg.as_bytes()).await;
        // }

        // Process incoming messages until our stream is exhausted by a disconnect.
        loop {
            tokio::select! {
                // A message was received from a peer. Send it to the current user.
                Some(msg) = peer.inbound.recv() => {
                    tracing::info!("{}: {}", ip, String::from_utf8_lossy(&msg));
                    peer.lines.send(Bytes::from(msg)).await?;
                }
                result = peer.lines.next() => match result {
                    // A message was received from the current user, we should
                    // broadcast this message to the other users.
                    Some(Ok(msg)) => {
                        let mut peers = peers.lock().await;
                        let msg = format!("{}: {}", ip, String::from_utf8_lossy(&msg));
                        tracing::info!("{}", msg);
                        peers.broadcast(ip, msg.as_bytes()).await;
                    }
                    // An error occurred.
                    Some(Err(e)) => {
                        tracing::error!(
                            "an error occurred while processing messages for {}; error = {:?}",
                            ip,
                            e
                        );
                    }
                    // The stream has been exhausted.
                    None => {
                        tracing::error!("Exhausted");
                        break
                    },
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

/// The state for each connected client.
struct Peer {
    /// The TCP socket wrapped with the `Lines` codec, defined below.
    ///
    /// This handles sending and receiving data on the socket. When using
    /// `Lines`, we can work at the line level instead of having to manage the
    /// raw byte operations.
    lines: Framed<TcpStream, BytesCodec>,

    /// Receive half of the message channel.
    ///
    /// This is used to receive messages from peers. When a message is received
    /// off of this `Rx`, it will be written to the socket.
    inbound: Inbound,
}

impl Peer {
    /// Create a new instance of `Peer`.
    async fn new(
        peers: Arc<Mutex<Peers>>,
        stream: TcpStream,
    ) -> io::Result<Peer> {
        let mut lines = Framed::new(stream, BytesCodec::new());

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
        let ip = lines.get_ref().peer_addr()?;

        // Create a channel for this peer
        let (outbound, inbound) = mpsc::channel(1024);

        // Add an entry for this `Peer` in the peers.
        peers.lock().await.peers.insert(ip, outbound);

        Ok(Peer { lines, inbound })
    }
}

#[derive(Clone, Debug)]
pub enum Message {
    Ping(u32),
    Pong
}

impl Message {
    pub fn id(&self) -> u16 {
        match self {
            Self::Ping(..) => 0,
            Self::Pong => 1
        }
    }

    pub fn data(&self) -> Vec<u8> {
        match self {
            Self::Ping(nonce) => nonce.to_le_bytes().to_vec(),
            Self::Pong => vec![]
        }
    }

    pub fn serialize(&self) -> Result<Vec<u8>> {
        let buffer = [self.id().to_le_bytes().to_vec(), self.data()].concat();
        Ok(bincode::serialize(&buffer)?)
    }

    pub fn deserialize(buffer: &[u8]) -> Result<Self> {
        if buffer.len() < 2 {
            return Err(anyhow!("Invalid message"))
        }

        let id = u16::from_le_bytes([buffer[0], buffer[1]]);
        let data = &buffer[2..];

        match id {
            0 => Ok(Self::Ping(bincode::deserialize(data)?)),
            1 => match data.len() == 0 {
                true => Ok(Self::Pong),
                false => unreachable!()
            },
            _ => unreachable!()
        }
    }
}
