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

mod helpers;
pub use helpers::*;

mod handshake;
pub use handshake::*;

mod heartbeat;
pub use heartbeat::*;

mod inbound;
pub use inbound::*;

mod outbound;
pub use outbound::*;

mod routing;
pub use routing::*;

use snarkos_node_messages::{NodeType, RawStatus, Status};
use snarkos_node_tcp::{Config, Tcp};
use snarkvm::prelude::{Address, Network};

use anyhow::Result;
use core::str::FromStr;
use indexmap::{IndexMap, IndexSet};
use parking_lot::RwLock;
use std::{future::Future, net::SocketAddr, sync::Arc, time::Instant};
use tokio::task::JoinHandle;

// TODO (raychu86): Move this declaration.
const ALEO_MAXIMUM_FORK_DEPTH: u32 = 4096;

#[derive(Clone)]
pub struct Router<N: Network> {
    /// The TCP stack.
    tcp: Tcp,
    /// The local IP address of the node.
    local_ip: SocketAddr,
    /// The node type.
    node_type: NodeType,
    /// The address of the node.
    address: Address<N>,
    /// The node's current state.
    status: RawStatus,
    /// The cache.
    cache: Cache<N>,
    /// The resolver.
    resolver: Resolver,
    /// The set of trusted peers.
    trusted_peers: Arc<IndexSet<SocketAddr>>,
    /// The map of connected peer IPs to their peer handlers.
    connected_peers: Arc<RwLock<IndexMap<SocketAddr, Peer>>>,
    /// The set of candidate peer IPs.
    candidate_peers: Arc<RwLock<IndexSet<SocketAddr>>>,
    /// The set of restricted peer IPs.
    restricted_peers: Arc<RwLock<IndexMap<SocketAddr, Instant>>>,
    /// The spawned handles.
    handles: Arc<RwLock<Vec<JoinHandle<()>>>>,
    /// The boolean flag for the development mode.
    is_dev: bool,
}

impl<N: Network> Router<N> {
    /// The maximum number of candidate peers permitted to be stored in the node.
    const MAXIMUM_CANDIDATE_PEERS: usize = 10_000;
    /// The maximum number of connection failures permitted by an inbound connecting peer.
    const MAXIMUM_CONNECTION_FAILURES: usize = 3;
    /// The duration in seconds after which a connected peer is considered inactive or
    /// disconnected if no message has been received in the meantime.
    const RADIO_SILENCE_IN_SECS: u64 = 180; // 3 minutes
}

impl<N: Network> Router<N> {
    /// Initializes a new `Router` instance.
    pub async fn new(
        node_ip: SocketAddr,
        node_type: NodeType,
        address: Address<N>,
        trusted_peers: &[SocketAddr],
        max_peers: u16,
        is_dev: bool,
    ) -> Result<Self> {
        // Initialize the TCP stack.
        let tcp = Tcp::new(Config::new(node_ip, max_peers)).await?;
        // Fetch the listening IP address.
        let local_ip = tcp.listening_addr().expect("The listening address for this node must be present");
        // Initialize the router.
        Ok(Self {
            tcp,
            local_ip,
            node_type,
            address,
            status: RawStatus::new(),
            cache: Default::default(),
            resolver: Default::default(),
            trusted_peers: Arc::new(trusted_peers.iter().copied().collect()),
            connected_peers: Default::default(),
            candidate_peers: Default::default(),
            restricted_peers: Default::default(),
            handles: Default::default(),
            is_dev,
        })
    }

    /// Attempts to connect to the given peer IP.
    pub fn connect(&self, peer_ip: SocketAddr) {
        let router = self.clone();
        tokio::spawn(async move {
            // Attempt to connect to the candidate peer.
            debug!("Connecting to {peer_ip}...");
            if let Err(error) = router.tcp.connect(peer_ip).await {
                warn!("{error}");
                // Restrict the peer, if the connection failed, and is neither trusted nor a bootstrap peer.
                if !router.trusted_peers.contains(&peer_ip) && !router.bootstrap_peers().contains(&peer_ip) {
                    router.insert_restricted_peer(peer_ip);
                }
            }
            // Remove the peer from the candidate peers.
            router.remove_candidate_peer(peer_ip);
        });
    }

    /// Disconnects from the given peer IP, if the peer is connected.
    pub fn disconnect(&self, peer_ip: SocketAddr) {
        let router = self.clone();
        tokio::spawn(async move {
            // Disconnect from this peer.
            let _disconnected = router.tcp.disconnect(peer_ip).await;
            debug_assert!(_disconnected);
            // TODO (howardwu): Revisit this. It appears `handle_disconnect` does not necessarily trigger.
            //  See https://github.com/AleoHQ/snarkOS/issues/2102.
            // Remove the peer from the connected peers.
            router.remove_connected_peer(peer_ip);
        });
    }

