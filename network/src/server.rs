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
    external::{
        message::MessageName,
        message_types::{GetSync, MemoryPool as MemoryPoolMessage},
        protocol::*,
        Channel,
        GetMemoryPool,
    },
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

use crate::{
    external::{message::Message, message_types::*, propagate_block, protocol::SyncState, PingPongManager},
    internal::process_transaction_internal,
};
use snarkos_consensus::memory_pool::Entry;
use snarkos_objects::{Block as BlockStruct, BlockHeaderHash};
use snarkos_utilities::{
    bytes::{FromBytes, ToBytes},
    to_bytes,
};

use std::{collections::HashMap, net::IpAddr};

impl Server {
    /// This method handles all messages sent from connected peers.
    ///
    /// Messages are received by a single tokio MPSC receiver with
    /// the message name, bytes, associated channel, and a tokio oneshot sender.
    ///
    /// The oneshot sender lets the connection thread know when the message is handled.
    pub async fn message_handler(&mut self) {
        // TODO (raychu86) Create a macro to the handle the error messages.
        // TODO (howardwu): Come back and add error handlers to these.
        while let Some((tx, name, bytes, mut channel)) = self.receiver.recv().await {
            if name == Block::name() {
                if let Ok(block) = Block::deserialize(bytes) {
                    if let Err(err) = self.receive_block_message(block, channel.clone(), true).await {
                        error!(
                            "Message handler errored when receiving a {} message from {}. {}",
                            name, channel.address, err
                        );
                    }
                }
            } else if name == GetBlock::name() {
                if let Ok(getblock) = GetBlock::deserialize(bytes) {
                    if let Err(err) = self.receive_get_block(getblock, channel.clone()).await {
                        error!(
                            "Message handler errored when receiving a {} message from {}. {}",
                            name, channel.address, err
                        );
                    }
                }
            } else if name == GetMemoryPool::name() {
                if let Ok(getmemorypool) = GetMemoryPool::deserialize(bytes) {
                    if let Err(err) = self.receive_get_memory_pool(getmemorypool, channel.clone()).await {
                        error!(
                            "Message handler errored when receiving a {} message from {}. {}",
                            name, channel.address, err
                        );
                    }
                }
            } else if name == GetPeers::name() {
                if let Ok(getpeers) = GetPeers::deserialize(bytes) {
                    if let Err(err) = self.receive_get_peers(getpeers, channel.clone()).await {
                        error!(
                            "Message handler errored when receiving a {} message from {}. {}",
                            name, channel.address, err
                        );
                    }
                }
            } else if name == GetSync::name() {
                if let Ok(getsync) = GetSync::deserialize(bytes) {
                    if let Err(err) = self.receive_get_sync(getsync, channel.clone()).await {
                        error!(
                            "Message handler errored when receiving a {} message from {}. {}",
                            name, channel.address, err
                        );
                    }
                }
            } else if name == MemoryPoolMessage::name() {
                if let Ok(mempool) = MemoryPoolMessage::deserialize(bytes) {
                    if let Err(err) = self.receive_memory_pool(mempool).await {
                        error!(
                            "Message handler errored when receiving a {} message from {}. {}",
                            name, channel.address, err
                        );
                    }
                }
            } else if name == Peers::name() {
                if let Ok(peers) = Peers::deserialize(bytes) {
                    if let Err(err) = self.receive_peers(peers, channel.clone()).await {
                        error!(
                            "Message handler errored when receiving a {} message from {}. {}",
                            name, channel.address, err
                        );
                    }
                }
            } else if name == Ping::name() {
                if let Ok(ping) = Ping::deserialize(bytes) {
                    if let Err(err) = self.receive_ping(ping, channel.clone()).await {
                        error!(
                            "Message handler errored when receiving a {} message from {}. {}",
                            name, channel.address, err
                        );
                    }
                }
            } else if name == Pong::name() {
                if let Ok(pong) = Pong::deserialize(bytes) {
                    if let Err(err) = self.receive_pong(pong, channel.clone()).await {
                        error!(
                            "Message handler errored when receiving a {} message from {}. {}",
                            name, channel.address, err
                        );
                    }
                }
            } else if name == Sync::name() {
                if let Ok(sync) = Sync::deserialize(bytes) {
                    if let Err(err) = self.receive_sync(sync).await {
                        error!(
                            "Message handler errored when receiving a {} message from {}. {}",
                            name, channel.address, err
                        );
                    }
                }
            } else if name == SyncBlock::name() {
                if let Ok(block) = Block::deserialize(bytes) {
                    if let Err(err) = self.receive_block_message(block, channel.clone(), false).await {
                        error!(
                            "Message handler errored when receiving a {} message from {}. {}",
                            name, channel.address, err
                        );
                    }
                }
            } else if name == Transaction::name() {
                if let Ok(transaction) = Transaction::deserialize(bytes) {
                    if let Err(err) = self.receive_transaction(transaction, channel.clone()).await {
                        error!(
                            "Message handler errored when receiving a {} message from {}. {}",
                            name, channel.address, err
                        );
                    }
                }
            } else if name == Version::name() {
                if let Ok(version) = Version::deserialize(bytes) {
                    // TODO (raychu86) Does `receive_version` need to return a channel?
                    match self.receive_version(version, channel.clone()).await {
                        Ok(returned_channel) => channel = returned_channel,
                        Err(err) => error!(
                            "Message handler errored when receiving a {} message from {}. {}",
                            name, channel.address, err
                        ),
                    }
                }
            } else if name == Verack::name() {
                if let Ok(verack) = Verack::deserialize(bytes) {
                    if !self.receive_verack(verack, channel.clone()).await {
                        error!(
                            "Message handler errored when receiving a {} message from {}",
                            name, channel.address
                        );
                    }
                }
            } else if name == MessageName::from("disconnect") {
                info!("Disconnected from peer {:?}", channel.address);
                {
                    let mut peer_book = self.context.peer_book.write().await;
                    peer_book.disconnected_peer(&channel.address);
                }
            } else {
                debug!("Message name not recognized {:?}", name.to_string());
            }

            if let Err(error) = tx.send(channel) {
                warn!("Error resetting connection thread ({:?})", error);
            }
        }
    }

