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

use std::{cmp, net::SocketAddr, time::Duration};

use rand::seq::IteratorRandom;
use snarkvm_dpc::Storage;
use tokio::task;

use snarkos_metrics::{self as metrics, connections::*};

use crate::{message::*, KnownNetworkMessage, NetworkError, Node};

impl<S: Storage + core::marker::Sync + Send> Node<S> {
    /// Obtain a list of addresses of connected peers for this node.
    pub(crate) fn connected_peers(&self) -> Vec<SocketAddr> {
        self.peer_book.connected_peers()
    }
}

impl<S: Storage + Send + Sync + 'static> Node<S> {
    ///
    /// Broadcasts updates with connected peers and maintains a permitted number of connected peers.
    ///
    pub(crate) async fn update_peers(&self) {
        // Fetch the number of connected and connecting peers.
        let active_peer_count = self.peer_book.get_active_peer_count();
        info!(
            "Connected to {} peer{}",
            active_peer_count,
            if active_peer_count == 1 { "" } else { "s" }
        );

        // Drop peers whose RTT is too high or have too many failures.
        self.peer_book.judge_peers().await;
        // give us 100ms to close some negatively judge_badd connections (probably less needed, but we have time)
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Fetch active peer count after high RTTs disconnects.
        let active_peer_count = self.peer_book.get_active_peer_count();
        let min_peers = self.config.minimum_number_of_connected_peers() as u32;
        let max_peers = self.config.maximum_number_of_connected_peers() as u32;

        // Calculate the peer counts to disconnect and connect based on the node type and current
        // peer counts.
        let (number_to_disconnect, number_to_connect) = match self.config.is_bootnode() {
            true => {
                // Bootnodes disconnect down to the min peer count, this to free up room for
                // the next crawled peers...
                let number_to_disconnect = active_peer_count.saturating_sub(min_peers);
                // ...then they connect to disconnected peers leaving 20% of their capacity open
                // incoming connections.
                const CRAWLING_CAPACITY_PERCENTAGE: f64 = 0.8;
                let crawling_capacity = (CRAWLING_CAPACITY_PERCENTAGE * max_peers as f64).floor() as u32;
                let number_to_connect = crawling_capacity.saturating_sub(active_peer_count - number_to_disconnect);

                (number_to_disconnect, number_to_connect)
            }
            false => (
                // Non-bootnodes disconnect if above the max peer count...
                active_peer_count.saturating_sub(max_peers),
                // ...and connect if below the min peer count.
                min_peers.saturating_sub(active_peer_count),
            ),
        };

        if number_to_disconnect != 0 {
            let mut current_peers = self.peer_book.connected_peers_snapshot().await;

            // Bootnodes will disconnect from random peers...
            if !self.config.is_bootnode() {
                // ...while regular peers from the most recently connected.
                current_peers.sort_unstable_by_key(|peer| peer.quality.last_connected);
            }

            for _ in 0..number_to_disconnect {
                if let Some(peer) = current_peers.pop() {
                    self.disconnect_from_peer(peer.address).await;
                }
            }
        }

        // Attempt to connect to the default bootnodes of the network if the node has no active
        // connections.
        if self.peer_book.get_active_peer_count() == 0 {
            self.connect_to_addresses(&self.config.bootnodes()).await;
        }

        if number_to_connect != 0 {
            self.connect_to_disconnected_peers(number_to_connect as usize).await;
        }

        // Only broadcast requests if any peers are connected.
        if self.peer_book.get_connected_peer_count() != 0 {
            // Broadcast a `GetPeers` message to request for more peers.
            self.broadcast_getpeers_requests().await;

            // Send a `Ping` to every connected peer.
            self.broadcast_pings().await;
        }
    }

    async fn initiate_connection(&self, remote_address: SocketAddr) -> Result<(), NetworkError> {
        debug!("Connecting to {}...", remote_address);

        // Local address must be known by now.
        let own_address = self.local_address().unwrap();

        // Don't connect if maximum number of connections has been reached.
        if !self.can_connect() {
            return Err(NetworkError::TooManyConnections);
        }

        if remote_address == own_address
            || ((remote_address.ip().is_unspecified() || remote_address.ip().is_loopback())
                && remote_address.port() == own_address.port())
        {
            return Err(NetworkError::SelfConnectAttempt);
        }
        if self.peer_book.is_connected(remote_address) {
            return Err(NetworkError::PeerAlreadyConnected);
        }

        metrics::increment_counter!(ALL_INITIATED);

        self.peer_book.get_or_connect(self.clone(), remote_address).await?;

        Ok(())
    }

    ///
    /// Broadcasts a connection request to all the supplied addresses.
    ///
    /// This function filters out any peers the node server is
    /// either connnecting to or already connected to.
    ///
    pub async fn connect_to_addresses(&self, addrs: &[SocketAddr]) {
        // Local address must be known by now.
        let own_address = self.local_address().unwrap();

        for node_addr in addrs.iter().filter(|addr| **addr != own_address).copied() {
            if self.peer_book.is_connected(node_addr) {
                break;
            }

            let node = self.clone();
            task::spawn(async move {
                match node.initiate_connection(node_addr).await {
                    Err(NetworkError::PeerAlreadyConnecting) | Err(NetworkError::PeerAlreadyConnected) => {
                        // no issue here, already connecting
                    }
                    Err(e @ NetworkError::TooManyConnections) => {
                        warn!("Couldn't connect to peer {}: {}", node_addr, e);
                        // the connection hasn't been established, no need to disconnect
                    }
                    Err(e) => {
                        warn!("Couldn't connect to peer {}: {}", node_addr, e);
                        node.disconnect_from_peer(node_addr).await;
                    }
                    Ok(_) => {}
                }
            });
        }
    }

    ///
    /// Broadcasts a connection request to all disconnected peers.
    ///
    async fn connect_to_disconnected_peers(&self, count: usize) {
        // Local address must be known by now.
        let own_address = self.local_address().unwrap();

        // If this node is not a bootnode, attempt to satisfy the minimum number of peer connections.
        let random_peers = {
            trace!(
                "Connecting to {} disconnected peers",
                cmp::min(count, self.peer_book.disconnected_peers().len())
            );

            let bootnodes = self.config.bootnodes();

            // Iterate through a selection of random peers and attempt to connect.
            let mut candidates = self.peer_book.disconnected_peers_snapshot();

            candidates.retain(|peer| peer.address != own_address && !bootnodes.contains(&peer.address));

            if self.config.is_bootnode() {
                // Bootnodes choose peers they haven't dialed in a while.
                candidates.sort_unstable_by_key(|peer| peer.quality.last_connected);
            }

            // Only keep the addresses.
            let addr_iter = candidates.iter().map(|peer| peer.address);

            if self.config.is_bootnode() {
                addr_iter.take(count).collect()
            } else {
                addr_iter.choose_multiple(&mut rand::thread_rng(), count)
            }
        };

        for remote_address in random_peers {
            let node = self.clone();
            task::spawn(async move {
                match node.initiate_connection(remote_address).await {
                    Err(NetworkError::PeerAlreadyConnecting) | Err(NetworkError::PeerAlreadyConnected) => {
                        // no issue here, already connecting
                    }
                    Err(e @ NetworkError::TooManyConnections) | Err(e @ NetworkError::SelfConnectAttempt) => {
                        warn!("Couldn't connect to peer {}: {}", remote_address, e);
                        // the connection hasn't been established, no need to disconnect
                    }
                    Err(e) => {
                        warn!("Couldn't connect to peer {}: {}", remote_address, e);
                        node.disconnect_from_peer(remote_address).await;
                    }
                    Ok(_) => {}
                }
            });
        }
    }

    /// Broadcasts a `GetPeers` message to all connected peers to request for more peers.
    async fn broadcast_getpeers_requests(&self) {
        // Check that this node is not a bootnode.
        if !self.config.is_bootnode() {
            // Fetch the number of connected and connecting peers.
            let number_of_peers = self.peer_book.get_active_peer_count() as usize;

            // Check if this node server is below the permitted number of connected peers.
            let min_peers = self.config.minimum_number_of_connected_peers() as usize;
            if number_of_peers >= min_peers {
                return;
            }
        }

        trace!("Sending `GetPeers` requests to connected peers");

        self.peer_book.broadcast(Payload::GetPeers).await;
    }

    /// Broadcasts a `Ping` message to all connected peers.
    async fn broadcast_pings(&self) {
        trace!("Broadcasting `Ping` messages");

        // Consider peering tests that don't use the sync layer.
        let current_block_height = if let Some(sync) = self.sync() {
            sync.current_block_height()
        } else {
            0
        };

        self.peer_book.broadcast(Payload::Ping(current_block_height)).await;
    }

    ///
    /// Removes the given remote address channel and sets the peer in the peer book
    /// as disconnected from this node server.
    ///
    #[inline]
    pub async fn disconnect_from_peer(&self, remote_address: SocketAddr) {
        if let Some(handle) = self.peer_book.get_peer_handle(remote_address) {
            if handle.disconnect().await {
                trace!("Disconnected from {}", remote_address);
            }
        }
    }

    pub(crate) async fn send_peers(&self, remote_address: SocketAddr) {
        // Broadcast the sanitized list of connected peers back to the requesting peer.

        use crate::Peer;
        use rand::prelude::SliceRandom;

        let connected_peers = self.peer_book.connected_peers_snapshot().await;

        let basic_filter =
            |peer: &Peer| peer.address != remote_address && !self.config.bootnodes().contains(&peer.address);
        let strict_filter = |peer: &Peer| basic_filter(peer) && peer.is_routable.unwrap_or(false);

        // Strictly filter the connected peers by only including the routable addresses.
        let strictly_filtered_peers: Vec<SocketAddr> = connected_peers
            .iter()
            .filter(|peer| strict_filter(peer))
            .map(|peer| peer.address)
            .collect();

        // Bootnodes apply less strict filtering rules if the set is empty by falling back on
        // connected peers that may or may not be routable...
        let peers = if self.config.is_bootnode() && strictly_filtered_peers.is_empty() {
            let filtered_peers: Vec<SocketAddr> = connected_peers
                .iter()
                .filter(|peer| basic_filter(peer))
                .map(|peer| peer.address)
                .collect();

            // ...and if need be on disconnected peers.
            if filtered_peers.is_empty() {
                self.peer_book
                    .disconnected_peers_snapshot()
                    .iter()
                    .filter(|peer| basic_filter(peer))
                    .map(|peer| peer.address)
                    .collect()
            } else {
                filtered_peers
            }
        } else {
            strictly_filtered_peers
        };

        // Limit set size.
        let peers = peers
            .choose_multiple(&mut rand::thread_rng(), crate::SHARED_PEER_COUNT)
            .copied()
            .collect();

        self.peer_book.send_to(remote_address, Payload::Peers(peers)).await;
    }

    /// A node has sent their list of peer addresses.
    /// Add all new/updated addresses to our disconnected.
    /// The connection handler will be responsible for sending out handshake requests to them.
    pub(crate) async fn process_inbound_peers(&self, source: SocketAddr, peers: Vec<SocketAddr>) {
        let local_address = self.local_address().unwrap(); // the address must be known by now

        for peer_address in peers.iter().filter(|&peer_addr| *peer_addr != local_address) {
            // Inform the peer book that we found a peer.
            // The peer book will determine if we have seen the peer before,
            // and include the peer if it is new.
            self.peer_book
                .add_peer(*peer_address, self.config.bootnodes().contains(peer_address))
                .await;
        }

        if let Some(known_network) = self.known_network() {
            // If this node is tracking the network, record the connections. This can
            // then be used to construct the graph and query peer info from the peerbook.
            let _ = known_network.sender.try_send(KnownNetworkMessage::Peers(source, peers));
        }
    }

    pub fn can_connect(&self) -> bool {
        let num_connected = self.peer_book.get_active_peer_count() as usize;

        let max_peers = self.config.maximum_number_of_connected_peers() as usize;

        if num_connected > max_peers {
            warn!(
                "Max number of connections ({} connected; max: {}) reached",
                num_connected, max_peers
            );
            false
        } else {
            true
        }
    }
}
