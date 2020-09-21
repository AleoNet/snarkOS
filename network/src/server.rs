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
    connection_manager::ConnectionManager,
    external::{message::MessageName, message_types::GetSync, protocol::*, Channel, GetMemoryPool},
    internal::context::Context,
    RequestManager,
};
use snarkos_consensus::{ConsensusParameters, MemoryPool, MerkleTreeLedger};
use snarkos_dpc::base_dpc::{
    instantiated::{Components, Tx},
    parameters::PublicParameters,
};
use snarkos_errors::network::ServerError;

use std::{
    net::{Shutdown, SocketAddr},
    sync::Arc,
    time::Duration,
};
use tokio::{
    net::TcpListener,
    sync::{mpsc, oneshot, Mutex},
    task,
    time::delay_for,
};

/// The main networking component of a node.
pub struct Server {
    pub consensus: ConsensusParameters,
    pub context: Arc<Context>,
    pub storage: Arc<MerkleTreeLedger>,
    pub parameters: PublicParameters<Components>,
    pub memory_pool_lock: Arc<Mutex<MemoryPool<Tx>>>,
    pub sync_handler_lock: Arc<Mutex<SyncHandler>>,
    pub connection_frequency: u64,
    pub sender: mpsc::Sender<(oneshot::Sender<Arc<Channel>>, MessageName, Vec<u8>, Arc<Channel>)>,
    pub receiver: mpsc::Receiver<(oneshot::Sender<Arc<Channel>>, MessageName, Vec<u8>, Arc<Channel>)>,
    pub request_manager: RequestManager,
}

impl Server {
    /// Constructs a new `Server`.
    pub fn new(
        context: Arc<Context>,
        consensus: ConsensusParameters,
        storage: Arc<MerkleTreeLedger>,
        parameters: PublicParameters<Components>,
        memory_pool_lock: Arc<Mutex<MemoryPool<Tx>>>,
        sync_handler_lock: Arc<Mutex<SyncHandler>>,
        connection_frequency: u64,
    ) -> Self {
        let (sender, receiver) = mpsc::channel(1024);
        let request_manager = RequestManager::new();

        Server {
            consensus,
            context,
            storage,
            parameters,
            memory_pool_lock,
            receiver,
            sender,
            sync_handler_lock,
            connection_frequency,
            request_manager,
        }
    }

    /// Returns the default bootnode addresses of the network.
    pub fn get_bootnodes(&self) -> Vec<SocketAddr> {
        // Initialize the vector to be returned.
        let mut bootnode_addresses = Vec::with_capacity(self.context.bootnodes.len());
        // Iterate through and parse the list of bootnode addresses.
        for bootnode in self.context.bootnodes.iter() {
            if let Ok(bootnode_address) = bootnode.parse::<SocketAddr>() {
                bootnode_addresses.push(bootnode_address);
            }
        }
        bootnode_addresses
    }