    /// A peer has sent us a new block to process.
    async fn receive_block_message(
        &mut self,
        message: Block,
        channel: Arc<Channel>,
        propagate: bool,
    ) -> Result<(), ServerError> {
        let block = BlockStruct::deserialize(&message.data)?;

        info!(
            "Received a block from epoch {} with hash {:?}",
            block.header.time,
            hex::encode(block.header.get_hash().0)
        );

        // Verify the block and insert it into the storage.
        if !self.storage.block_hash_exists(&block.header.get_hash()) {
            {
                let mut memory_pool = self.memory_pool_lock.lock().await;
                let inserted = self
                    .consensus
                    .receive_block(&self.parameters, &self.storage, &mut memory_pool, &block)
                    .is_ok();

                if inserted && propagate {
                    // This is a new block, send it to our peers.

                    propagate_block(self.context.clone(), message.data, channel.address).await?;
                } else if !propagate {
                    if let Ok(mut sync_handler) = self.sync_handler_lock.try_lock() {
                        sync_handler.clear_pending(Arc::clone(&self.storage));

                        if sync_handler.sync_state != SyncState::Idle {
                            // We are currently syncing with a node, ask for the next block.
                            if let Some(channel) = self
                                .context
                                .connections
                                .read()
                                .await
                                .get(&sync_handler.sync_node_address)
                            {
                                sync_handler.increment(channel, Arc::clone(&self.storage)).await?;
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// A peer has requested a block.
    async fn receive_get_block(&mut self, message: GetBlock, channel: Arc<Channel>) -> Result<(), ServerError> {
        if let Ok(block) = self.storage.get_block(&message.block_hash) {
            channel.write(&SyncBlock::new(block.serialize()?)).await?;
        }

        Ok(())
    }

    /// A peer has requested our memory pool transactions.
    async fn receive_get_memory_pool(
        &mut self,
        _message: GetMemoryPool,
        channel: Arc<Channel>,
    ) -> Result<(), ServerError> {
        let memory_pool = self.memory_pool_lock.lock().await;

        let mut transactions = vec![];

        for (_tx_id, entry) in &memory_pool.transactions {
            if let Ok(transaction_bytes) = to_bytes![entry.transaction] {
                transactions.push(transaction_bytes);
            }
        }

        if !transactions.is_empty() {
            channel.write(&MemoryPoolMessage::new(transactions)).await?;
        }

        Ok(())
    }

    /// A peer has sent us their memory pool transactions.
    async fn receive_memory_pool(&mut self, message: MemoryPoolMessage) -> Result<(), ServerError> {
        let mut memory_pool = self.memory_pool_lock.lock().await;

        for transaction_bytes in message.transactions {
            let transaction: Tx = Tx::read(&transaction_bytes[..])?;
            let entry = Entry::<Tx> {
                size: transaction_bytes.len(),
                transaction,
            };

            if let Ok(inserted) = memory_pool.insert(&self.storage, entry) {
                if let Some(txid) = inserted {
                    debug!("Transaction added to memory pool with txid: {:?}", hex::encode(txid));
                }
            }
        }

        Ok(())
    }

    /// A node has requested our list of peer addresses.
    /// Send an Address message with our current peer list.
    async fn receive_get_peers(&mut self, _message: GetPeers, channel: Arc<Channel>) -> Result<(), ServerError> {
        // If we received a message, but aren't connected to the peer,
        // inform the peer book that we found a peer.
        // The peer book will determine if we have seen the peer before,
        // and include the peer if it is new.
        let mut peer_book = self.context.peer_book.write().await;
        if !peer_book.is_connected(&channel.address) {
            peer_book.found_peer(&channel.address);
        }

        // Broadcast the sanitized list of connected peers back to requesting peer.
        let mut peers = HashMap::new();
        for (remote_address, peer_info) in peer_book.get_all_connected().iter() {
            // Skip the iteration if the requesting peer that we're sending the response to
            // appears in the list of peers.
            if *remote_address == channel.address {
                continue;
            }
            peers.insert(*remote_address, *peer_info.last_seen());
        }
        channel.write(&Peers::new(peers)).await?;

        Ok(())
    }

    /// A miner has sent their list of peer addresses.
    /// Add all new/updated addresses to our gossiped.
    /// The connection handler will be responsible for sending out handshake requests to them.
    async fn receive_peers(&mut self, message: Peers, channel: Arc<Channel>) -> Result<(), ServerError> {
        // If we received a message, but aren't connected to the peer,
        // inform the peer book that we found a peer.
        // The peer book will determine if we have seen the peer before,
        // and include the peer if it is new.
        let mut peer_book = self.context.peer_book.write().await;
        if !peer_book.is_connected(&channel.address) {
            peer_book.found_peer(&channel.address);
        }

        // Process all of the peers sent in the message,
        // by informing the peer book of that we found peers.
        let local_address = *self.context.local_address.read().await;

        for (peer_address, _) in message.addresses.iter() {
            // Skip if the peer address is the node's local address.
            let is_zero_address = match "0.0.0.0".to_string().parse::<IpAddr>() {
                Ok(zero_ip) => (*peer_address).ip() == zero_ip,
                _ => false,
            };
            if *peer_address == local_address || is_zero_address {
                continue;
            }
            // Inform the peer book that we found a peer.
            // The peer book will determine if we have seen the peer before,
            // and include the peer if it is new.
            else if !peer_book.is_connected(&channel.address) {
                peer_book.found_peer(&channel.address);
            }
        }

        Ok(())
    }

    /// A peer has sent us a ping message.
    /// Reply with a pong message.
    async fn receive_ping(&mut self, message: Ping, channel: Arc<Channel>) -> Result<(), ServerError> {
        // If we received a ping, but aren't connected to the peer,
        // inform the peer book that we found a peer.
        // The peer book will determine if we have seen the peer before,
        // and include the peer if it is new.
        let mut peer_book = self.context.peer_book.write().await;
        if !peer_book.is_connected(&channel.address) {
            peer_book.found_peer(&channel.address);
        }

        PingPongManager::send_pong(message, channel).await?;
        Ok(())
    }

    /// A peer has sent us a pong message.
    /// Check if it matches a ping we sent out.
    async fn receive_pong(&mut self, message: Pong, channel: Arc<Channel>) -> Result<(), ServerError> {
        // If we received a pong, but aren't connected to the peer,
        // inform the peer book that we found a peer.
        // The peer book will determine if we have seen the peer before,
        // and include the peer if it is new.
        let mut peer_book = self.context.peer_book.write().await;
        if !peer_book.is_connected(&channel.address) {
            peer_book.found_peer(&channel.address);
        }

        if let Err(error) = self
            .context
            .pings
            .write()
            .await
            .accept_pong(channel.address, message)
            .await
        {
            debug!(
                "Invalid pong message from: {:?}, Full error: {:?}",
                channel.address, error
            )
        }

        Ok(())
    }

    /// A peer has requested our chain state to sync with.
    async fn receive_get_sync(&mut self, message: GetSync, channel: Arc<Channel>) -> Result<(), ServerError> {
        let latest_shared_hash = self.storage.get_latest_shared_hash(message.block_locator_hashes)?;
        let current_height = self.storage.get_current_block_height();

        if let Ok(height) = self.storage.get_block_number(&latest_shared_hash) {
            if height < current_height {
                let mut max_height = current_height;

                // if the requester is behind more than 4000 blocks
                if height + 4000 < current_height {
                    // send the max 4000 blocks
                    max_height = height + 4000;
                }

                let mut block_hashes: Vec<BlockHeaderHash> = vec![];

                for block_num in height + 1..=max_height {
                    block_hashes.push(self.storage.get_block_hash(block_num)?);
                }

                // send block hashes to requester
                channel.write(&Sync::new(block_hashes)).await?;
            } else {
                channel.write(&Sync::new(vec![])).await?;
            }
        } else {
            channel.write(&Sync::new(vec![])).await?;
        }

        Ok(())
    }

    /// A peer has sent us their chain state.
    async fn receive_sync(&mut self, message: Sync) -> Result<(), ServerError> {
        let height = self.storage.get_current_block_height();
        let mut sync_handler = self.sync_handler_lock.lock().await;

        sync_handler.receive_hashes(message.block_hashes, height);

        // Received block headers
        if let Some(channel) = self
            .context
            .connections
            .read()
            .await
            .get(&sync_handler.sync_node_address)
        {
            sync_handler.increment(channel, Arc::clone(&self.storage)).await?;
        }

        Ok(())
    }

    /// A peer has sent us a transaction.
    async fn receive_transaction(&mut self, message: Transaction, channel: Arc<Channel>) -> Result<(), ServerError> {
        process_transaction_internal(
            self.context.clone(),
            &self.consensus,
            &self.parameters,
            self.storage.clone(),
            self.memory_pool_lock.clone(),
            message.bytes,
            channel.address,
        )
        .await?;

        Ok(())
    }

    /// A connected peer has acknowledged a handshake request.
    /// Check if the Verack matches the last handshake message we sent.
    /// Update our peer book and send a request for more peers.
    async fn receive_verack(&mut self, message: Verack, channel: Arc<Channel>) -> bool {
        match self.request_manager.accept_response(channel.address, message).await {
            true => {
                // If we received a verack, but aren't connected to the peer,
                // inform the peer book that we found a peer.
                // The peer book will determine if we have seen the peer before,
                // and include the peer if it is new.
                let mut peer_book = self.context.peer_book.write().await;
                if !peer_book.is_connected(&channel.address) {
                    peer_book.found_peer(&channel.address);
                }
                // Ask connected peer for more peers.
                channel.write(&GetPeers).await.is_ok()
            }
            false => {
                debug!("Received an invalid verack message from {:?}", channel.address);
                false
            }
        }
    }

    /// A connected peer has sent handshake request.
    /// Update peer's channel.
    /// If peer's block height is greater than ours, send a sync request.
    ///
    /// This method may seem redundant to handshake protocol functions but a peer can send additional
    /// Version messages if they want to update their ip address/port or want to share their chain height.
    async fn receive_version(&mut self, message: Version, channel: Arc<Channel>) -> Result<Arc<Channel>, ServerError> {
        let peer_address = SocketAddr::new(channel.address.ip(), message.address_sender.port());

        let peer_book = &mut self.context.peer_book.read().await;

        if *self.context.local_address.read().await != peer_address {
            if peer_book.num_connected() < self.context.max_peers {
                self.request_manager
                    .receive_request(message.clone(), peer_address)
                    .await;
            }

            // If our peer has a longer chain, send a sync message
            if message.height > self.storage.get_current_block_height() {
                debug!("Received a version message with a greater height {}", message.height);
                // Update the sync node if the sync_handler is Idle and there are no requested block headers
                if let Ok(mut sync_handler) = self.sync_handler_lock.try_lock() {
                    if !sync_handler.is_syncing()
                        && (sync_handler.block_headers.len() == 0 && sync_handler.pending_blocks.is_empty())
                    {
                        debug!("Attempting to sync with peer {}", peer_address);
                        sync_handler.sync_node_address = peer_address;

                        if let Ok(block_locator_hashes) = self.storage.get_block_locator_hashes() {
                            channel.write(&GetSync::new(block_locator_hashes)).await?;
                        }
                    } else {
                        if let Some(channel) = self
                            .context
                            .connections
                            .read()
                            .await
                            .get(&sync_handler.sync_node_address)
                        {
                            sync_handler.increment(channel, Arc::clone(&self.storage)).await?;
                        }
                    }
                }
            }
        }
        Ok(channel)
    }
}
