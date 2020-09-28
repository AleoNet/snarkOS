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
    external::{message::MessageName, message_types::GetSync, protocol::*, Channel, Version},
    internal::context::Context,
};
use snarkos_consensus::{ConsensusParameters, MemoryPool, MerkleTreeLedger};
use snarkos_dpc::base_dpc::{
    instantiated::{Components, Tx},
    parameters::PublicParameters,
};
use snarkos_errors::network::ServerError;

use chrono::{DateTime, Utc};
use std::{
    collections::HashMap,
    net::{Shutdown, SocketAddr},
    sync::Arc,
};
use tokio::{
    net::TcpListener,
    sync::{mpsc, oneshot, Mutex},
    task,
};
use tracing_futures::Instrument;

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
        }
    }

    /// Starts the server event loop.
    ///
    /// 1. Initialize TCP listener at `local_address` and accept new TCP connections.
    /// 2. Spawn a new thread to handle new connections.
    /// 3. Start the connection handler.
    /// 4. Send a handshake request to all bootnodes.
    /// 5. Send a handshake request to all stored peers.
    /// 6. Start the message handler.
    pub async fn listen(mut self) -> Result<(), ServerError> {
        // 1. Initialize TCP listener at `local_address` and accept new TCP connections.
        let (mut listener, local_address) = {
            let address = self.context.local_address.read().await;
            let local_address = format!("0.0.0.0:{}", address.port()).parse::<SocketAddr>()?;
            info!("Starting listener...");
            (TcpListener::bind(&local_address).await?, local_address)
        };
        info!("Listening at {:?}", local_address);

        // Prepare to spawn the main loop.
        let sender = self.sender.clone();
        let storage = self.storage.clone();
        let context = self.context.clone();
        let sync_handler_lock = self.sync_handler_lock.clone();

        // 2. Spawn a new thread to handle new connections.
        let future = async move {
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
                if context.peer_book.read().await.connected_total() >= context.max_peers {
                    warn!("Rejected a connection request as this exceeds the maximum number of peers allowed");
                    if let Err(error) = reader.shutdown(Shutdown::Write) {
                        error!("Failed to shutdown peer reader ({})", error);
                    }
                    continue;
                }

                // Follow handshake protocol and drop peer connection if unsuccessful.
                let height = storage.get_latest_block_height();
                let mut handshakes = context.handshakes.write().await; // Acquire the handshake lock
                // TODO (raychu86) Establish a formal node version
                if let Ok((handshake, discovered_local_address, version_message)) =
                    handshakes.receive_any(1u64, height, remote_address, reader).await
                {
                    // Bootstrap discovery of local node IP via VERACK responses
                    {
                        let mut local_address = context.local_address.write().await;
                        if *local_address != discovered_local_address {
                            *local_address = discovered_local_address;
                            info!("Discovered local address: {:?}", *local_address);
                            let mut peer_book = context.peer_book.write().await;
                            peer_book.forget_peer(discovered_local_address);
                        }
                    }

                    // Store the channel established with the handshake
                    {
                        let mut connections = context.connections.write().await; // Acquire the connections lock
                        connections.store_channel(&handshake.channel);
                    }

                    if let Some(version) = version_message {
                        // If our peer has a longer chain, send a sync message
                        if version.height > storage.get_latest_block_height() {
                            // Update the sync node if the sync_handler is Idle
                            if let Ok(mut sync_handler) = sync_handler_lock.try_lock() {
                                if !sync_handler.is_syncing() {
                                    sync_handler.sync_node = handshake.channel.address;

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
        };
        task::spawn(future.in_current_span());

        // 3. Start the connection handler.
        debug!("Starting connection handler");
        self.connection_handler().await;

        // 4. Send handshake request to bootnodes.
        debug!("Sending handshake request to bootnodes");
        self.connect_bootnodes().await;

        // If the node is a bootnode, do not send requests to stored peers
        if !self.context.is_bootnode {
            // 5. Send a handshake request to all stored peers.
            debug!("Sending handshake request to all stored peers");
            self.connect_peers_from_storage().await;
        }

        // 6. Start the message handler.
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
        let future = async move {
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
        };
        task::spawn(future.in_current_span());
    }

    /// Send a handshake request to a node at address without blocking the server listener.
    fn send_handshake_non_blocking(&self, remote_address: SocketAddr) {
        let context = self.context.clone();
        let storage = self.storage.clone();

        let future = async move {
            let height = storage.get_latest_block_height();
            let version = Version::new(1u64, height, remote_address, *context.local_address.read().await);

            let mut handshakes = context.handshakes.write().await;
            handshakes.send_request(&version).await.unwrap_or_else(|error| {
                info!("Failed to connect to {:?}", error);
                ()
            });
        };
        task::spawn(future.in_current_span());
    }

    /// Send a handshake request the first bootnode and store the rest as gossipped peers
    async fn connect_bootnodes(&mut self) {
        let local_address = *self.context.local_address.read().await;
        for bootnode in self.context.bootnodes.iter() {
            if let Ok(bootnode_address) = bootnode.parse::<SocketAddr>() {
                // This node should not attempt to connect to itself.
                if local_address != bootnode_address {
                    info!("Connecting to {:?} (bootnode)...", bootnode_address);
                    self.send_handshake_non_blocking(bootnode_address);
                }
            }
        }
    }

    /// Send a handshake request to every peer this server previously connected to.
    async fn connect_peers_from_storage(&mut self) {
        if let Ok(serialized_peers) = self.storage.get_peer_book() {
            if let Ok(stored_connected_peers) =
                bincode::deserialize::<HashMap<SocketAddr, DateTime<Utc>>>(&serialized_peers)
            {
                let local_address = *self.context.local_address.read().await;
                for (saved_address, _old_time) in stored_connected_peers {
                    // This node should not attempt to connect to itself.
                    if local_address != saved_address {
                        info!("Connecting to {:?} (saved peer)...", saved_address);
                        self.send_handshake_non_blocking(saved_address);
                    }
                }
            }
        }
    }
}
