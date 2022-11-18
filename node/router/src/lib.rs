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

mod disconnect;
pub use disconnect::*;

mod handshake;
pub use handshake::*;

mod peer;
pub use peer::*;

mod reading;
pub use reading::*;

mod writing;
pub use writing::*;

use snarkos_node_executor::{NodeType, RawStatus};
use snarkos_node_messages::*;
use snarkos_node_tcp::{protocols::Writing, Config, Tcp};
use snarkvm::prelude::{Address, Network};

use anyhow::Result;
use indexmap::{IndexMap, IndexSet};
use parking_lot::RwLock;
use rand::{prelude::IteratorRandom, rngs::OsRng};
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{atomic::AtomicU8, Arc},
    time::{Duration, SystemTime},
};
use std::marker::PhantomData;
use std::time::Instant;

// TODO (raychu86): Move this declaration.
const ALEO_MAXIMUM_FORK_DEPTH: u32 = 4096;

/// The first-seen port number, number of attempts, and timestamp of the last inbound connection request.
type ConnectionStats = ((u16, u32), SystemTime);

#[derive(Clone)]
pub struct Router<N: Network, R: Routes<N>> {
    /// The TCP stack.
    tcp: Tcp,
    /// The address of the node.
    address: Address<N>,
    /// The node's current state.
    status: RawStatus,
    /// The set of trusted peers.
    trusted_peers: Arc<IndexSet<SocketAddr>>,
    /// The map of connected peer IPs to their peer handlers.
    connected_peers: Arc<RwLock<IndexMap<SocketAddr, Peer>>>,
    /// The set of candidate peer IPs.
    candidate_peers: Arc<RwLock<IndexSet<SocketAddr>>>,
    /// The set of restricted peer IPs.
    restricted_peers: Arc<RwLock<IndexMap<SocketAddr, Instant>>>,
    /// The map of peers to their first-seen port number, number of attempts, and timestamp of the last inbound connection request.
    seen_inbound_connections: Arc<RwLock<IndexMap<SocketAddr, ConnectionStats>>>,
    /// The cache.
    pub cache: Cache<N>,
    /// The map of peer IPs to the number of puzzle requests.
    pub seen_inbound_puzzle_requests: Arc<RwLock<IndexMap<SocketAddr, Arc<AtomicU8>>>>,
}

#[rustfmt::skip]
impl<N: Network, R: Routes<N>> Router<N, R> {
    /// The duration in seconds to sleep in between heartbeat executions.
    const HEARTBEAT_IN_SECS: u64 = 9; // 9 seconds
    /// The frequency at which the node sends a puzzle request.
    const PUZZLE_REQUEST_IN_SECS: u64 = N::ANCHOR_TIME as u64;
    /// The maximum number of puzzle requests per interval.
    const MAXIMUM_PUZZLE_REQUESTS_PER_INTERVAL: u8 = 10;
    /// The maximum number of candidate peers permitted to be stored in the node.
    const MAXIMUM_CANDIDATE_PEERS: usize = 10_000;
    /// The maximum number of connection failures permitted by an inbound connecting peer.
    const MAXIMUM_CONNECTION_FAILURES: u32 = 3;
    /// The duration in seconds to sleep in between ping requests with a connected peer.
    const PING_SLEEP_IN_SECS: u64 = 60; // 1 minute
    /// The duration in seconds after which a connected peer is considered inactive or
    /// disconnected if no message has been received in the meantime.
    const RADIO_SILENCE_IN_SECS: u64 = 180; // 3 minutes
}

impl<N: Network, R: Routes<N>> Router<N, R> {
    /// Initializes a new `Router` instance.
    pub async fn new(
        node_ip: SocketAddr,
        address: Address<N>,
        trusted_peers: &[SocketAddr],
    ) -> Result<Self> {
        // Initialize the router.
        let router = Self {
            tcp: Tcp::new(Config::new(node_ip, R::MAXIMUM_NUMBER_OF_PEERS as u16)).await?,
            address,
            status: RawStatus::new(),
            trusted_peers: Arc::new(trusted_peers.iter().copied().collect()),
            connected_peers: Default::default(),
            candidate_peers: Default::default(),
            restricted_peers: Default::default(),
            cache: Default::default(),
            seen_inbound_puzzle_requests: Default::default(),
            seen_inbound_connections: Default::default(),
        };

        // Initialize the heartbeat.
        router.initialize_heartbeat().await;
        // Initialize the puzzle request.
        router.initialize_puzzle_request().await;
        // Initialize the report.
        router.initialize_report().await;
        // Initialize the GC.
        router.initialize_gc().await;

        Ok(router)
    }

