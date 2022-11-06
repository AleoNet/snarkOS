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

use crate::{Message, MessageCodec, Router, RouterRequest};
use snarkos_node_executor::{spawn_task_loop, Executor, NodeType, Status};
use snarkvm::prelude::*;

use anyhow::Result;
use core::time::Duration;
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::Arc,
    time::{Instant, SystemTime},
};
use tokio::{
    net::TcpStream,
    sync::{mpsc, mpsc::error::SendError, RwLock},
};
use tokio_util::codec::Framed;

/// Shorthand for the parent half of the `Peer` outbound message channel.
pub(crate) type PeerSender<N> = mpsc::Sender<Message<N>>;
/// Shorthand for the child half of the `Peer` outbound message channel.
pub(crate) type PeerHandler<N> = mpsc::Receiver<Message<N>>;

/// The state for each connected peer.
#[derive(Clone, Debug)]
pub struct Peer<N: Network> {
    /// The IP address of the peer, with the port set to the listener port.
    ip: SocketAddr,
    /// The message version of the peer.
    pub version: Arc<RwLock<u32>>,
    /// The node type of the peer.
    pub node_type: Arc<RwLock<NodeType>>,
    /// The node type of the peer.
    pub status: Arc<RwLock<Status>>,
    /// The block height of the peer.
    pub block_height: Arc<RwLock<u32>>,
    /// The timestamp of the last message received from this peer.
    pub last_seen: Arc<RwLock<Instant>>,
    /// The map of (message ID, random nonce) pairs to their last seen timestamp.
    pub seen_messages: Arc<RwLock<HashMap<(u16, u32), SystemTime>>>,
    /// The sender channel to the peer.
    peer_sender: PeerSender<N>,
}

impl<N: Network> Peer<N> {
    /// Initializes a new instance of `Peer`.
    pub async fn initialize<E: Executor>(
        ip: SocketAddr,
        node_type: NodeType,
        status: Status,
        router: Router<N>,
        outbound_socket: Framed<TcpStream, MessageCodec<N>>,
    ) -> Result<Self> {
        // Initialize an MPSC channel for sending requests to the `Peer` struct.
        let (peer_sender, peer_handler) = mpsc::channel(1024);

        // Construct the peer.
        let peer = Peer {
            ip,
            version: Arc::new(RwLock::new(0)),
            node_type: Arc::new(RwLock::new(node_type)),
            status: Arc::new(RwLock::new(status)),
            block_height: Arc::new(RwLock::new(0)),
            last_seen: Arc::new(RwLock::new(Instant::now())),
            seen_messages: Default::default(),
            peer_sender,
        };

        // Initialize the garbage collector for the peer.
        peer.initialize_gc::<E>().await;

        // Add an entry for this `Peer` in the connected peers.
        match router.process(RouterRequest::PeerConnected(peer.clone(), outbound_socket, peer_handler)).await {
            // Return the peer.
            Ok(_) => Ok(peer),
            Err(error) => bail!("[PeerConnected] {error}"),
        }
    }

    /// Returns the IP address of the peer, with the port set to the listener port.
    pub fn ip(&self) -> &SocketAddr {
        &self.ip
    }

    /// Returns the node type.
    pub async fn node_type(&self) -> NodeType {
        *self.node_type.read().await
    }

    /// Returns `true` if the peer is a beacon.
    pub async fn is_beacon(&self) -> bool {
        self.node_type().await.is_beacon()
    }

    /// Returns `true` if the peer is a validator.
    pub async fn is_validator(&self) -> bool {
        self.node_type().await.is_validator()
    }

    /// Returns `true` if the peer is a prover.
    pub async fn is_prover(&self) -> bool {
        self.node_type().await.is_prover()
    }

    /// Returns `true` if the peer is a client.
    pub async fn is_client(&self) -> bool {
        self.node_type().await.is_client()
    }

    /// Sends the given message to this peer.
    pub async fn send(&self, message: Message<N>) -> Result<(), SendError<Message<N>>> {
        self.peer_sender.send(message).await
    }

    /// Initialize a new instance of the garbage collector.
    async fn initialize_gc<E: Executor>(&self) {
        let peer = self.clone();
        spawn_task_loop!(E, {
            const SLEEP: u64 = 60 * 10; // 10 minutes
            loop {
                // Sleep for the heartbeat interval.
                tokio::time::sleep(Duration::from_secs(SLEEP)).await;

                // Clear the seen messages to only those in the last 5 seconds.
                peer.seen_messages
                    .write()
                    .await
                    .retain(|_, timestamp| timestamp.elapsed().unwrap_or_default().as_secs() <= 5);
            }
        });
    }
}
