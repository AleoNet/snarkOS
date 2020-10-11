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
        Channel, Handshake, PingPongManager,
    },
    internal::{PeerBook, PeerInfo},
    Environment, NetworkError, SendHandler,
};
use snarkos_models::objects::Transaction;

use chrono::Utc;
use std::{
    collections::{HashMap, HashSet},
    net::SocketAddr,
    sync::Arc,
};
use tokio::sync::RwLock;

/// A stateful component for managing the peer connections of this node.
#[derive(Clone)]
pub struct PeerManager {
    /// The network parameters for this node.
    environment: Environment,
    /// The list of connected and disconnected peers for this node.
    peer_book: Arc<RwLock<PeerBook>>,

    /// The channels of the peers that this node is connected to.
    channels: HashMap<SocketAddr, Arc<Channel>>,

    /// A list of remote addresses currently sending a request.
    pending_addresses: Arc<RwLock<HashSet<SocketAddr>>>,
}

impl PeerManager {
    ///
    /// Creates a new instance of `PeerManager`.
    ///
    /// Initializes the `PeerManager` with the following steps.
    /// 1. Attempt to connect to all default bootnodes on the network.
    /// 2. Attempt to connect to all disconnected peers from the stored peer book.
    ///
    #[inline]
    pub async fn new(environment: Environment) -> Result<Self, NetworkError> {
        // Load the peer book from storage, or create a new peer book.
        let mut peer_book = match PeerBook::load(&*environment.storage_read().await) {
            // Case 1 - The peer book was found in storage.
            Ok(peer_book) => peer_book,
            // Case 2 - Either the peer book does not exist in storage, or could not be deserialized.
            // Create a new instance of the peer book.
            _ => PeerBook::new(*environment.local_address()),
        };

        // Instantiate a peer manager.
        let peer_manager = Self {
            environment,
            peer_book: Arc::new(RwLock::new(peer_book)),
            channels: HashMap::new(),

            pending_addresses: Arc::new(RwLock::new(HashSet::default())),
        };

        // Save the peer book to storage.
        peer_manager.save_peer_book_to_storage().await?;

        Ok(peer_manager)
    }

    /// Returns the local address of this node.
    #[inline]
    pub fn local_address(&self) -> SocketAddr {
        // TODO (howardwu): Check that env addr and peer book addr match.
        // // Acquire the peer book reader.
        // let peer_book = self.peer_book.read().await;
        // // Fetch the local address of this node.
        // peer_book.local_address()

        *self.environment.local_address()
    }

    /// Updates the local address stored in the `PeerBook`.
    #[inline]
    pub async fn set_local_address(&mut self, local_address: SocketAddr) {
        // Acquire the peer book write lock.
        let mut peer_book = self.peer_book.write().await;
        // Update the local address stored in the peer book.
        peer_book.set_local_address(local_address);
    }

    /// Returns `true` if a given address is connected to this node.
    #[inline]
    pub async fn is_connected(&self, address: &SocketAddr) -> bool {
        // Acquire the peer book reader.
        let peer_book = self.peer_book.read().await;
        // Fetch if the given address is connected in the peer book.
        peer_book.is_connected(address)
    }

    /// Returns `true` if a given address is a disconnected peer of this node.
    #[inline]
    pub async fn is_disconnected(&self, address: &SocketAddr) -> bool {
        // Acquire the peer book reader.
        let peer_book = self.peer_book.read().await;
        // Fetch if the given address is disconnected in the peer book.
        peer_book.is_disconnected(address)
    }

    /// Returns the number of peers connected to this node.
    #[inline]
    pub async fn num_connected(&self) -> u16 {
        // Acquire the peer book reader.
        let peer_book = self.peer_book.read().await;
        // Fetch the number of connected peers.
        peer_book.num_connected()
    }

    /// Returns a reference to the connected peers of this node.
    #[inline]
    pub async fn get_all_connected(&self) -> HashMap<SocketAddr, PeerInfo> {
        // Acquire the peer book reader.
        let peer_book = self.peer_book.read().await;
        // Fetch the connected peers of this node.
        peer_book.get_all_connected().clone()
    }

    /// Returns a reference to the disconnected peers of this node.
    #[inline]
    pub async fn get_all_disconnected(&self) -> HashMap<SocketAddr, PeerInfo> {
        // Acquire the peer book reader.
        let peer_book = self.peer_book.read().await;
        // Fetch the disconnected peers of this node.
        peer_book.get_all_disconnected().clone()
    }

    /// Adds the given address to the disconnected peers in this peer book.
    /// Returns `true` on success. Otherwise, returns `false`.
    #[inline]
    pub async fn found_peer(&self, address: &SocketAddr) -> bool {
        // Acquire the peer book reader.
        let mut peer_book = self.peer_book.write().await;
        // Fetch if the given address is disconnected in the peer book.
        peer_book.found_peer(address)
    }

