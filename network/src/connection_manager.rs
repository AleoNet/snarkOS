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
        PingPongManager,
    },
    internal::{context::Context, PeerBook, PeerInfo},
    RequestManager,
};
use snarkos_consensus::MerkleTreeLedger;

use chrono::Utc;
use std::{collections::HashMap, net::SocketAddr, sync::Arc};
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct ConnectionManager {
    /// The set of connected and disconnected peers to the node.
    peer_book: Arc<RwLock<PeerBook>>,
    /// The channels of the peers that the node is connected to.
    channels: HashMap<SocketAddr, Arc<Channel>>,
    /// The request manager of the node.
    request_manager: RequestManager,
    /// The ledger storage of the node.
    storage: Arc<MerkleTreeLedger>,
    /// TODO (howardwu): Remove this.
    /// The ping pong manager of the node.
    ping_pong: Arc<RwLock<PingPongManager>>,
    /// The default bootnode addresses of the network.
    bootnode_addresses: Vec<SocketAddr>,
    /// The minimum number of peers the node should be connected to.
    minimum_peer_count: u16,
    /// The frequency that the manager should run the handler.
    connection_frequency: u64,
}

impl ConnectionManager {
    ///
    /// Creates a new instance of a `ConnectionManager`.
    ///
    /// Initializes the `ConnectionManager` with the following steps.
    /// 1. Attempt to connect to all default bootnodes on the network.
    /// 2. Attempt to connect to all disconnected peers from the stored peer book.
    ///
    #[inline]
    pub async fn new(
        context: &Arc<Context>,
        request_manager: RequestManager,
        storage: &Arc<MerkleTreeLedger>,
        bootnode_addresses: Vec<SocketAddr>,
        connection_frequency: u64,
    ) -> Self {
        // Load the preliminary local address from context.
        let preliminary_local_address = context.local_address.read().await;

        // Load the peer book from storage.
        let peer_book = PeerBook::load(&storage).unwrap_or_else(|| {
            // If the load fails, either the peer book does not exist,
            // or it has been corrupted (e.g. from a data structure change).
            let local_address = format!("0.0.0.0:{}", preliminary_local_address.port())
                .parse()
                .expect("local address");
            let mut peer_book = PeerBook::new(local_address);
            // Store the new peer book into the database.
            peer_book
                .store(&storage)
                .unwrap_or_else(|error| debug!("Failed to store peer book into database {}", error));
            peer_book
        });
        drop(preliminary_local_address);

        // Instantiate a connection manager.
        let connection_manager = Self {
            peer_book: Arc::new(RwLock::new(peer_book)),
            request_manager,
            storage: storage.clone(),
            channels: HashMap::new(),
            ping_pong: context.pings.clone(),
            bootnode_addresses,
            minimum_peer_count: context.min_peers,
            connection_frequency,
        };

        // Initialize the connection manager.
        debug!("Initializing the connection manager...");
        {
            // 1. Attempt to connect to all default bootnodes on the network.
            connection_manager.connect_to_bootnodes().await;
            // 2. Attempt to connect to all disconnected peers from the stored peer book.
            if !context.is_bootnode {
                // Only attempt the connection if the node is not a bootnode.
                connection_manager.connect_to_all_disconnected_peers().await;
            }
        }
        // Completed initializing connection manager.
        debug!("Completed initializing connection manager.");

        connection_manager
    }

    /// Returns the local address of the node.
    #[inline]
    pub async fn get_local_address(&self) -> SocketAddr {
        // Acquire the peer book reader.
        let peer_book = self.peer_book.read().await;
        // Get the local address of the node.
        let local_address = peer_book.local_address().clone();
        // Drop the peer book reader.
        drop(peer_book);
        local_address
    }