    /// Returns the IP address of this node.
    pub fn local_ip(&self) -> SocketAddr {
        self.tcp.listening_addr().expect("The listening address for this node must be present")
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

    /// Returns the list of metrics for the connected peers.
    pub fn connected_metrics(&self) -> Vec<(SocketAddr, NodeType)> {
        let mut connected_metrics = Vec::new();
        for (ip, peer) in self.connected_peers.read().iter() {
            connected_metrics.push((*ip, peer.node_type()));
        }
        connected_metrics
    }

    /// Returns the list of connected peers that are beacons.
    pub fn connected_beacons(&self) -> Vec<SocketAddr> {
        let mut connected_beacons = Vec::new();
        for (ip, peer) in self.connected_peers.read().iter() {
            if peer.is_beacon() {
                connected_beacons.push(*ip);
            }
        }
        connected_beacons
    }

    /// Returns the list of reliable peers.
    pub fn reliable_peers(&self) -> Vec<SocketAddr> {
        let mut connected_peers: Vec<_> = self.connected_peers.read().keys().copied().collect();
        connected_peers.retain(|ip| self.trusted_peers.contains(ip));
        connected_peers
    }

    /// Inserts the given peer into the connected peers.
    pub fn insert_connected_peer(&self, peer: Peer) {
        // Add an entry for this `Peer` in the connected peers.
        self.connected_peers.write().insert(*peer.ip(), peer.clone());
        // Remove this peer from the candidate peers, if it exists.
        self.candidate_peers.write().remove(peer.ip());
        // Remove this peer from the restricted peers, if it exists.
        self.restricted_peers.write().remove(peer.ip());
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
    pub fn insert_restricted_peer(&self, peer_addr: SocketAddr) {
        // Remove this peer from the connected peers, if it exists.
        self.connected_peers.write().remove(&peer_addr);
        // Remove this peer from the candidate peers, if it exists.
        self.candidate_peers.write().remove(&peer_addr);
        // Add the peer to the restricted peers.
        self.restricted_peers.write().insert(peer_addr, Instant::now());
    }

    /// Inserts the disconnected peer into the candidate peers.
    pub fn insert_disconnected_peer(&self, peer_addr: SocketAddr) {
        // Remove this peer from the connected peers, if it exists.
        self.connected_peers.write().remove(&peer_addr);
        // Add the peer to the candidate peers.
        self.candidate_peers.write().insert(peer_addr);
    }

    /// Updates the connected peer with the given function.
    pub fn update_connected_peer<Fn: FnMut(&mut Peer)>(&self, peer_addr: SocketAddr, mut write_fn: Fn) {
        if let Some(peer) = self.connected_peers.write().get_mut(&peer_addr) {
            write_fn(peer)
        }
    }

    /// Removes the given address from the candidate peers, if it exists.
    pub fn remove_candidate_peer(&self, peer_addr: SocketAddr) {
        self.candidate_peers.write().remove(&peer_addr);
    }

    /// Sends a "PuzzleRequest" to a reliable peer.
    pub fn send_puzzle_request(&self, node_type: NodeType) {
        // TODO (howardwu): Change this logic for Phase 3.
        // Retrieve a reliable peer.
        let reliable_peer = match node_type.is_validator() {
            true => self.connected_beacons().first().copied(),
            false => self.reliable_peers().first().copied(),
        };
        // If a reliable peer exists, send a "PuzzleRequest" to it.
        if let Some(reliable_peer) = reliable_peer {
            // Send the "PuzzleRequest" to the reliable peer.
            self.send(reliable_peer, Message::PuzzleRequest(PuzzleRequest));
        } else {
            warn!("[PuzzleRequest] There are no reliable peers available yet");
        }
    }

    /// Sends the given message to specified peer.
    pub fn send(&self, peer_ip: SocketAddr, message: Message<N>) {
        // Determine whether to send the message.
        if !self.should_send(&message) {
            return;
        }
        // Ensure the peer is connected before sending.
        match self.connected_peers.read().contains_key(&peer_ip) {
            true => {
                trace!("Sending '{}' to '{peer_ip}'", message.name());
                if let Err(error) = self.unicast(peer_ip, message) {
                    trace!("Failed to send message to '{peer_ip}': {error}");
                }
            }
            false => warn!("Attempted to send to a non-connected peer {peer_ip}"),
        }
    }

    /// Sends the given message to every connected peer, excluding the sender and any specified peer IPs.
    pub fn propagate(&self, mut message: Message<N>, excluded_peers: Vec<SocketAddr>) {
        // // Perform ahead-of-time, non-blocking serialization just once for applicable objects.
        // if let Message::UnconfirmedBlock(ref mut message) = message {
        //     if let Ok(serialized_block) = Data::serialize(message.block.clone()).await {
        //         let _ = std::mem::replace(&mut message.block, Data::Buffer(serialized_block));
        //     } else {
        //         error!("Block serialization is bugged");
        //     }
        // } else if let Message::UnconfirmedSolution(ref mut message) = message {
        //     if let Ok(serialized_solution) = Data::serialize(message.solution.clone()).await {
        //         let _ = std::mem::replace(&mut message.solution, Data::Buffer(serialized_solution));
        //     } else {
        //         error!("Solution serialization is bugged");
        //     }
        // } else if let Message::UnconfirmedTransaction(ref mut message) = message {
        //     if let Ok(serialized_transaction) = Data::serialize(message.transaction.clone()).await {
        //         let _ = std::mem::replace(&mut message.transaction, Data::Buffer(serialized_transaction));
        //     } else {
        //         error!("Transaction serialization is bugged");
        //     }
        // }

        // Determine whether to send the message.
        if !self.should_send(&message) {
            return;
        }
        // Iterate through all peers that are not the sender and excluded peers.
        for peer_ip in self
            .connected_peers()
            .iter()
            .filter(|peer_ip| !self.is_local_ip(peer_ip) && !excluded_peers.contains(peer_ip))
        {
            trace!("Sending '{}' to '{peer_ip}'", message.name());
            if let Err(error) = self.unicast(*peer_ip, message.clone()) {
                warn!("Failed to send '{}' to '{peer_ip}': {error}", message.name());
            }
        }
    }

    /// Sends the given message to every connected beacon, excluding the sender and any specified beacon IPs.
    pub fn propagate_to_beacons(&self, mut message: Message<N>, excluded_beacons: Vec<SocketAddr>) {
        // // Perform ahead-of-time, non-blocking serialization just once for applicable objects.
        // if let Message::UnconfirmedBlock(ref mut message) = message {
        //     if let Ok(serialized_block) = Data::serialize(message.block.clone()).await {
        //         let _ = std::mem::replace(&mut message.block, Data::Buffer(serialized_block));
        //     } else {
        //         error!("Block serialization is bugged");
        //     }
        // } else if let Message::UnconfirmedSolution(ref mut message) = message {
        //     if let Ok(serialized_solution) = Data::serialize(message.solution.clone()).await {
        //         let _ = std::mem::replace(&mut message.solution, Data::Buffer(serialized_solution));
        //     } else {
        //         error!("Solution serialization is bugged");
        //     }
        // } else if let Message::UnconfirmedTransaction(ref mut message) = message {
        //     if let Ok(serialized_transaction) = Data::serialize(message.transaction.clone()).await {
        //         let _ = std::mem::replace(&mut message.transaction, Data::Buffer(serialized_transaction));
        //     } else {
        //         error!("Transaction serialization is bugged");
        //     }
        // }

        // Determine whether to send the message.
        if !self.should_send(&message) {
            return;
        }
        // Iterate through all beacons that are not the sender and excluded beacons.
        for peer_ip in self
            .connected_beacons()
            .iter()
            .filter(|peer_ip| !self.is_local_ip(peer_ip) && !excluded_beacons.contains(peer_ip))
        {
            trace!("Sending '{}' to '{peer_ip}'", message.name());
            if let Err(error) = self.unicast(*peer_ip, message.clone()) {
                warn!("Failed to send '{}' to '{peer_ip}': {error}", message.name());
            }
        }
    }

    /// Returns `true` if the message should be sent.
    fn should_send(&self, message: &Message<N>) -> bool {
        // Determine whether to send the message.
        match message {
            Message::UnconfirmedBlock(message) => {
                // Update the timestamp for the unconfirmed block.
                let seen_before = self.cache.insert_outbound_block(message.block_hash).is_some();
                // Determine whether to send the block.
                !seen_before
            }
            Message::UnconfirmedSolution(message) => {
                // Update the timestamp for the unconfirmed solution.
                let seen_before = self.cache.insert_outbound_solution(message.puzzle_commitment).is_some();
                // Determine whether to send the solution.
                !seen_before
            }
            Message::UnconfirmedTransaction(message) => {
                // Update the timestamp for the unconfirmed transaction.
                let seen_before = self.cache.insert_outbound_transaction(message.transaction_id).is_some();
                // Determine whether to send the transaction.
                !seen_before
            }
            // For all other message types, return `true`.
            _ => true,
        }
    }
}

impl<N: Network, R: Routes<N>> Router<N, R> {
    /// Initialize a new instance of the heartbeat.
    async fn initialize_heartbeat(&self) {
        let router = self.clone();
        tokio::spawn(async move {
            loop {
                // Process a heartbeat in the router.
                router.heartbeat().await;
                // Sleep for `Self::HEARTBEAT_IN_SECS` seconds.
                tokio::time::sleep(Duration::from_secs(Self::HEARTBEAT_IN_SECS)).await;
            }
        });
    }

    /// Initialize a new instance of the puzzle request.
    async fn initialize_puzzle_request(&self) {
        if !R::NODE_TYPE.is_beacon() {
            let router = self.clone();
            tokio::spawn(async move {
                loop {
                    // Send a "PuzzleRequest".
                    router.send_puzzle_request(R::NODE_TYPE);
                    // Sleep for `Self::PUZZLE_REQUEST_IN_SECS` seconds.
                    tokio::time::sleep(Duration::from_secs(Self::PUZZLE_REQUEST_IN_SECS)).await;
                }
            });
        }
    }

    /// Initialize a new instance of the report.
    async fn initialize_report(&self) {
        let router = self.clone();
        tokio::spawn(async move {
            let url = "https://vm.aleo.org/testnet3/report";
            loop {
                // Prepare the report.
                let mut report = HashMap::new();
                report.insert("node_address".to_string(), router.address.to_string());
                report.insert("node_type".to_string(), R::NODE_TYPE.to_string());
                // Transmit the report.
                if reqwest::Client::new().post(url).json(&report).send().await.is_err() {
                    warn!("Failed to send report");
                }
                // Sleep for a fixed duration in seconds.
                tokio::time::sleep(Duration::from_secs(3600 * 6)).await;
            }
        });
    }

    /// Initialize a new instance of the garbage collector.
    async fn initialize_gc(&self) {
        let router = self.clone();
        tokio::spawn(async move {
            loop {
                // Sleep for the interval.
                tokio::time::sleep(Duration::from_secs(Self::RADIO_SILENCE_IN_SECS)).await;
                // Clear the seen puzzle requests.
                router.seen_inbound_puzzle_requests.write().clear();
            }
        });
    }
}

impl<N: Network, R: Routes<N>> Router<N, R> {
    /// Handles the heartbeat request.
    async fn heartbeat(&self) {
        debug!("Peers: {:?}", self.connected_peers());

        // TODO (howardwu): Remove this in Phase 3.
        if R::NODE_TYPE.is_beacon() {
            // Proceed to send disconnect requests to these peers.
            for peer_ip in self.connected_peers() {
                if !self.trusted_peers().contains(&peer_ip) {
                    info!("Disconnecting from '{peer_ip}' (exceeded maximum connections)");
                    self.send(peer_ip, Message::Disconnect(DisconnectReason::TooManyPeers.into()));
                    // Disconnect from this peer.
                    let _disconnected = self.tcp.disconnect(peer_ip).await;
                    debug_assert!(_disconnected);
                    // Restrict this peer to prevent reconnection.
                    self.insert_restricted_peer(peer_ip);
                }
            }
        }

        // Check if any connected peer is stale.
        let connected_peers = self.connected_peers.read().clone();
        for (peer_ip, peer) in connected_peers.into_iter() {
            // Disconnect if the peer has not communicated back within the predefined time.
            let last_seen_elapsed = peer.last_seen().elapsed().as_secs();
            if last_seen_elapsed > Self::RADIO_SILENCE_IN_SECS {
                warn!("Peer {peer_ip} has not communicated in {last_seen_elapsed} seconds");
                // Disconnect from this peer.
                let _disconnected = self.tcp.disconnect(peer_ip).await;
                debug_assert!(_disconnected);
                // Restrict this peer to prevent reconnection.
                self.insert_restricted_peer(peer_ip);
            }

            // Drop the peer, if they have sent more than 50 messages in the last 5 seconds.
            let frequency = peer.message_frequency();
            if frequency >= 50 {
                warn!("Dropping {peer_ip} for spamming messages (frequency = {frequency})");
                // Disconnect from this peer.
                let _disconnected = self.tcp.disconnect(peer_ip).await;
                debug_assert!(_disconnected);
                // Restrict this peer to prevent reconnection.
                self.insert_restricted_peer(peer_ip);
            }
        }

        // Compute the number of excess peers.
        let num_excess_peers = self.number_of_connected_peers().saturating_sub(R::MAXIMUM_NUMBER_OF_PEERS);
        // Ensure the number of connected peers is below the maximum threshold.
        if num_excess_peers > 0 {
            debug!("Exceeded maximum number of connected peers, disconnecting from {num_excess_peers} peers");
            // Determine the peers to disconnect from.
            let peer_ips_to_disconnect = self
                .connected_peers()
                .into_iter()
                .filter(
                    |peer_ip| /* !E::beacon_nodes().contains(&peer_ip) && */ !self.trusted_peers().contains(peer_ip),
                )
                .take(num_excess_peers);

            // Proceed to send disconnect requests to these peers.
            for peer_ip in peer_ips_to_disconnect {
                info!("Disconnecting from '{peer_ip}' (exceeded maximum connections)");
                self.send(peer_ip, Message::Disconnect(DisconnectReason::TooManyPeers.into()));
                // Disconnect from this peer.
                let _disconnected = self.tcp.disconnect(peer_ip).await;
                debug_assert!(_disconnected);
                // Restrict this peer to prevent reconnection.
                self.insert_restricted_peer(peer_ip);
            }
        }

        // TODO (howardwu): This logic can be optimized and unified with the context around it.
        // Determine if the node is connected to more sync nodes than allowed.
        let connected_beacons = self.connected_beacons();
        let num_excess_beacons = connected_beacons.len().saturating_sub(1);
        if num_excess_beacons > 0 {
            debug!("Exceeded maximum number of beacons");

            // Proceed to send disconnect requests to these peers.
            for peer_ip in connected_beacons.iter().copied().choose_multiple(&mut OsRng::default(), num_excess_beacons)
            {
                info!("Disconnecting from 'beacon' {peer_ip} (exceeded maximum connections)");
                self.send(peer_ip, Message::Disconnect(DisconnectReason::TooManyPeers.into()));
                // Disconnect from this peer.
                let _disconnected = self.tcp.disconnect(peer_ip).await;
                debug_assert!(_disconnected);
                // Restrict this peer to prevent reconnection.
                self.insert_restricted_peer(peer_ip);
            }
        }

        // Ensure that the trusted nodes are connected.
        for peer_ip in self.trusted_peers() {
            // If the peer is not connected, attempt to connect to it.
            if !self.is_connected(peer_ip) {
                // Attempt to connect to the trusted peer.
                if let Err(error) = self.tcp.connect(*peer_ip).await {
                    warn!("Failed to connect to trusted peer '{peer_ip}': {error}");
                }
            }
        }

        // Obtain the number of connected peers.
        let num_connected = self.number_of_connected_peers();
        let num_to_connect_with = R::MINIMUM_NUMBER_OF_PEERS.saturating_sub(num_connected);
        // Request more peers if the number of connected peers is below the threshold.
        if num_to_connect_with > 0 {
            trace!("Sending requests for more peer connections");

            // Request more peers from the connected peers.
            for candidate_addr in self.candidate_peers().into_iter().take(num_to_connect_with) {
                // Attempt to connect to the candidate peer.
                let connection_succesful = self.tcp.connect(candidate_addr).await.is_ok();
                // Remove the peer from the candidate peers.
                self.remove_candidate_peer(candidate_addr);
                // Restrict the peer if the connection was not successful.
                if !connection_succesful {
                    self.insert_restricted_peer(candidate_addr);
                }
            }

            // If we have connected peers, request more addresses from them.
            if num_connected > 0 {
                for peer_ip in self.connected_peers().iter().choose_multiple(&mut OsRng::default(), 3) {
                    self.send(*peer_ip, Message::PeerRequest(PeerRequest));
                }
            }
        }
    }
}