    ///
    /// Starts the server event loop.
    ///
    /// 1. Initialize TCP listener at `local_address` and accept new TCP connections.
    /// 2. Spawn a new thread to handle new connections.
    /// 3. Start the connection handler.
    /// 6. Start the message handler.
    ///
    pub async fn listen(mut self) -> Result<(), ServerError> {
        // Prepare to spawn the main loop.
        let sender = self.sender.clone();
        let storage = self.storage.clone();
        let context = self.context.clone();
        let sync_handler_lock = self.sync_handler_lock.clone();
        let mut request_manager = self.request_manager.clone();

        let connection_manager = ConnectionManager::new(
            &context,
            request_manager.clone(),
            self.context.connections.clone(),
            &storage,
            self.get_bootnodes(),
            self.connection_frequency,
        )
        .await;
        let mut new_connection_manager = connection_manager.clone();

        // TODO (howardwu): Find the actual address of the node.
        // 1. Initialize TCP listener and accept new TCP connections.
        let local_address = new_connection_manager.get_local_address().await;
        debug!("Starting listener at {:?}...", local_address);
        let mut listener = TcpListener::bind(&local_address).await?;
        info!("Listening at {:?}", local_address);

        // 2. Spawn a new thread to handle new connections.
        task::spawn(async move {
            debug!("Spawning a new thread to handle new connections");
            loop {
                // Listen for new peers.
                let (reader, remote_address) = match listener.accept().await {
                    Ok((reader, remote_address)) => {
                        info!("Received a connection request from {}", remote_address);
                        (reader, remote_address)
                    }
                    Err(error) => {
                        error!("Failed to accept connection {}", error);
                        continue;
                    }
                };

                // Check if we've exceed our maximum number of allowed peers.
                if context.peer_book.read().await.num_connected() >= context.max_peers {
                    warn!("Rejected a connection request as this exceeds the maximum number of peers allowed");
                    if let Err(error) = reader.shutdown(Shutdown::Write) {
                        error!("Failed to shutdown peer reader ({})", error);
                    }
                    continue;
                }

                // Follow handshake protocol and drop peer connection if unsuccessful.
                let height = storage.get_current_block_height();

                // TODO (raychu86) Establish a formal node version
                if let Some((handshake, discovered_local_address, version_message)) = request_manager
                    .receive_connection_request(1u64, height, remote_address, reader)
                    .await
                {
                    // Bootstrap discovery of local node IP via VERACK responses
                    {
                        let local_address = new_connection_manager.get_local_address().await;
                        if local_address != discovered_local_address {
                            new_connection_manager.set_local_address(discovered_local_address).await;
                            info!("Discovered local address: {:?}", local_address);
                        }
                    }
                    // Store the channel established with the handshake
                    new_connection_manager.add_channel(&handshake.channel);

                    if let Some(version) = version_message {
                        // If our peer has a longer chain, send a sync message
                        if version.height > storage.get_current_block_height() {
                            // Update the sync node if the sync_handler is Idle
                            if let Ok(mut sync_handler) = sync_handler_lock.try_lock() {
                                if !sync_handler.is_syncing() {
                                    sync_handler.sync_node_address = handshake.channel.address;

                                    if let Ok(block_locator_hashes) = storage.get_block_locator_hashes() {
                                        if let Err(err) =
                                            handshake.channel.write(&GetSync::new(block_locator_hashes)).await
                                        {
                                            error!(
                                                "Error sending GetSync message to {}, {}",
                                                handshake.channel.address, err
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Inner loop spawns one thread per connection to read messages
                    Self::spawn_connection_thread(handshake.channel.clone(), sender.clone());
                }
            }
        });

        // 3. Start the connection handler.
        debug!("Starting connection handler");
        self.connection_handler(connection_manager).await;

        // 4. Start the message handler.
        debug!("Starting message handler");
        self.message_handler().await;

        Ok(())
    }

    /// Spawns one thread per peer tcp connection to read messages.
    /// Each thread is given a handle to the channel and a handle to the server mpsc sender.
    /// To ensure concurrency, each connection thread sends a tokio oneshot sender handle with every message to the server mpsc receiver.
    /// The thread then waits for the oneshot receiver to receive a signal from the server before reading again.
    fn spawn_connection_thread(
        mut channel: Arc<Channel>,
        mut message_handler_sender: mpsc::Sender<(oneshot::Sender<Arc<Channel>>, MessageName, Vec<u8>, Arc<Channel>)>,
    ) {
        task::spawn(async move {
            // Determines the criteria for disconnecting from a peer.
            fn should_disconnect(failure_count: &u8) -> bool {
                // Tolerate up to 10 failed communications.
                *failure_count >= 10
            }

            // Logs the failure and determines whether to disconnect from a peer.
            async fn handle_failure<T: std::fmt::Display>(
                failure: &mut bool,
                failure_count: &mut u8,
                disconnect_from_peer: &mut bool,
                error: T,
            ) {
                // Only increment failure_count if we haven't seen a failure yet.
                if !*failure {
                    // Update the state to reflect a new failure.
                    *failure = true;
                    *failure_count += 1;
                    warn!(
                        "Connection errored {} time(s) (error message: {})",
                        failure_count, error
                    );

                    // Determine if we should disconnect.
                    *disconnect_from_peer = should_disconnect(failure_count);
                } else {
                    debug!("Connection errored again in the same loop (error message: {})", error);
                }

                // Sleep for 10 seconds
                tokio::time::delay_for(std::time::Duration::from_secs(10)).await;
            }

            let mut failure_count = 0u8;
            let mut disconnect_from_peer = false;

            loop {
                // Initialize the failure indicator.
                let mut failure = false;

                // Read the next message from the channel. This is a blocking operation.
                let (message_name, message_bytes) = match channel.read().await {
                    Ok((message_name, message_bytes)) => (message_name, message_bytes),
                    Err(error) => {
                        handle_failure(&mut failure, &mut failure_count, &mut disconnect_from_peer, error).await;

                        // Determine if we should send a disconnect message.
                        match disconnect_from_peer {
                            true => (MessageName::from("disconnect"), vec![]),
                            false => continue,
                        }
                    }
                };

                // Use a oneshot channel to give the channel control
                // to the message handler after reading from the channel.
                let (tx, rx) = oneshot::channel();

                // Send the successful read data to the message handler.
                if let Err(error) = message_handler_sender
                    .send((tx, message_name, message_bytes, channel.clone()))
                    .await
                {
                    handle_failure(&mut failure, &mut failure_count, &mut disconnect_from_peer, error).await;
                    continue;
                };

                // Wait for the message handler to give back channel control.
                match rx.await {
                    Ok(peer_channel) => channel = peer_channel,
                    Err(error) => {
                        handle_failure(&mut failure, &mut failure_count, &mut disconnect_from_peer, error).await
                    }
                };

                // Break out of the loop if the peer disconnects.
                if disconnect_from_peer {
                    warn!("Disconnecting from an unreliable peer");
                    break;
                }
            }
        });
    }

    // TODO (howardwu): Untangle this and find its components new homes.
    /// Manages the number of active connections according to the connection frequency.
    /// 1. Get more connected peers if we are under the minimum number specified by the network context.
    ///     1.1 Ask our connected peers for their peers.
    ///     1.2 Ask our gossiped peers to handshake and become connected.
    /// 2. Maintain connected peers by sending ping messages.
    /// 3. Purge peers that have not responded in connection_frequency x 5 seconds.
    /// 4. Reselect a sync node if we purged it.
    /// 5. Update our memory pool every connection_frequency x memory_pool_interval seconds.
    /// All errors encountered by the connection handler will be logged to the console but will not stop the thread.
    pub async fn connection_handler(&self, connection_manager: ConnectionManager) {
        let context = self.context.clone();
        let memory_pool_lock = self.memory_pool_lock.clone();
        let sync_handler_lock = self.sync_handler_lock.clone();
        let storage = self.storage.clone();
        let connection_frequency = self.connection_frequency;

        // Start a separate thread for the handler.
        task::spawn(async move {
            let mut interval_ticker: u8 = 0;

            loop {
                // Wait for connection_frequency seconds in between each loop
                delay_for(Duration::from_millis(connection_frequency)).await;

                connection_manager.handler().await;

                // TODO (howardwu): Rewrite this into a dedicated manager for syncing.
                {
                    let local_address = connection_manager.get_local_address().await;

                    // If we have disconnected from our sync node,
                    // then set our sync state to idle and find a new sync node.
                    if let Ok(mut sync_handler) = sync_handler_lock.try_lock() {
                        let peer_book = context.peer_book.read().await;
                        if peer_book.is_disconnected(&sync_handler.sync_node_address) {
                            if let Some(peer) = peer_book
                                .get_all_connected()
                                .iter()
                                .max_by(|a, b| a.1.last_seen().cmp(&b.1.last_seen()))
                            {
                                sync_handler.sync_state = SyncState::Idle;
                                sync_handler.sync_node_address = peer.0.clone();
                            };
                        }
                        drop(peer_book)
                    }

                    // Update our memory pool after memory_pool_interval frequency loops.
                    if interval_ticker >= context.memory_pool_interval {
                        if let Ok(sync_handler) = sync_handler_lock.try_lock() {
                            // Ask our sync node for more transactions.
                            if local_address != sync_handler.sync_node_address {
                                if let Some(channel) =
                                    connection_manager.get_channel(&sync_handler.sync_node_address).await
                                {
                                    if let Err(_) = channel.write(&GetMemoryPool).await {
                                        // Acquire the peer book write lock.
                                        let mut peer_book = context.peer_book.write().await;
                                        peer_book.disconnected_peer(&sync_handler.sync_node_address);
                                        drop(peer_book);
                                    }
                                }
                            }
                        }

                        // Update the node's memory pool.
                        let mut memory_pool = match memory_pool_lock.try_lock() {
                            Ok(memory_pool) => memory_pool,
                            _ => continue,
                        };
                        memory_pool.cleanse(&storage).unwrap_or_else(|error| {
                            debug!("Failed to cleanse memory pool transactions in database {}", error)
                        });
                        memory_pool.store(&storage).unwrap_or_else(|error| {
                            debug!("Failed to store memory pool transaction in database {}", error)
                        });

                        interval_ticker = 0;
                    } else {
                        interval_ticker += 1;
                    }
                }
            }
        });
    }
}
