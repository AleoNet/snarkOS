// Copyright (C) 2019-2020 Aleo Systems Inc.
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

use crate::{
    external::{
        message_types::{GetPeers, Version},
        Channel,
        Handshakes,
        PingPongManager,
    },
    internal::{
        context::{Connections, Context},
        PeerBook,
        PeerInfo,
    },
};
use snarkos_consensus::MerkleTreeLedger;

use chrono::Utc;
use std::{collections::HashMap, net::SocketAddr, sync::Arc};
use tokio::sync::RwLock;

pub struct ConnectionManager {
    /// The address of the node.
    local_address: SocketAddr,
    /// The set of connected and disconnected peers to the node.
    peer_book: Arc<RwLock<PeerBook>>,
    /// The ledger storage of the node.
    storage: Arc<MerkleTreeLedger>,
    /// The channels of the peers that the node is connected to.
    channels: HashMap<SocketAddr, Arc<Channel>>,
    /// TODO (howardwu): Remove this.
    /// The handshakes with connected peers
    handshakes: Arc<RwLock<Handshakes>>,
    /// The ping pong manager for the node.
    ping_pong: Arc<RwLock<PingPongManager>>,
    /// The minimum number of peers the node should be connected to.
    minimum_peer_count: u16,
    /// The frequency that the manager should run the handler.
    connection_frequency: u64,

    tmp_connections: Arc<RwLock<Connections>>,
}

impl ConnectionManager {
    /// Creates a new instance of a `ConnectionManager`.
    #[inline]
    pub fn new(
        context: &Arc<Context>,
        local_address: SocketAddr,
        connections: Arc<RwLock<Connections>>,
        storage: &Arc<MerkleTreeLedger>,
        connection_frequency: u64,
    ) -> Self {
        Self {
            local_address,
            peer_book: context.peer_book.clone(),
            storage: storage.clone(),
            channels: HashMap::new(),
            handshakes: context.handshakes.clone(),
            ping_pong: context.pings.clone(),
            minimum_peer_count: context.min_peers,
            connection_frequency,

            tmp_connections: connections,
        }
    }

    /// Returns the local address of the node.
    #[inline]
    pub fn local_address(&self) -> SocketAddr {
        self.local_address
    }

    /// Returns the number of peers connected to the node.
    #[inline]
    pub async fn get_num_connected(&self) -> u16 {
        // Acquire the peer book reader.
        let peer_book = self.peer_book.read().await;
        // Fetch the number of connected peers.
        let num_connected_peers = peer_book.num_connected();
        // Drop the peer book reader.
        drop(peer_book);
        num_connected_peers
    }

    /// Returns the connected peers of the node.
    #[inline]
    pub async fn get_all_connected(&self) -> HashMap<SocketAddr, PeerInfo> {
        // Acquire the peer book reader.
        let peer_book = self.peer_book.read().await;
        // Get a clone of the connected peers of the node.
        let connected_peers = peer_book.get_all_connected().clone();
        // Drop the peer book reader.
        drop(peer_book);
        connected_peers
    }

    /// Returns the disconnected peers of the node.
    #[inline]
    pub async fn get_all_disconnected(&self) -> HashMap<SocketAddr, PeerInfo> {
        // Acquire the peer book reader.
        let peer_book = self.peer_book.read().await;
        // Get a clone of the disconnected peers of the node.
        let disconnected_peers = peer_book.get_all_disconnected().clone();
        // Drop the peer book reader.
        drop(peer_book);
        disconnected_peers
    }

    /// TODO (howardwu): Remove the active channels and handshakes of the peer from this struct.
    /// Disconnects the given address from the node.
    #[inline]
    pub async fn disconnect_from_peer(&self, remote_address: &SocketAddr) -> bool {
        // Acquire the peer book write lock.
        let mut peer_book = self.peer_book.write().await;
        // Disconnected from the peer.
        let is_disconnected = peer_book.disconnected_peer(remote_address);
        // Drop the peer book write lock.
        drop(peer_book);
        is_disconnected
    }

    /// Attempts to get the channel for a given address.
    /// Returns `Some(channel)` if the address is a connected peer.
    /// Otherwise, returns `None`.
    #[inline]
    pub async fn get_channel(&self, remote_address: &SocketAddr) -> Option<Arc<Channel>> {
        // TODO (howardwu): Remove this logic in favor of the below calling convention.
        //  This is a temporary solution as part of a much larger refactor.
        // self.channels.get(remote_address)

        // Acquire the tmp_connections read lock.
        let tmp_connections = self.tmp_connections.read().await;
        // Forget the local address.
        let channel = tmp_connections.get(remote_address);
        // Drop the tmp_connections read lock.
        drop(tmp_connections);
        channel
    }

    /// Manages all peer connections and processes updates with all connected peers.
    #[inline]
    pub async fn handler(&self) {
        // Refresh the internal state of the peer book.
        self.refresh_peer_book().await;

        // If the node is connected to less peers than the minimum required,
        // ask every peer the node is connected to for more peers.
        if self.get_num_connected().await < self.minimum_peer_count {
            // Broadcast a `GetPeers` message to request for more peers.
            self.broadcast_getpeers_requests().await;
            // Attempt a handshake connection with every disconnected peer.
            self.connect_to_all_disconnected_peers().await;
        }

        // TODO (howardwu): Unify `Ping` and `Version`requests.
        //  This is a remnant and these currently do not need to be distinct.

        // Broadcast a `Ping` request to each connected peer.
        self.broadcast_ping_requests().await;

        // Broadcast a `Version` request to each connected peer.
        self.broadcast_version_requests().await;

        // Store the internal state of the peer book.
        self.store_peer_book().await;
    }

