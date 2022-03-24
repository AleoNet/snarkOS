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

use crate::{NetworkState, Peer};
use snarkos_environment::{
    network::{Data, DisconnectReason, Message},
    Environment,
};
use snarkvm::dpc::prelude::*;

#[cfg(any(feature = "test", feature = "prometheus"))]
use snarkos_metrics as metrics;

use anyhow::{bail, Result};
use once_cell::sync::OnceCell;
use rand::{prelude::IteratorRandom, rngs::OsRng, thread_rng, Rng};
use std::{
    collections::{HashMap, HashSet},
    net::SocketAddr,
    sync::Arc,
    time::{Duration, Instant, SystemTime},
};
use tokio::{net::TcpStream, sync::RwLock, time::timeout};

///
/// A list of peers connected to the node server.
///
#[derive(Debug)]
pub struct Peers<N: Network, E: Environment> {
    network_state: OnceCell<NetworkState<N, E>>,
    /// The local address of this node.
    local_ip: SocketAddr,
    /// The local nonce for this node session.
    local_nonce: u64,
    /// The map of known peer IPs to their corresponding `Peer` instance.
    peers: RwLock<HashMap<SocketAddr, Arc<Peer<N, E>>>>,
    /// The map connected peer IPs to their nonce.
    connected_peers: RwLock<HashMap<SocketAddr, u64>>,
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
    /// Initializes a new instance of `Peers`.
    ///
    pub async fn new(local_ip: SocketAddr, local_nonce: Option<u64>) -> Arc<Self> {
        // Sample the nonce.
        let local_nonce = match local_nonce {
            Some(nonce) => nonce,
            None => thread_rng().gen(),
        };

        // Initialize the peers.
        Arc::new(Self {
            network_state: OnceCell::new(),
            local_ip,
            local_nonce,
            peers: Default::default(),
            connected_peers: Default::default(),
            candidate_peers: Default::default(),
            restricted_peers: Default::default(),
            seen_inbound_connections: Default::default(),
            seen_outbound_connections: Default::default(),
        })
    }

    pub fn set_network_state(&self, network_state: NetworkState<N, E>) {
        self.network_state.set(network_state).expect("network state can only be set once");
    }

