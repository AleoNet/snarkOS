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

mod handler;
mod handshake;

use crate::{
    message::{Data, DisconnectReason, Message, MessageCodec},
    peers::{ConnectionResult, PeersRequest},
    spawn_task,
    state::State,
};

use snarkos_environment::{
    helpers::{NodeType, Status},
    Environment,
};
use snarkvm::prelude::*;

use anyhow::Result;
use futures::SinkExt;
use std::{
    collections::{HashMap, HashSet},
    net::SocketAddr,
    sync::Arc,
    time::{Duration, Instant, SystemTime},
};
use tokio::{
    net::TcpStream,
    sync::{mpsc, RwLock},
    task,
    time::timeout,
};
use tokio_stream::StreamExt;
use tokio_util::codec::Framed;

// TODO (raychu86): Move this declaration.
const ALEO_MAXIMUM_FORK_DEPTH: u32 = 4096;

/// Shorthand for the parent half of the `Peer` outbound message channel.
pub(crate) type PeerRouter<N> = mpsc::Sender<Message<N>>;
/// Shorthand for the child half of the `Peer` outbound message channel.
type PeerHandler<N> = mpsc::Receiver<Message<N>>;

///
/// The state for each connected client.
///
#[derive(Clone, Debug)]
pub struct Peer<N: Network, E: Environment> {
    /// The state of the node.
    state: State<N, E>,
    /// The router to the peer.
    peer_router: PeerRouter<N>,
    /// The IP address of the peer, with the port set to the listener port.
    listener_ip: Arc<SocketAddr>,
    /// The message version of the peer.
    version: Arc<RwLock<u32>>,
    /// The node type of the peer.
    node_type: Arc<RwLock<NodeType>>,
    /// The node type of the peer.
    status: Arc<RwLock<Status>>,
    /// The block height of the peer.
    block_height: Arc<RwLock<u32>>,
    /// The timestamp of the last message received from this peer.
    last_seen: Arc<RwLock<Instant>>,
    /// The map of block hashes to their last seen timestamp.
    seen_inbound_blocks: Arc<RwLock<HashMap<N::BlockHash, SystemTime>>>,
    /// The map of transaction IDs to their last seen timestamp.
    seen_inbound_transactions: Arc<RwLock<HashMap<N::TransactionID, SystemTime>>>,
    /// The map of peers to a map of block hashes to their last seen timestamp.
    seen_outbound_blocks: Arc<RwLock<HashMap<N::BlockHash, SystemTime>>>,
    /// The map of peers to a map of transaction IDs to their last seen timestamp.
    seen_outbound_transactions: Arc<RwLock<HashMap<N::TransactionID, SystemTime>>>,
}

impl<N: Network, E: Environment> Peer<N, E> {
    /// Returns the IP address of the peer, with the port set to the listener port.
    pub fn ip(&self) -> &SocketAddr {
        &self.listener_ip
    }

    /// Sends the given message to this peer.
    pub async fn send(&self, message: Message<N>) -> Result<()> {
        trace!("Sending '{}' to {}", message.name(), self.ip());
        self.peer_router.send(message).await?;
        Ok(())
    }
}