    /// TODO (howardwu): Add logic to remove the active channels
    ///  and handshakes of the peer from this struct.
    /// Attempts to disconnect the given address from this node.
    #[inline]
    pub async fn disconnect_from_peer(&self, remote_address: &SocketAddr) -> bool {
        // Acquire the peer book write lock.
        let mut peer_book = self.peer_book.write().await;
        // Set the peer as disconnected in the peer book.
        peer_book.disconnected_peer(remote_address)
    }

    /// Returns the list of remote addresses currently sending a request.
    #[inline]
    pub async fn get_pending_addresses(&self) -> Vec<SocketAddr> {
        // Acquire the pending addresses read lock.
        let pending_addresses = self.pending_addresses.read().await;
        pending_addresses.clone().into_iter().collect()
    }

    ///
    /// Attempts to fetch the channel for a given address.
    ///
    /// Returns `Some(channel)` if the address is a connected peer.
    /// Otherwise, returns `None`.
    ///
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
        // If this node is connected to less peers than the minimum required,
        // ask every peer this node is connected to for more peers.
        if self.num_connected().await < self.environment.min_peers() {
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
        self.save_peer_book_to_storage().await;
    }

    /// Broadcasts connection requests to the default bootnodes of the network
    /// and each disconnected peer saved in the peer book.
    #[inline]
    async fn initialize(&self) {
        debug!("Starting initialization of the peer manager");

        // Attempt to connect to the default bootnodes of the network.
        trace!("Broadcasting connection requests to the default bootnodes");
        self.connect_to_bootnodes().await;

        // Check that this node is not a bootnode.
        if !self.environment.is_bootnode() {
            // Attempt to connect to each disconnected peer saved in the peer book.
            trace!("Broadcasting connection requests to disconnected peers");
            self.connect_to_all_disconnected_peers().await;
        }

        debug!("Completed initialization of the peer manager");
    }

    /// Broadcasts a connection request to all default bootnodes of the network.
    #[inline]
    async fn connect_to_bootnodes(&self) {
        // Get the local address of this node.
        let local_address = self.local_address();
        // Get the current block height of this node.
        let block_height = self.environment.current_block_height().await;
        // Get the current connected peers of this node.
        let connected_peers = self.get_all_connected().await;
        // Get the current pending peers of the send handler.
        let pending_peers = self.get_pending_addresses().await;

        // Iterate through each bootnode address and attempt a connection request.
        for bootnode_address in self.environment.bootnodes().iter() {
            // Ensure this node does not try connecting to itself.
            let is_self = local_address == *bootnode_address;
            // Ensure this node does not try reconnecting to a connected peer.
            let is_connected = connected_peers.contains_key(bootnode_address);
            // Ensure this node does not try requesting a duplicate connection.
            let is_pending = pending_peers.contains(bootnode_address);

            if !is_self && !is_connected && !is_pending {
                // TODO (raychu86) Establish a formal node version
                // Create a version message.
                let version = Version::new(1u64, block_height, *bootnode_address, local_address);
                // Send a connection request with the send handler.
                // self.environment
                //     .send_handler()
                //     .send_connection_request(&self.environment, &version)
                //     .await;
                self.send_connection_request(&self.environment, &version).await;
            }
        }
    }

    /// Broadcasts a connection request to all disconnected peers.
    #[inline]
    async fn connect_to_all_disconnected_peers(&self) {
        // Get the local address of this node.
        let local_address = self.local_address();
        // Get the current block height of this node.
        let block_height = self.environment.current_block_height().await;
        // Get the current pending peers of the send handler.
        let pending_peers = self.get_pending_addresses().await;

        // Iterate through each connected peer and attempts a connection request.
        for (remote_address, _) in self.get_all_disconnected().await {
            // Check if the peer manager is already attempting to connect to the remote address.
            let is_pending = pending_peers.contains(&remote_address);
            // Ensure the peer manager does not create a duplicate connection request.
            if !is_pending {
                // TODO (raychu86) Establish a formal node version
                // Create a version message.
                let version = Version::new(1u64, block_height, remote_address, local_address);
                // Send a connection request with the send handler.
                // self.environment
                //     .send_handler()
                //     .send_connection_request(&self.environment, &version)
                //     .await;
                self.send_connection_request(&self.environment, &version).await;
            }
        }
    }