    fn expect_network_state(&self) -> &NetworkState<N, E> {
        self.network_state.get().expect("network state must be set")
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
    /// Returns the set of connected sync nodes.
    ///
    pub async fn connected_sync_nodes(&self) -> HashSet<SocketAddr> {
        let sync_nodes = E::sync_nodes();
        self.connected_peers
            .read()
            .await
            .keys()
            .filter(|addr| sync_nodes.contains(addr))
            .copied()
            .collect()
    }

    ///
    /// Returns the number of connected sync nodes.
    ///
    pub async fn number_of_connected_sync_nodes(&self) -> usize {
        let sync_nodes = E::sync_nodes();
        self.connected_peers
            .read()
            .await
            .keys()
            .filter(|addr| sync_nodes.contains(addr))
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

    ///
    /// Returns the list of nonces for the connected peers.
    ///
    pub(crate) async fn connected_nonces(&self) -> Vec<u64> {
        self.connected_peers.read().await.values().copied().collect()
    }

    async fn validate_connection(&self, peer_ip: SocketAddr) -> Result<()> {
        // Ensure the peer IP is not this node.
        if peer_ip == self.local_ip
            || (peer_ip.ip().is_unspecified() || peer_ip.ip().is_loopback()) && peer_ip.port() == self.local_ip.port()
        {
            bail!("Skipping connection request to {} (attempted to self-connect)", peer_ip);
        }

        // Ensure the node does not surpass the maximum number of peer connections.
        if self.number_of_connected_peers().await >= E::MAXIMUM_NUMBER_OF_PEERS {
            bail!("Skipping connection request to {} (maximum peers reached)", peer_ip);
        }

        // Ensure the peer is a new connection.
        if self.is_connected_to(peer_ip).await {
            bail!("Skipping connection request to {} (already connected)", peer_ip);
        }

        // Ensure the peer is not restricted.
        if self.is_restricted(peer_ip).await {
            bail!("Skipping connection request to {} (restricted)", peer_ip);
        }

        Ok(())
    }

    pub async fn connect(&self, peer_ip: SocketAddr) -> Result<()> {
        if let Err(error) = self.validate_connection(peer_ip).await {
            debug!("{}", error);
            bail!(error)
        }

        // Attempt to open a TCP stream.
        // Lock seen_outbound_connections for further processing.
        let mut seen_outbound_connections = self.seen_outbound_connections.write().await;

        // Ensure the node respects the connection frequency limit.
        let last_seen = seen_outbound_connections.entry(peer_ip).or_insert(SystemTime::UNIX_EPOCH);
        let elapsed = last_seen.elapsed().unwrap_or(Duration::MAX).as_secs();
        if elapsed < E::RADIO_SILENCE_IN_SECS {
            bail!("Skipping connection request to {} (tried {} secs ago)", peer_ip, elapsed);
        }

        debug!("Connecting to {}...", peer_ip);
        // Update the last seen timestamp for this peer.
        seen_outbound_connections.insert(peer_ip, SystemTime::now());

        // Release the lock over seen_outbound_connections.
        drop(seen_outbound_connections);

        // Initialize the peer handler.
        // TODO: split into functions, maybe in encapsulate in peer?
        match timeout(Duration::from_millis(E::CONNECTION_TIMEOUT_IN_MILLIS), TcpStream::connect(peer_ip)).await {
            Ok(stream) => match stream {
                Ok(stream) => {
                    match Peer::new(
                        self.expect_network_state().clone(),
                        stream,
                        self.local_ip,
                        self.local_nonce,
                        &self.connected_nonces().await,
                    )
                    .await
                    {
                        Ok(peer) => {
                            self.peers.write().await.insert(peer_ip, peer);
                        }
                        Err(error) => {
                            trace!("{}", error);
                        }
                    };
                }
                Err(error) => {
                    trace!("Failed to connect to '{}': '{:?}'", peer_ip, error);
                    self.candidate_peers.write().await.remove(&peer_ip);
                }
            },
            Err(error) => {
                error!("Unable to reach '{}': '{:?}'", peer_ip, error);
                self.candidate_peers.write().await.remove(&peer_ip);
            }
        };

        Ok(())
    }

    pub async fn heartbeat(&self) {
        // Obtain the number of connected peers.
        let number_of_connected_peers = self.number_of_connected_peers().await;
        // Ensure the number of connected peers is below the maximum threshold.
        if number_of_connected_peers > E::MAXIMUM_NUMBER_OF_PEERS {
            debug!("Exceeded maximum number of connected peers");

            // Determine the peers to disconnect from.
            let num_excess_peers = number_of_connected_peers.saturating_sub(E::MAXIMUM_NUMBER_OF_PEERS);
            let peer_ips_to_disconnect = self
                .connected_peers
                .read()
                .await
                .iter()
                .filter(|(peer_ip, _)| {
                    !E::sync_nodes().contains(peer_ip) && !E::beacon_nodes().contains(peer_ip) && !E::trusted_nodes().contains(peer_ip)
                })
                .take(num_excess_peers)
                .map(|(&peer_ip, _)| peer_ip)
                .collect::<Vec<SocketAddr>>();

            // Proceed to send disconnect requests to these peers.
            for peer_ip in peer_ips_to_disconnect {
                info!("Disconnecting from {} (exceeded maximum connections)", peer_ip);
                self.send(peer_ip, Message::Disconnect(DisconnectReason::TooManyPeers)).await;
                // Add an entry for this `Peer` in the restricted peers.
                self.restricted_peers.write().await.insert(peer_ip, Instant::now());
            }
        }

        // TODO (howardwu): This logic can be optimized and unified with the context around it.
        // Determine if the node is connected to more sync nodes than expected.
        let connected_sync_nodes = self.connected_sync_nodes().await;
        let number_of_connected_sync_nodes = connected_sync_nodes.len();
        let num_excess_sync_nodes = number_of_connected_sync_nodes.saturating_sub(1);
        if num_excess_sync_nodes > 0 {
            debug!("Exceeded maximum number of sync nodes");

            // Proceed to send disconnect requests to these peers.
            for peer_ip in connected_sync_nodes
                .iter()
                .copied()
                .choose_multiple(&mut OsRng::default(), num_excess_sync_nodes)
            {
                info!("Disconnecting from {} (exceeded maximum connections)", peer_ip);
                self.send(peer_ip, Message::Disconnect(DisconnectReason::TooManyPeers)).await;
                // Add an entry for this `Peer` in the restricted peers.
                self.restricted_peers.write().await.insert(peer_ip, Instant::now());
            }
        }

        // Ensure that the trusted nodes are connected.
        if !E::trusted_nodes().is_empty() {
            let connected_peers = self.connected_peers().await.into_iter().collect::<HashSet<_>>();
            let trusted_nodes = E::trusted_nodes();
            let disconnected_trusted_nodes = trusted_nodes.difference(&connected_peers).copied();

            for peer_ip in disconnected_trusted_nodes {
                // Initialize the connection process.
                let _ = self.connect(peer_ip).await;
            }
        }

        // Skip if the number of connected peers is above the minimum threshold.
        match number_of_connected_peers < E::MINIMUM_NUMBER_OF_PEERS {
            true => {
                trace!("Sending request for more peer connections");
                // Request more peers if the number of connected peers is below the threshold.
                for peer_ip in self.connected_peers().await.iter().choose_multiple(&mut OsRng::default(), 3) {
                    self.send(*peer_ip, Message::PeerRequest).await;
                }
            }
            false => return,
        };

        // Add the sync nodes to the list of candidate peers.
        if number_of_connected_sync_nodes == 0 {
            self.add_candidate_peers(E::sync_nodes().iter()).await;
        }

        // Add the beacon nodes to the list of candidate peers.
        self.add_candidate_peers(E::beacon_nodes().iter()).await;

        // Attempt to connect to more peers if the number of connected peers is below the minimum threshold.
        // Select the peers randomly from the list of candidate peers.
        let midpoint_number_of_peers = E::MINIMUM_NUMBER_OF_PEERS.saturating_add(E::MAXIMUM_NUMBER_OF_PEERS) / 2;
        for peer_ip in self
            .candidate_peers()
            .await
            .iter()
            .copied()
            .choose_multiple(&mut OsRng::default(), midpoint_number_of_peers)
        {
            // Ensure this node is not connected to more than the permitted number of sync nodes.
            if E::sync_nodes().contains(&peer_ip) && number_of_connected_sync_nodes >= 1 {
                continue;
            }

            if !self.is_connected_to(peer_ip).await {
                trace!("Attempting connection to {}...", peer_ip);

                let _ = self.connect(peer_ip).await;
            }
        }
    }

    pub async fn peer_connecting(&self, stream: TcpStream, peer_ip: SocketAddr) {
        if let Err(error) = self.validate_connection(peer_ip).await {
            debug!("{}", error);

            return;
        }

        // Sanitize the port from the peer, if it is a remote IP address.
        let (peer_lookup, peer_port) = match peer_ip.ip().is_loopback() {
            // Loopback case - Do not sanitize, merely pass through.
            true => (peer_ip, peer_ip.port()),
            // Remote case - Sanitize, storing u16::MAX for the peer IP address to dedup the peer next time.
            false => (SocketAddr::new(peer_ip.ip(), u16::MAX), peer_ip.port()),
        };

        // Lock seen_inbound_connections for further processing.
        let mut seen_inbound_connections = self.seen_inbound_connections.write().await;

        // Fetch the inbound tracker entry for this peer.
        let ((initial_port, num_attempts), last_seen) = seen_inbound_connections
            .entry(peer_lookup)
            .or_insert(((peer_port, 0), SystemTime::UNIX_EPOCH));
        let elapsed = last_seen.elapsed().unwrap_or(Duration::MAX).as_secs();

        // Reset the inbound tracker entry for this peer, if the predefined elapsed time has passed.
        if elapsed > E::RADIO_SILENCE_IN_SECS {
            // Reset the initial port for this peer.
            *initial_port = peer_port;
            // Reset the number of attempts for this peer.
            *num_attempts = 0;
            // Reset the last seen timestamp for this peer.
            *last_seen = SystemTime::now();
        }

        // Ensure the connecting peer has not surpassed the connection attempt limit.
        if *initial_port < peer_port && *num_attempts > E::MAXIMUM_CONNECTION_FAILURES {
            trace!("Dropping connection request from {} (tried {} secs ago)", peer_ip, elapsed);
            // Add an entry for this `Peer` in the restricted peers.
            self.restricted_peers.write().await.insert(peer_ip, Instant::now());
        } else {
            debug!("Received a connection request from {}", peer_ip);
            // Update the number of attempts for this peer.
            *num_attempts += 1;

            // Release the lock over seen_inbound_connections.
            drop(seen_inbound_connections);

            match Peer::new(
                self.expect_network_state().clone(),
                stream,
                self.local_ip,
                self.local_nonce,
                &self.connected_nonces().await,
            )
            .await
            {
                Ok(peer) => {
                    self.peers.write().await.insert(peer_ip, peer);
                }
                Err(error) => {
                    trace!("{}", error);
                }
            };
        }
    }

    pub async fn peer_connected(&self, peer_ip: SocketAddr, peer_nonce: u64) {
        // Add an entry for this `Peer` in the connected peers.
        self.connected_peers.write().await.insert(peer_ip, peer_nonce);
        // Remove an entry for this `Peer` in the candidate peers, if it exists.
        self.candidate_peers.write().await.remove(&peer_ip);

        #[cfg(any(feature = "test", feature = "prometheus"))]
        {
            let number_of_connected_peers = self.number_of_connected_peers().await;
            let number_of_candidate_peers = self.number_of_candidate_peers().await;
            metrics::gauge!(metrics::peers::CONNECTED, number_of_connected_peers as f64);
            metrics::gauge!(metrics::peers::CANDIDATE, number_of_candidate_peers as f64);
        }
    }

    pub async fn peer_disconnected(&self, peer_ip: SocketAddr) {
        // Remove an entry for this `Peer` in the connected peers, if it exists.
        self.connected_peers.write().await.remove(&peer_ip);
        // Add an entry for this `Peer` in the candidate peers.
        self.candidate_peers.write().await.insert(peer_ip);

        #[cfg(any(feature = "test", feature = "prometheus"))]
        {
            let number_of_connected_peers = self.number_of_connected_peers().await;
            let number_of_candidate_peers = self.number_of_candidate_peers().await;
            metrics::gauge!(metrics::peers::CONNECTED, number_of_connected_peers as f64);
            metrics::gauge!(metrics::peers::CANDIDATE, number_of_candidate_peers as f64);
        }
    }

    pub async fn peer_restricted(&self, peer_ip: SocketAddr) {
        // Remove an entry for this `Peer` in the connected peers, if it exists.
        self.connected_peers.write().await.remove(&peer_ip);
        // Add an entry for this `Peer` in the restricted peers.
        self.restricted_peers.write().await.insert(peer_ip, Instant::now());

        #[cfg(any(feature = "test", feature = "prometheus"))]
        {
            let number_of_connected_peers = self.number_of_connected_peers().await;
            let number_of_restricted_peers = self.number_of_restricted_peers().await;
            metrics::gauge!(metrics::peers::CONNECTED, number_of_connected_peers as f64);
            metrics::gauge!(metrics::peers::RESTRICTED, number_of_restricted_peers as f64);
        }
    }

    pub async fn send_peer_response(&self, recipient: SocketAddr) {
        // Send a `PeerResponse` message.
        let connected_peers = self.connected_peers().await;
        self.send(recipient, Message::PeerResponse(connected_peers)).await;
    }

    pub async fn receive_peer_response(&self, peer_ips: Vec<SocketAddr>) {
        self.add_candidate_peers(peer_ips.iter()).await;

        #[cfg(any(feature = "test", feature = "prometheus"))]
        {
            let number_of_candidate_peers = self.number_of_candidate_peers().await;
            metrics::gauge!(metrics::peers::CANDIDATE, number_of_candidate_peers as f64);
        }
    }

    ///
    /// Adds the given peer IPs to the set of candidate peers.
    ///
    /// This method skips adding any given peers if the combined size exceeds the threshold,
    /// as the peer providing this list could be subverting the protocol.
    ///
    async fn add_candidate_peers<'a, T: ExactSizeIterator<Item = &'a SocketAddr> + IntoIterator>(&self, peers: T) {
        // Acquire the candidate peers write lock.
        let mut candidate_peers = self.candidate_peers.write().await;
        // Ensure the combined number of peers does not surpass the threshold.
        for peer_ip in peers.take(E::MAXIMUM_CANDIDATE_PEERS.saturating_sub(candidate_peers.len())) {
            // Ensure the peer is not self and is a new candidate peer.
            let is_self = *peer_ip == self.local_ip
                || (peer_ip.ip().is_unspecified() || peer_ip.ip().is_loopback()) && peer_ip.port() == self.local_ip.port();
            if !is_self && !self.is_connected_to(*peer_ip).await {
                // Proceed to insert each new candidate peer IP.
                candidate_peers.insert(*peer_ip);
            }
        }
    }

    ///
    /// Sends the given message to specified peer.
    ///
    pub async fn send(&self, peer_addr: SocketAddr, message: Message<N, E>) {
        let target_addr = self.connected_peers.read().await.get(&peer_addr).cloned();
        let peer = self.peers.read().await.get(&peer_addr).cloned();

        match (target_addr, peer) {
            // TODO: clean this up.
            (Some(_), Some(peer)) => {
                if let Err(error) = peer.outbound_sender.send(message).await {
                    trace!("Message sending failed: {}", error);
                    self.connected_peers.write().await.remove(&peer_addr);

                    #[cfg(any(feature = "test", feature = "prometheus"))]
                    {
                        let number_of_connected_peers = self.number_of_connected_peers().await;
                        metrics::gauge!(metrics::peers::CONNECTED, number_of_connected_peers as f64);
                    }
                }
            }
            _ => warn!("Attempted to send to a non-connected peer {}", peer_addr),
        }
    }

    ///
    /// Sends the given message to every connected peer, excluding the sender.
    ///
    pub async fn propagate(&self, sender: SocketAddr, mut message: Message<N, E>) {
        // Perform ahead-of-time, non-blocking serialization just once for applicable objects.
        if let Message::UnconfirmedBlock(_, _, ref mut data) = message {
            let serialized_block = Data::serialize(data.clone()).await.expect("Block serialization is bugged");
            let _ = std::mem::replace(data, Data::Buffer(serialized_block));
        }

        // Iterate through all peers that are not the sender, sync node, or beacon node.
        for peer in self
            .connected_peers()
            .await
            .iter()
            .filter(|peer_ip| *peer_ip != &sender && !E::sync_nodes().contains(peer_ip) && !E::beacon_nodes().contains(peer_ip))
            .copied()
            .collect::<Vec<_>>()
        {
            self.send(peer, message.clone()).await;
        }
    }

    ///
    /// Removes the addresses of all known peers.
    ///
    #[cfg(feature = "test")]
    pub async fn reset_known_peers(&self) {
        self.candidate_peers.write().await.clear();
        self.restricted_peers.write().await.clear();
        self.seen_inbound_connections.write().await.clear();
        self.seen_outbound_connections.write().await.clear();
    }
}