    /// Returns the node type.
    pub const fn node_type(&self) -> NodeType {
        self.node_type
    }

    /// Returns the Aleo address of the node.
    pub const fn address(&self) -> Address<N> {
        self.address
    }

    /// Returns the status.
    pub fn status(&self) -> Status {
        self.status.get()
    }

    /// Returns `true` if the node is in development mode.
    pub const fn is_dev(&self) -> bool {
        self.is_dev
    }

    /// Returns the IP address of this node.
    pub const fn local_ip(&self) -> SocketAddr {
        self.local_ip
    }

    /// Returns `true` if the given IP is this node.
    pub fn is_local_ip(&self, ip: &SocketAddr) -> bool {
        *ip == self.local_ip()
            || (ip.ip().is_unspecified() || ip.ip().is_loopback()) && ip.port() == self.local_ip().port()
    }

    /// Returns `true` if the node is connected to the given IP.
    pub fn is_connected(&self, ip: &SocketAddr) -> bool {
        self.connected_peers.read().contains_key(ip)
    }

    /// Returns `true` if the given IP is restricted.
    pub fn is_restricted(&self, ip: &SocketAddr) -> bool {
        match self.restricted_peers.read().get(ip) {
            Some(timestamp) => timestamp.elapsed().as_secs() < Self::RADIO_SILENCE_IN_SECS,
            None => false,
        }
    }

    /// Returns the listener IP address from the (ambiguous) peer address.
    pub fn resolve_to_listener(&self, peer_addr: &SocketAddr) -> Option<SocketAddr> {
        self.resolver.get_listener(peer_addr)
    }

    /// Returns the (ambiguous) peer address from the listener IP address.
    pub fn resolve_to_ambiguous(&self, peer_ip: &SocketAddr) -> Option<SocketAddr> {
        self.resolver.get_ambiguous(peer_ip)
    }

    /// Returns the maximum number of connected peers.
    pub fn max_connected_peers(&self) -> usize {
        self.tcp.config().max_connections as usize
    }

    /// Returns the number of connected peers.
    pub fn number_of_connected_peers(&self) -> usize {
        self.connected_peers.read().len()
    }

    /// Returns the number of candidate peers.
    pub fn number_of_candidate_peers(&self) -> usize {
        self.candidate_peers.read().len()
    }

    /// Returns the number of restricted peers.
    pub fn number_of_restricted_peers(&self) -> usize {
        self.restricted_peers.read().len()
    }

    /// Returns the connected peers.
    pub fn get_connected_peers(&self) -> Vec<Peer> {
        self.connected_peers.read().values().cloned().collect()
    }

    /// Returns the list of connected peers.
    pub fn connected_peers(&self) -> Vec<SocketAddr> {
        self.connected_peers.read().keys().copied().collect()
    }

    /// Returns the list of candidate peers.
    pub fn candidate_peers(&self) -> IndexSet<SocketAddr> {
        self.candidate_peers.read().clone()
    }

    /// Returns the list of restricted peers.
    pub fn restricted_peers(&self) -> Vec<SocketAddr> {
        self.restricted_peers.read().keys().copied().collect()
    }

    /// Returns the list of trusted peers.
    pub fn trusted_peers(&self) -> &IndexSet<SocketAddr> {
        &self.trusted_peers
    }

    /// Returns the list of bootstrap peers.
    pub fn bootstrap_peers(&self) -> Vec<SocketAddr> {
        if self.is_dev {
            // In development mode, connect to the dedicated local beacon.
            match self.node_type.is_beacon() {
                true => vec![],
                false => vec![SocketAddr::from(([127, 0, 0, 1], 4130))],
            }
        } else {
            // TODO (howardwu): Change this for Phase 3.
            vec![
                SocketAddr::from_str("164.92.111.59:4133").unwrap(),
                SocketAddr::from_str("159.223.204.96:4133").unwrap(),
                SocketAddr::from_str("167.71.219.176:4133").unwrap(),
                SocketAddr::from_str("157.245.205.209:4133").unwrap(),
                SocketAddr::from_str("134.122.95.106:4133").unwrap(),
                SocketAddr::from_str("161.35.24.55:4133").unwrap(),
                SocketAddr::from_str("138.68.103.139:4133").unwrap(),
                SocketAddr::from_str("207.154.215.49:4133").unwrap(),
                SocketAddr::from_str("46.101.114.158:4133").unwrap(),
                SocketAddr::from_str("138.197.190.94:4133").unwrap(),
            ]
        }
    }