    /// TODO (howardwu): Refactor this into `SendHandler`. Just remove pending peers so it stays here.
    ///
    /// Sends a connection request with a given version message.
    ///
    /// Broadcasts a handshake request with a given version message.
    ///
    /// Creates a new handshake with a remote address,
    /// and attempts to send a handshake request to them.
    ///
    /// Upon success, the handshake is stored in the manager.
    ///
    #[inline]
    pub async fn send_connection_request(&self, environment: &Environment, version: &Version) {
        // Increment the request counter.
        // self.send_request_count.fetch_add(1, Ordering::SeqCst);

        // Clone an instance of version, handshakes, pending_addresses,
        // send_success_count, and send_failure_count for the tokio thread.
        let version = version.clone();
        let handshakes = environment.handshakes().clone();
        let pending_addresses = self.pending_addresses.clone();
        // let send_success_count = self.send_success_count.clone();
        // let send_failure_count = self.send_failure_count.clone();

        // Spawn a new thread to make this a non-blocking operation.
        tokio::task::spawn(async move {
            // Get the remote address for logging.
            let remote_address = version.address_receiver;
            info!("Attempting connection to {:?}", remote_address);

            // Acquire the pending addresses write lock.
            let mut pending_peers = pending_addresses.write().await;
            // Add the remote address to the pending addresses.
            pending_peers.insert(remote_address);
            // Drop the pending addresses write lock.
            drop(pending_peers);

            // Acquire the handshake and pending addresses write locks.
            let mut handshakes = handshakes.write().await;
            // Attempt a handshake with the remote address.
            debug!("Requesting handshake with {:?}", remote_address);
            match Handshake::send_new(&version).await {
                Ok(handshake) => {
                    // Store the handshake.
                    handshakes.insert(remote_address, handshake);
                    // Increment the success counter.
                    // send_success_count.fetch_add(1, Ordering::SeqCst);
                    debug!("Sent handshake to {:?}", remote_address);
                }
                _ => {
                    // Increment the failed counter.
                    // send_failure_count.fetch_add(1, Ordering::SeqCst);
                    info!("Unsuccessful connection with {:?}", remote_address);
                }
            };
            // Drop the handshake write lock.
            drop(handshakes);

            // Acquire the pending addresses write lock.
            let mut pending_peers = pending_addresses.write().await;
            // Remove the remote address from the pending addresses.
            pending_peers.remove(&remote_address);
            // Drop the pending addresses write lock.
            drop(pending_peers);
        });
    }

    /// TODO (howardwu): Implement manual serializers and deserializers to prevent forward breakage
    ///  when the PeerBook or PeerInfo struct fields change.
    ///
    /// Stores the current peer book to the given storage object.
    ///
    /// This function checks that this node is not connected to itself,
    /// and proceeds to serialize the peer book into a byte vector for storage.
    ///
    #[inline]
    async fn save_peer_book_to_storage(&self) -> Result<(), NetworkError> {
        trace!("Peer manager is saving the peer book to storage");

        // Acquire the peer book write lock.
        let mut peer_book = self.peer_book.write().await;
        // Check that the node does not maintain a connection to itself.
        peer_book.forget_peer(self.local_address());
        // Serialize the peer book.
        let serialized_peer_book = bincode::serialize(&*peer_book)?;
        // Drop the peer book write lock.
        drop(peer_book);

        // Acquire the storage write lock.
        let storage = self.environment.storage_mut().await;
        // Save the serialized peer book to storage.
        storage.save_peer_book_to_storage(serialized_peer_book)?;

        trace!("Peer manager saved the peer book to storage");
        Ok(())
    }

    /// Broadcasts a `Version` message to all connected peers.
    #[inline]
    async fn broadcast_version_requests(&self) {
        // Get the local address of this node.
        let local_address = self.local_address();

        // Broadcast a `Version` message to each connected peer for a periodic sync.
        if self.num_connected().await > 0 {
            debug!("Sending out periodic version message to peers");

            // Send a `Version` message to every connected peer of this node.
            for (remote_address, _) in self.get_all_connected().await {
                if let Some(channel) = self.get_channel(&remote_address).await {
                    // Get the handshake nonce.
                    if let Some(nonce) = self
                        .environment
                        .send_handler()
                        .get_handshake_nonce(&self.environment, &remote_address)
                        .await
                    {
                        // Send a version message to peers.
                        // If they are behind, they will attempt to sync.
                        // TODO (raychu86) Establish a formal node version
                        let version = Version::from(
                            1u64,
                            self.environment.current_block_height().await,
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
        // that this node hasn't heard from in a while.
        for (remote_address, peer_info) in self.get_all_connected().await {
            // Calculate the time since we last saw the peer.
            let elapsed_in_millis = (Utc::now() - *peer_info.last_seen()).num_milliseconds();
            if elapsed_in_millis.is_positive() && elapsed_in_millis as u64 > (self.environment.sync_interval() * 3) {
                if let Some(channel) = self.get_channel(&remote_address).await {
                    // Acquire the ping pong manager write lock.
                    let mut ping_pong = self.environment.ping_pong().write().await;
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
}