    /// Updates the local address stored in the `PeerBook`.
    #[inline]
    pub async fn set_local_address(&mut self, local_address: SocketAddr) {
        // Acquire the peer book write lock.
        let mut peer_book = self.peer_book.write().await;
        // Update the local address stored in the peer book.
        peer_book.set_local_address(local_address);
        // Drop the peer book write lock.
        drop(peer_book);
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

    /// TODO (howardwu): Add logic to remove the active channels
    ///  and handshakes of the peer from this struct.
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
    pub async fn get_channel(&self, remote_address: &SocketAddr) -> Option<&Arc<Channel>> {
        self.channels.get(remote_address)
    }

    /// Stores a new channel at the peer address it is connected to.
    pub fn add_channel(&mut self, channel: &Arc<Channel>) {
        self.channels.insert(channel.address, channel.clone());
    }

    /// Manages all peer connections and processes updates with all connected peers.
    #[inline]
    pub async fn handler(&self) {
        // If the node is connected to less peers than the minimum required,
        // ask every peer the node is connected to for more peers.
        if self.get_num_connected().await < self.minimum_peer_count {
            // Broadcast a `GetPeers` message to request for more peers.
            self.broadcast_getpeers_requests().await;
            // Attempt a connection request with every disconnected peer.
            self.connect_to_all_disconnected_peers().await;
            // Attempt a connection request with every bootnode peer again.
            // The goal is to reconnect with any bootnode peers we might
            // have failed to connect to. The manager will filter attempts
            // to connect with itself or already connected bootnode peers.
            self.connect_to_bootnodes().await;
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

    /// An internal operation to write the internal state of the peer book to storage.
    #[inline]
    async fn store_peer_book(&self) {
        // Acquire the peer book write lock.
        let mut peer_book = self.peer_book.write().await;
        // Store the peer book into the database.
        peer_book
            .store(&self.storage)
            .unwrap_or_else(|error| debug!("Failed to store connected peers in database {}", error));
        // Drop the peer book write lock.
        drop(peer_book);
    }

    /// Broadcasts a `Version` message to all connected peers.
    #[inline]
    async fn broadcast_version_requests(&self) {
        // Get the local address of the node.
        let local_address = self.get_local_address().await;

        // Broadcast a `Version` message to each connected peer for a periodic sync.
        if self.get_num_connected().await > 0 {
            debug!("Sending out periodic version message to peers");

            // Send a `Version` message to every connected peer of the node.
            for (remote_address, _) in self.get_all_connected().await {
                if let Some(channel) = self.get_channel(&remote_address).await {
                    // Get the handshake nonce.
                    if let Some(nonce) = self.request_manager.get_handshake_nonce(&remote_address).await {
                        // Send a version message to peers.
                        // If they are behind, they will attempt to sync.
                        // TODO (raychu86) Establish a formal node version
                        let version = Version::from(
                            1u64,
                            self.storage.get_current_block_height(),
                            remote_address,
                            local_address,
                            nonce,
                        );
                        // Disconnect from the peer if the version request fails to send.
                        if let Err(_) = channel.write(&version).await {
                            self.disconnect_from_peer(&remote_address).await;
                        }
                    }
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

    /// Attempts a connection request with all disconnected peers.
    #[inline]
    async fn connect_to_all_disconnected_peers(&self) {
        // Get the local address of the node.
        let local_address = self.get_local_address().await;
        // Get the current block height of the node.
        let block_height = self.storage.get_current_block_height();
        // Get the current pending peers of the request manager.
        let pending_peers = self.request_manager.get_pending_addresses().await;

        // Iterate through each connected peer and attempts a connection request.
        for (remote_address, _) in self.get_all_disconnected().await {
            // Ensure the node does not try requesting a duplicate connection.
            let is_pending = pending_peers.contains(&remote_address);

            if !is_pending {
                // TODO (raychu86) Establish a formal node version
                // Create a version message.
                let version = Version::new(1u64, block_height, remote_address, local_address);
                // Send a connection request with the request manager.
                self.request_manager.send_connection_request(&version).await;
            }
        }
    }

    /// Attempts a connection request with all bootnode addresses provided by the network.
    #[inline]
    async fn connect_to_bootnodes(&self) {
        // Get the local address of the node.
        let local_address = self.get_local_address().await;
        // Get the current block height of the node.
        let block_height = self.storage.get_current_block_height();
        // Get the current connected peers of the node.
        let connected_peers = self.get_all_connected().await;
        // Get the current pending peers of the request manager.
        let pending_peers = self.request_manager.get_pending_addresses().await;

        // Iterate through each bootnode address and attempt a connection request.
        for bootnode_address in self.bootnode_addresses.iter() {
            // Ensure the node does not try connecting to itself.
            let is_self = local_address == *bootnode_address;
            // Ensure the node does not try reconnecting to a connected peer.
            let is_connected = connected_peers.contains_key(bootnode_address);
            // Ensure the node does not try requesting a duplicate connection.
            let is_pending = pending_peers.contains(bootnode_address);

            if !is_self && !is_connected && !is_pending {
                // TODO (raychu86) Establish a formal node version
                // Create a version message.
                let version = Version::new(1u64, block_height, *bootnode_address, local_address);
                // Send a connection request with the request manager.
                self.request_manager.send_connection_request(&version).await;
            }
        }
    }
}