    /// Returns the list of connected bootstrap peers.
    pub fn connected_bootstrap_peers(&self) -> Vec<SocketAddr> {
        let mut connected_bootstrap = Vec::new();
        for bootstrap_ip in self.bootstrap_peers() {
            if self.is_connected(&bootstrap_ip) {
                connected_bootstrap.push(bootstrap_ip);
            }
        }
        connected_bootstrap
    }

    /// Returns the list of metrics for the connected peers.
    pub fn connected_metrics(&self) -> Vec<(SocketAddr, NodeType)> {
        self.connected_peers.read().iter().map(|(ip, peer)| (*ip, peer.node_type())).collect()
    }

    /// Returns the list of connected peers that are beacons.
    pub fn connected_beacons(&self) -> Vec<SocketAddr> {
        self.connected_peers
            .read()
            .iter()
            .filter_map(|(ip, peer)| match peer.is_beacon() {
                true => Some(*ip),
                false => None,
            })
            .collect()
    }

    /// Returns the oldest connected peer.
    pub fn oldest_connected_peer(&self) -> Option<SocketAddr> {
        self.connected_peers.read().iter().min_by_key(|(_, peer)| peer.last_seen()).map(|(peer_ip, _)| *peer_ip)
    }

    /// Inserts the given peer into the connected peers.
    pub fn insert_connected_peer(&self, peer: Peer, peer_addr: SocketAddr) {
        // Adds a bidirectional map between the listener address and (ambiguous) peer address.
        self.resolver.insert_peer(peer.ip(), peer_addr);
        // Add an entry for this `Peer` in the connected peers.
        self.connected_peers.write().insert(peer.ip(), peer.clone());
        // Remove this peer from the candidate peers, if it exists.
        self.candidate_peers.write().remove(&peer.ip());
        // Remove this peer from the restricted peers, if it exists.
        self.restricted_peers.write().remove(&peer.ip());
    }

    /// Inserts the given peer IPs to the set of candidate peers.
    ///
    /// This method skips adding any given peers if the combined size exceeds the threshold,
    /// as the peer providing this list could be subverting the protocol.
    pub fn insert_candidate_peers(&self, peers: &[SocketAddr]) {
        // Compute the maximum number of candidate peers.
        let max_candidate_peers = Self::MAXIMUM_CANDIDATE_PEERS.saturating_sub(self.number_of_candidate_peers());
        // Ensure the combined number of peers does not surpass the threshold.
        for peer_ip in peers.iter().take(max_candidate_peers) {
            // Ensure the peer is not itself, is not already connected, and is not restricted.
            if self.is_local_ip(peer_ip) || self.is_connected(peer_ip) || self.is_restricted(peer_ip) {
                continue;
            }
            // Proceed to insert each new candidate peer IP.
            self.candidate_peers.write().insert(*peer_ip);
        }
    }

    /// Inserts the given peer into the restricted peers.
    pub fn insert_restricted_peer(&self, peer_ip: SocketAddr) {
        // Remove this peer from the connected peers, if it exists.
        self.connected_peers.write().remove(&peer_ip);
        // Remove this peer from the candidate peers, if it exists.
        self.candidate_peers.write().remove(&peer_ip);
        // Add the peer to the restricted peers.
        self.restricted_peers.write().insert(peer_ip, Instant::now());
    }

    /// Removes the connected peer and adds them to the candidate peers.
    pub fn remove_connected_peer(&self, peer_ip: SocketAddr) {
        // Removes the bidirectional map between the listener address and (ambiguous) peer address.
        self.resolver.remove_peer(&peer_ip);
        // Remove this peer from the connected peers, if it exists.
        self.connected_peers.write().remove(&peer_ip);
        // Add the peer to the candidate peers.
        self.candidate_peers.write().insert(peer_ip);
    }

    /// Removes the given address from the candidate peers, if it exists.
    pub fn remove_candidate_peer(&self, peer_ip: SocketAddr) {
        self.candidate_peers.write().remove(&peer_ip);
    }

    /// Updates the connected peer with the given function.
    pub fn update_connected_peer<Fn: FnMut(&mut Peer)>(&self, peer_ip: SocketAddr, mut write_fn: Fn) {
        if let Some(peer) = self.connected_peers.write().get_mut(&peer_ip) {
            write_fn(peer)
        }
    }

    /// Spawns a task with the given future.
    pub fn spawn<T: Future<Output = ()> + Send + 'static>(&self, future: T) {
        self.handles.write().push(tokio::spawn(future));
    }

    /// Shuts down the router.
    pub async fn shut_down(&self) {
        trace!("Shutting down the router...");
        // Update the node status.
        self.status.update(Status::ShuttingDown);
        // Abort the tasks.
        self.handles.read().iter().for_each(|handle| handle.abort());
        // Close the listener.
        self.tcp.shut_down().await;
    }
}
