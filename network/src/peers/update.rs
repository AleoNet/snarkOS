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

use super::*;

impl<N: Network, E: Environment> Peers<N, E> {
    ///
    /// Performs the given `request` to the peers.
    /// All requests must go through this `update`, so that a unified view is preserved.
    ///
    pub(crate) async fn update(&self, request: PeersRequest<N, E>) {
        debug!("Peers: {:?}", self.connected_peers().await);

        match request {
            PeersRequest::Connect(peer_ip, connection_result) => {
                // Ensure the peer IP is not this node.
                if self.state.is_local_ip(&peer_ip) {
                    debug!("Skipping connection request to {} (attempted to self-connect)", peer_ip);
                }
                // Ensure the node does not surpass the maximum number of peer connections.
                else if self.number_of_connected_peers().await >= E::MAXIMUM_NUMBER_OF_PEERS {
                    debug!("Skipping connection request to {} (maximum peers reached)", peer_ip);
                }
                // Ensure the peer is a new connection.
                else if self.is_connected_to(peer_ip).await {
                    debug!("Skipping connection request to {} (already connected)", peer_ip);
                }
                // Ensure the peer is not restricted.
                else if self.is_restricted(peer_ip).await {
                    debug!("Skipping connection request to {} (restricted)", peer_ip);
                }
                // Attempt to open a TCP stream.
                else {
                    // Lock seen_outbound_connections for further processing.
                    let mut seen_outbound_connections = self.seen_outbound_connections.write().await;

                    // Ensure the node respects the connection frequency limit.
                    let last_seen = seen_outbound_connections.entry(peer_ip).or_insert(SystemTime::UNIX_EPOCH);
                    let elapsed = last_seen.elapsed().unwrap_or(Duration::MAX).as_secs();
                    if elapsed < E::RADIO_SILENCE_IN_SECS {
                        trace!("Skipping connection request to {} (tried {} secs ago)", peer_ip, elapsed);
                    } else {
                        debug!("Connecting to {}...", peer_ip);
                        // Update the last seen timestamp for this peer.
                        seen_outbound_connections.insert(peer_ip, SystemTime::now());

                        // Release the lock over seen_outbound_connections.
                        drop(seen_outbound_connections);

                        // Initialize the peer.
                        match timeout(Duration::from_millis(E::CONNECTION_TIMEOUT_IN_MILLIS), TcpStream::connect(peer_ip)).await {
                            Ok(stream) => match stream {
                                Ok(stream) => Peer::handshake(self.state.clone(), stream, Some(connection_result)).await,
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
                    }
                }
            }
            PeersRequest::Heartbeat => {
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
                        .keys()
                        .filter(|peer_ip| !E::beacon_nodes().contains(peer_ip) && !E::trusted_nodes().contains(peer_ip))
                        .take(num_excess_peers)
                        .copied()
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
                let connected_beacon_nodes = self.connected_beacon_nodes().await;
                let number_of_connected_beacon_nodes = connected_beacon_nodes.len();
                let num_excess_beacon_nodes = number_of_connected_beacon_nodes.saturating_sub(1);
                if num_excess_beacon_nodes > 0 {
                    debug!("Exceeded maximum number of sync nodes");

                    // Proceed to send disconnect requests to these peers.
                    for peer_ip in connected_beacon_nodes
                        .iter()
                        .copied()
                        .choose_multiple(&mut OsRng::default(), num_excess_beacon_nodes)
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
                        let (router, handler) = oneshot::channel();
                        let request = PeersRequest::Connect(peer_ip, router);
                        if let Err(error) = self.peers_router.send(request).await {
                            warn!("Failed to transmit the request: '{}'", error);
                        }

                        // Do not wait for the result of each connection.
                        // Procure a resource id to register the task with, as it might be terminated at any point in time.
                        let resource_id = E::resources().procure_id();
                        E::resources().register_task(
                            Some(resource_id),
                            task::spawn(async move {
                                let _ = handler.await;

                                E::resources().deregister(resource_id);
                            }),
                        );
                    }
                }

                // Skip if the number of connected peers is above the minimum threshold.
                match number_of_connected_peers < E::MINIMUM_NUMBER_OF_PEERS {
                    true => {
                        if number_of_connected_peers > 0 {
                            trace!("Sending requests for more peer connections");
                            // Request more peers if the number of connected peers is below the threshold.
                            for peer_ip in self.connected_peers().await.iter().choose_multiple(&mut OsRng::default(), 3) {
                                self.send(*peer_ip, Message::PeerRequest).await;
                            }
                        }
                    }
                    false => return,
                };

                // Add the sync nodes to the list of candidate peers.
                if number_of_connected_beacon_nodes == 0 {
                    self.add_candidate_peers(E::beacon_nodes().iter()).await;
                }

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
                    if E::beacon_nodes().contains(&peer_ip) && number_of_connected_beacon_nodes >= 1 {
                        continue;
                    }

                    if !self.is_connected_to(peer_ip).await {
                        trace!("Attempting connection to {}...", peer_ip);

                        // Initialize the connection process.
                        let (router, handler) = oneshot::channel();
                        let request = PeersRequest::Connect(peer_ip, router);
                        if let Err(error) = self.peers_router.send(request).await {
                            warn!("Failed to transmit the request: '{}'", error);
                        }
                        // Do not wait for the result of each connection.
                        // Procure a resource id to register the task with, as it might be terminated at any point in time.
                        let resource_id = E::resources().procure_id();
                        E::resources().register_task(
                            Some(resource_id),
                            task::spawn(async move {
                                let _ = handler.await;

                                E::resources().deregister(resource_id);
                            }),
                        );
                    }
                }
            }
            PeersRequest::MessagePropagate(sender, message) => self.propagate(sender, message).await,
            PeersRequest::MessageSend(sender, message) => self.send(sender, message).await,
            PeersRequest::PeerConnecting(stream, peer_ip) => {
                // Ensure the peer IP is not this node.
                if self.state.is_local_ip(&peer_ip) {
                    debug!("Skipping connection request to {} (attempted to self-connect)", peer_ip);
                }
                // Ensure the node does not surpass the maximum number of peer connections.
                else if self.number_of_connected_peers().await >= E::MAXIMUM_NUMBER_OF_PEERS {
                    debug!("Dropping connection request from {} (maximum peers reached)", peer_ip);
                }
                // Ensure the node is not already connected to this peer.
                else if self.is_connected_to(peer_ip).await {
                    debug!("Dropping connection request from {} (already connected)", peer_ip);
                }
                // Ensure the peer is not restricted.
                else if self.is_restricted(peer_ip).await {
                    debug!("Dropping connection request from {} (restricted)", peer_ip);
                }
                // Spawn a handler to be run asynchronously.
                else {
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
                    if *num_attempts > E::MAXIMUM_CONNECTION_FAILURES {
                        trace!("Dropping connection request from {} (tried {} secs ago)", peer_ip, elapsed);
                        // Add an entry for this `Peer` in the restricted peers.
                        self.restricted_peers.write().await.insert(peer_ip, Instant::now());
                    } else {
                        debug!("Received a connection request from {}", peer_ip);
                        // Update the number of attempts for this peer.
                        *num_attempts += 1;

                        // Release the lock over seen_inbound_connections.
                        drop(seen_inbound_connections);

                        // Initialize the peer handler.
                        Peer::handshake(self.state.clone(), stream, None).await;
                    }
                }
            }
            PeersRequest::PeerConnected(peer_ip, peer) => {
                // Add an entry for this `Peer` in the connected peers.
                self.connected_peers.write().await.insert(peer_ip, peer);
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
            PeersRequest::PeerDisconnected(peer_ip) => {
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
            PeersRequest::PeerRestricted(peer_ip) => {
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
            PeersRequest::SendPeerResponse(recipient, rtt_start) => {
                // Send a `PeerResponse` message.
                let connected_peers = self.connected_peers().await;
                self.send(recipient, Message::PeerResponse(connected_peers, rtt_start)).await;
            }
            PeersRequest::ReceivePeerResponse(peer_ips) => {
                self.add_candidate_peers(peer_ips.iter()).await;

                #[cfg(any(feature = "test", feature = "prometheus"))]
                {
                    let number_of_candidate_peers = self.number_of_candidate_peers().await;
                    metrics::gauge!(metrics::peers::CANDIDATE, number_of_candidate_peers as f64);
                }
            }
        }
    }

    /// Adds the given peer IPs to the set of candidate peers.
    ///
    /// This method skips adding any given peers if the combined size exceeds the threshold,
    /// as the peer providing this list could be subverting the protocol.
    async fn add_candidate_peers<'a, T: ExactSizeIterator<Item = &'a SocketAddr> + IntoIterator>(&self, peers: T) {
        // Acquire the candidate peers write lock.
        let mut candidate_peers = self.candidate_peers.write().await;
        // Ensure the combined number of peers does not surpass the threshold.
        for peer_ip in peers.take(E::MAXIMUM_CANDIDATE_PEERS.saturating_sub(candidate_peers.len())) {
            // Ensure the peer is not itself and is a new candidate peer.
            if !self.state.is_local_ip(peer_ip) && !self.is_connected_to(*peer_ip).await {
                // Proceed to insert each new candidate peer IP.
                candidate_peers.insert(*peer_ip);
            }
        }
    }

    /// Sends the given message to specified peer.
    async fn send(&self, peer_ip: SocketAddr, message: Message<N>) {
        let target_peer = self.connected_peers.read().await.get(&peer_ip).cloned();
        match target_peer {
            Some(peer) => {
                if let Err(error) = peer.send(message).await {
                    trace!("Outbound channel failed: {}", error);
                    self.connected_peers.write().await.remove(&peer_ip);

                    #[cfg(any(feature = "test", feature = "prometheus"))]
                    {
                        let number_of_connected_peers = self.number_of_connected_peers().await;
                        metrics::gauge!(metrics::peers::CONNECTED, number_of_connected_peers as f64);
                    }
                }
            }
            None => warn!("Attempted to send to a non-connected peer {peer_ip}"),
        }
    }

    /// Sends the given message to every connected peer, excluding the sender.
    async fn propagate(&self, sender: SocketAddr, mut message: Message<N>) {
        // Perform ahead-of-time, non-blocking serialization just once for applicable objects.
        if let Message::UnconfirmedBlock(_, _, ref mut data) = message {
            let serialized_block = Data::serialize(data.clone()).await.expect("Block serialization is bugged");
            let _ = std::mem::replace(data, Data::Buffer(serialized_block));
        }

        // Iterate through all peers that are not the sender or a beacon node.
        for peer in self
            .connected_peers()
            .await
            .iter()
            .filter(|peer_ip| *peer_ip != &sender && !E::beacon_nodes().contains(peer_ip))
        {
            self.send(*peer, message.clone()).await;
        }
    }

    /// Removes the addresses of all known peers.
    pub async fn reset_known_peers(&self) {
        self.candidate_peers.write().await.clear();
        self.restricted_peers.write().await.clear();
        self.seen_inbound_connections.write().await.clear();
        self.seen_outbound_connections.write().await.clear();
    }
}