    /// An internal operation to update the internal state of the peer book.
    #[inline]
    async fn refresh_peer_book(&self) {
        // [This is a redundant check for added safety]
        // Remove the local_address from the peer book
        // in case the node found itself as a peer.
        {
            // Acquire the peer book write lock.
            let mut peer_book = self.peer_book.write().await;
            // Forget the local address.
            peer_book.forget_peer(self.local_address());
            // Drop the peer book write lock.
            drop(peer_book);
        }
    }

    /// An internal operation to write the internal state of the peer book to storage.
    #[inline]
    async fn store_peer_book(&self) {
        // Acquire the peer book reader.
        let peer_book = self.peer_book.read().await;
        // Store the peer book into the database.
        peer_book
            .store(&self.storage)
            .unwrap_or_else(|error| debug!("Failed to store connected peers in database {}", error));
        // Drop the peer book reader.
        drop(peer_book);
    }

    /// Broadcasts a `Version` message to all connected peers.
    #[inline]
    async fn broadcast_version_requests(&self) {
        // Get the local address.
        let local_address = self.local_address();

        // Broadcast a `Version` message to each connected peer for a periodic sync.
        if self.get_num_connected().await > 0 {
            debug!("Sending out periodic version message to peers");

            // Send a `Version` message to every connected peer of the node.
            for (remote_address, _) in self.get_all_connected().await {
                if let Some(channel) = self.get_channel(&remote_address).await {
                    // Acquire the handshakes read lock.
                    let handshakes = self.handshakes.read().await;
                    if let Some(handshake) = handshakes.get(&remote_address) {
                        // Send a version message to peers.
                        // If they are behind, they will attempt to sync.
                        let version = Version::from(
                            1u64, // TODO (raychu86) Establish a formal node version
                            self.storage.get_current_block_height(),
                            remote_address,
                            local_address,
                            handshake.nonce,
                        );
                        // Disconnect from the peer if the version request fails to send.
                        if let Err(_) = channel.write(&version).await {
                            self.disconnect_from_peer(&remote_address).await;
                        }
                    }
                    // Drop the handshakes read lock.
                    drop(handshakes);
                } else {
                    // Disconnect from the peer if there is no active connection channel
                    self.disconnect_from_peer(&remote_address).await;
                }
            }
        }
    }

    /// Broadcasts a `Ping` message to all connected peers.
    #[inline]
    async fn broadcast_ping_requests(&self) {
        // Broadcast a `Ping` request to every connected peers
        // that the node hasn't heard from in a while.
        for (remote_address, peer_info) in self.get_all_connected().await {
            // Calculate the time since we last saw the peer.
            let elapsed_in_millis = (Utc::now() - *peer_info.last_seen()).num_milliseconds();
            if elapsed_in_millis.is_positive() && elapsed_in_millis as u64 > (self.connection_frequency * 3) {
                if let Some(channel) = self.get_channel(&remote_address).await {
                    // Acquire the ping pong manager write lock.
                    let mut ping_pong = self.ping_pong.write().await;
                    // Send a ping to the remote address.
                    if let Err(_) = ping_pong.send_ping(&channel).await {
                        warn!("Ping message failed to send to {}", remote_address);
                        // Disconnect from the peer if the ping request fails to send.
                        self.disconnect_from_peer(&remote_address).await;
                    }
                    // Drop the ping pong manager write lock.
                    drop(ping_pong);
                } else {
                    // Disconnect from the peer if the channel is not active.
                    self.disconnect_from_peer(&remote_address).await;
                }
            }
        }
    }

    /// Broadcasts a `GetPeers` message to all connected peers to request for more peers.
    #[inline]
    async fn broadcast_getpeers_requests(&self) {
        // Iterate through each connected peer and broadcast a `GetPeers` message.
        for (remote_address, _) in self.get_all_connected().await {
            // Fetch the connection channel.
            if let Some(channel) = self.get_channel(&remote_address).await {
                // Broadcast the message over the channel.
                if let Err(_) = channel.write(&GetPeers).await {
                    // Disconnect from the peer if the message fails to send.
                    self.disconnect_from_peer(&remote_address).await;
                }
            } else {
                // Disconnect from the peer if the channel is not active.
                self.disconnect_from_peer(&remote_address).await;
            }
        }
    }

    /// Attempts a handshake connection with every disconnected peer.
    #[inline]
    async fn connect_to_all_disconnected_peers(&self) {
        // Get the local address.
        let local_address = self.local_address();
        // Get the current block height.
        let block_height = self.storage.get_current_block_height();

        // Iterate through each connected peer and attempts a handshake request.
        for (remote_address, _) in self.get_all_disconnected().await {
            // Create a version message.
            // TODO (raychu86) Establish a formal node version
            let version = Version::new(1u64, block_height, remote_address, local_address);

            // Acquire the handshake write lock.
            let mut handshakes = self.handshakes.write().await;
            // Attempt a handshake with the remote address.
            if let Err(_) = handshakes.send_request(&version).await {
                debug!("Unable to establish a handshake with peer ({})", remote_address);
            }
            // Drop the handshake write lock.
            drop(handshakes)
        }
    }
}
