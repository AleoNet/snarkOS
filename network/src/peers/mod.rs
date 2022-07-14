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

mod update;

use crate::{
    message::{Data, DisconnectReason, Message},
    peer::{Peer, PeerRouter},
    spawn_task,
    state::State,
};
use snarkos_environment::Environment;
use snarkvm::prelude::*;

#[cfg(feature = "rpc")]
use snarkos_rpc::{initialize_rpc_node, RpcContext};

#[cfg(any(feature = "test", feature = "prometheus"))]
use snarkos_metrics as metrics;

use ::rand::{prelude::IteratorRandom, rngs::OsRng};
use anyhow::Result;
use std::{
    collections::{HashMap, HashSet},
    net::SocketAddr,
    time::{Duration, Instant, SystemTime},
};
use tokio::{
    net::TcpStream,
    sync::{mpsc, oneshot, RwLock},
    task,
    time::timeout,
};

/// Shorthand for the parent half of the `Peers` message channel.
pub type PeersRouter<N, E> = mpsc::Sender<PeersRequest<N, E>>;
/// Shorthand for the child half of the `Peers` message channel.
pub type PeersHandler<N, E> = mpsc::Receiver<PeersRequest<N, E>>;

/// Shorthand for the parent half of the connection result channel.
pub(crate) type ConnectionResult = oneshot::Sender<Result<()>>;

///
/// An enum of requests that the `Peers` struct processes.
///
#[derive(Debug)]
pub enum PeersRequest<N: Network, E: Environment> {
    /// Connect := (peer_ip, connection_result)
    Connect(SocketAddr, ConnectionResult),
    /// Heartbeat
    Heartbeat,
    /// MessagePropagate := (peer_ip, message)
    MessagePropagate(SocketAddr, Message<N>),
    /// MessageSend := (peer_ip, message)
    MessageSend(SocketAddr, Message<N>),
    /// PeerConnecting := (stream, peer_ip)
    PeerConnecting(TcpStream, SocketAddr),
    /// PeerConnected := (peer_ip, peer)
    PeerConnected(SocketAddr, Peer<N, E>),
    /// PeerDisconnected := (peer_ip)
    PeerDisconnected(SocketAddr),
    /// PeerRestricted := (peer_ip)
    PeerRestricted(SocketAddr),
    /// SendPeerResponse := (peer_ip, rtt_start)
    /// Note: rtt_start is for the request/response cycle for sharing peers.
    SendPeerResponse(SocketAddr, Option<Instant>),
    /// ReceivePeerResponse := (\[peer_ip\])
    ReceivePeerResponse(Vec<SocketAddr>),
}

///
/// A list of peers connected to the node.
///
pub struct Peers<N: Network, E: Environment> {
    /// The state of the node.
    state: State<N, E>,
    /// The peers router of the node.
    peers_router: PeersRouter<N, E>,
    /// The map connected peer IPs to their outbound message router.
    connected_peers: RwLock<HashMap<SocketAddr, Peer<N, E>>>,
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
    /// Initializes a new instance of `Peers` and its corresponding handler.
    ///
    pub async fn new(state: State<N, E>) -> (Self, mpsc::Receiver<PeersRequest<N, E>>) {
        // Initialize an MPSC channel for sending requests to the `Peers` struct.
        let (peers_router, peers_handler) = mpsc::channel(1024);

        // Initialize the peers.
        let peers = Self {
            state,
            peers_router,
            connected_peers: Default::default(),
            candidate_peers: Default::default(),
            restricted_peers: Default::default(),
            seen_inbound_connections: Default::default(),
            seen_outbound_connections: Default::default(),
        };

        (peers, peers_handler)
    }

    ///
    /// Returns the peers router.
    ///
    pub fn router(&self) -> &PeersRouter<N, E> {
        &self.peers_router
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
    /// Returns the set of connected beacon nodes.
    ///
    pub async fn connected_beacon_nodes(&self) -> HashSet<SocketAddr> {
        let beacon_nodes = E::beacon_nodes();
        self.connected_peers
            .read()
            .await
            .keys()
            .filter(|addr| beacon_nodes.contains(addr))
            .copied()
            .collect()
    }

    ///
    /// Returns the number of connected beacon nodes.
    ///
    pub async fn number_of_connected_beacon_nodes(&self) -> usize {
        let beacon_nodes = E::beacon_nodes();
        self.connected_peers
            .read()
            .await
            .keys()
            .filter(|addr| beacon_nodes.contains(addr))
            .count()
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
    /// Returns the number of restricted peers.
    ///
    pub async fn number_of_restricted_peers(&self) -> usize {
        self.restricted_peers.read().await.len()
    }
}
