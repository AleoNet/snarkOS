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
        handshake::HandshakeStatus,
        message::Message,
        message_types::*,
        Channel,
        Handshake,
        MessageName,
        PingPongManager,
    },
    request::Request,
    Environment,
    NetworkError,
    PeerSender,
    Receiver,
    SyncManager,
    SyncState,
};
use snarkos_consensus::memory_pool::Entry;
use snarkos_dpc::{
    instantiated::{Components, Tx},
    PublicParameters,
};
use snarkos_errors::network::{HandshakeError, ServerError};
use snarkos_objects::{Block as BlockStruct, BlockHeaderHash};
use snarkos_utilities::{
    bytes::{FromBytes, ToBytes},
    to_bytes,
};

use std::{
    collections::HashMap,
    fmt::Display,
    net::{IpAddr, Shutdown, SocketAddr},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::{mpsc, oneshot, Mutex, RwLock},
    task,
};

/// The map of remote addresses to their active read channels.
pub type Channels = HashMap<SocketAddr, Arc<Channel>>;

/// A stateless component for handling inbound network traffic.
#[derive(Debug, Clone)]
pub struct ReceiveHandler {
    /// The map of remote addresses to their active read channels.
    channels: Arc<RwLock<Channels>>,
    /// The sender for this handler to send responses to the peer manager.
    peer_sender: Option<Arc<RwLock<PeerSender>>>,
    /// A counter for the number of received responses the handler processes.
    receive_response_count: Arc<AtomicU64>,
    /// A counter for the number of received responses that succeeded.
    receive_success_count: Arc<AtomicU64>,
    /// A counter for the number of received responses that failed.
    receive_failure_count: Arc<AtomicU64>,
}

impl ReceiveHandler {
    /// Creates a new instance of a `ReceiveHandler`.
    #[inline]
    pub fn new() -> Self {
        Self {
            channels: Arc::new(RwLock::new(HashMap::new())),
            peer_sender: None,
            receive_response_count: Arc::new(AtomicU64::new(0)),
            receive_success_count: Arc::new(AtomicU64::new(0)),
            receive_failure_count: Arc::new(AtomicU64::new(0)),
        }
    }

    /**

    [1 invocation per peer manager] Add initialize_peer_sender() which sets the `PeerSender`.
        - Save the peer_sender into receive handler.

    [1 instance per peer manager] Add listen() which detects new peer. (for `channel`)
        - Save channel into receive handler.
        - For each new peer, call the handler().

    [1 instance per peer] Add handler() which initializes new handler. (for `channel`)
        - Calls authorize() to obtain a (channel, peer_sender)
        - Runs handler logic for peer.

    [1 invocation per peer] Add authorize() which authorizes the receiver for a peer.
        - Clones 1 instance of peer_sender per authorization.
        - Clone 1 instance of the channel (okay to clone as it is a read-only channel) per authorization
        - Returns a (channel, peer_sender) per authorization

     */

    ///
    /// Sets the peer sender in this receive handler.
    ///
    #[inline]
    pub fn initialize_peer_sender(&mut self, peer_sender: Arc<RwLock<PeerSender>>) -> Result<(), NetworkError> {
        // Check that the peer sender has not already been initialized.
        if self.peer_sender.is_some() {
            trace!("Peer sender was already set with the receive handler");
            return Err(NetworkError::ReceiveHandlerAlreadySetPeerSender);
        }

        // Set the peer sender in this receive handler.
        self.peer_sender = Some(peer_sender);

        trace!("Initialized the peer sender with the receive handler");
        Ok(())
    }

    // TODO (howardwu): Remove environment from function inputs.
    #[inline]
    pub async fn listen(self, environment: Environment) -> Result<(), NetworkError> {
        info!("a {:?}", self.peer_sender);

        let peer_sender = match self.peer_sender {
            Some(ref peer_sender) => peer_sender.clone(),
            None => return Err(NetworkError::ReceiveHandlerMissingPeerSender),
        };

        info!("b");

        // TODO (howardwu): Remove this peer manager instance for this function.
        let peer_manager = match environment.peer_manager {
            Some(ref peer_manager) => peer_manager.clone(),
            _ => return Err(NetworkError::ReceiveHandlerMissingPeerSender),
        };

        // TODO (howardwu): Find the actual address of this node.
        // 1. Initialize TCP listener and accept new TCP connections.
        let local_address = peer_manager.clone().read().await.local_address();
        debug!("Starting listener at {:?}...", local_address);
        let mut listener = TcpListener::bind(&local_address).await?;
        info!("Listening at {:?}", local_address);

        // task::spawn(async move {
        debug!("Starting thread for handling connection requests");
        /// Spawns one thread per peer tcp connection to read messages.
        /// Each thread is given a handle to the channel and a handle to the server mpsc sender.
        /// To ensure concurrency, each connection thread sends a tokio oneshot sender handle with every message to the server mpsc receiver.
        /// The thread then waits for the oneshot receiver to receive a signal from the server before reading again.

        // TODO (howardwu): Move this to an outer scope controlled by a manager.
        // Determines the criteria for disconnecting from a peer.
        fn should_disconnect(failure_count: &u8) -> bool {
            // Tolerate up to 10 failed communications.
            *failure_count >= 10
        }

        // TODO (howardwu): Move this to an outer scope controlled by a manager.
        // Logs the failure and determines whether to disconnect from a peer.
        async fn handle_failure<T: Display>(
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
            trace!("Starting listener");

            // Start listener for handling connection requests.
            let (channel, remote_address) = match listener.accept().await {
                Ok((channel, remote_address)) => {
                    info!("Received connection request from {}", remote_address);
                    (channel, remote_address)
                }
                Err(error) => {
                    error!("Failed to accept connection request\n{}", error);
                    continue;
                }
            };

            // TODO (howardwu): Move to peer manager.
            {
                // Fetch the current number of connected peers.
                let number_of_connected_peers = peer_manager.read().await.number_of_connected_peers().await;
                trace!("Connected with {} peers", number_of_connected_peers);

                // Check that the maximum number of peers has not been reached.
                if number_of_connected_peers >= environment.maximum_number_of_peers() {
                    warn!("Maximum number of peers is reached, this connection request is being dropped");
                    match channel.shutdown(Shutdown::Write) {
                        Ok(_) => {
                            debug!("Closed connection with {}", remote_address);
                            continue;
                        }
                        // TODO (howardwu): Evaluate whether to return this error, or silently continue.
                        Err(error) => {
                            error!("Failed to close connection with {}\n{}", remote_address, error);
                            continue;
                        }
                    }
                }
            }

            // TODO (howardwu): Determine whether to move to either peer manager or sync manager.
            {
                // Follow handshake protocol and drop peer connection if unsuccessful.
                let height = environment.current_block_height().await;

                // TODO (raychu86) Establish a formal node version
                if let Some((channel, discovered_local_address, version_message)) = self
                    // if let Some((handshake, discovered_local_address, version_message)) = self
                        .receive_connection_request(&environment, 1u64, height, remote_address, channel)
                        .await.unwrap()
                {
                    // TODO (howardwu): Enable this peer address discovery again.
                    // // Bootstrap discovery of local node IP via VERACK responses
                    // {
                    //     let local_address = peer_manager.local_address();
                    //     if local_address != discovered_local_address {
                    //         peer_manager.set_local_address(discovered_local_address).await;
                    //         info!("Discovered local address: {:?}", local_address);
                    //     }
                    // }
                    // // Store the channel established with the handshake
                    // peer_manager.add_channel(&handshake.channel);

                    // TODO (howardwu): Enable this sync logic if block height is lower than peer again.
                    // if let Some(version) = version_message {
                    //     // If our peer has a longer chain, send a sync message
                    //     if version.height > environment.current_block_height().await {
                    //         // Update the sync node if the sync_handler is Idle
                    //         if let Ok(mut sync_handler) = sync_manager.try_lock() {
                    //             if !sync_handler.is_syncing() {
                    //                 sync_handler.sync_node_address = handshake.channel.address;
                    //
                    //                 if let Ok(block_locator_hashes) =
                    //                     environment.storage_read().await.get_block_locator_hashes()
                    //                 {
                    //                     if let Err(err) =
                    //                         handshake.channel.write(&GetSync::new(block_locator_hashes)).await
                    //                     {
                    //                         error!(
                    //                             "Error sending GetSync message to {}, {}",
                    //                             handshake.channel.address, err
                    //                         );
                    //                     }
                    //                 }
                    //             }
                    //         }
                    //     }
                    // }

                    // TODO (howardwu): Delete me.
                    // // Inner loop spawns one thread per connection to read messages
                    // Self::spawn_connection_thread(handshake.channel.clone(), sender.clone());

                    // TODO (howardwu): Attention to this.
                    let mut channel = channel;

                    loop {
                        /// TODO (howardwu): Evaluate this.
                        ///
                        ///
                        /// POTENTIALLY ADD A `LOOP` in a `TASK::SPAWN`. See `spawn_connection_thread`.
                        /// If you add, add it until the very end of this function.
                        ///
                        ///
                        // Initialize the failure indicator.
                        let mut failure = false;

                        // Read the next message from the channel. This is a blocking operation.
                        let (message_name, message_bytes) = match channel.read().await {
                            Ok((message_name, message_bytes)) => (message_name, message_bytes),
                            Err(error) => {
                                handle_failure(&mut failure, &mut failure_count, &mut disconnect_from_peer, error)
                                    .await;

                                // Determine if we should send a disconnect message.
                                match disconnect_from_peer {
                                    true => (MessageName::from("disconnect"), vec![]),
                                    false => continue,
                                }
                            }
                        };

                        // TODO (howardwu): Filter and route message for either peer manager or sync manager.
                        //  For now, all messages go to the peer manager via `peer_sender`.
                        {
                            /// This method handles all messages sent from connected peers.
                            ///
                            /// Messages are received by a single tokio MPSC receiver with
                            /// the message name, bytes, associated channel, and a tokio oneshot sender.
                            ///
                            /// The oneshot sender lets the connection thread know when the message is handled.
                            ///
                            let name = message_name;
                            let bytes = message_bytes;
                            let environment = &environment;

                            if name == Block::name() {
                                if let Ok(block) = Block::deserialize(bytes) {
                                    if let Err(err) = self
                                        .receive_block_message(environment, block, channel.clone(), true)
                                        .await
                                    {
                                        error!(
                                            "Receive handler errored when receiving a {} message from {}. {}",
                                            name, remote_address, err
                                        );
                                    }
                                }
                            } else if name == GetBlock::name() {
                                if let Ok(getblock) = GetBlock::deserialize(bytes) {
                                    if let Err(err) =
                                        self.receive_get_block(environment, getblock, channel.clone()).await
                                    {
                                        error!(
                                            "Receive handler errored when receiving a {} message from {}. {}",
                                            name, remote_address, err
                                        );
                                    }
                                }
                            } else if name == GetMemoryPool::name() {
                                if let Ok(getmemorypool) = GetMemoryPool::deserialize(bytes) {
                                    if let Err(err) = self
                                        .receive_get_memory_pool(environment, getmemorypool, channel.clone())
                                        .await
                                    {
                                        error!(
                                            "Receive handler errored when receiving a {} message from {}. {}",
                                            name, remote_address, err
                                        );
                                    }
                                }
                            } else if name == GetPeers::name() {
                                if let Ok(getpeers) = GetPeers::deserialize(bytes) {
                                    if let Err(err) =
                                        self.receive_get_peers(environment, getpeers, channel.clone()).await
                                    {
                                        error!(
                                            "Receive handler errored when receiving a {} message from {}. {}",
                                            name, remote_address, err
                                        );
                                    }
                                }
                            } else if name == GetSync::name() {
                                if let Ok(getsync) = GetSync::deserialize(bytes) {
                                    if let Err(err) = self.receive_get_sync(environment, getsync, channel.clone()).await
                                    {
                                        error!(
                                            "Receive handler errored when receiving a {} message from {}. {}",
                                            name, remote_address, err
                                        );
                                    }
                                }
                            } else if name == MemoryPool::name() {
                                if let Ok(mempool) = MemoryPool::deserialize(bytes) {
                                    if let Err(err) = self.receive_memory_pool(environment, mempool).await {
                                        error!(
                                            "Receive handler errored when receiving a {} message from {}. {}",
                                            name, remote_address, err
                                        );
                                    }
                                }
                            } else if name == Peers::name() {
                                if let Ok(peers) = Peers::deserialize(bytes) {
                                    if let Err(err) = self.receive_peers(environment, peers, channel.clone()).await {
                                        error!(
                                            "Receive handler errored when receiving a {} message from {}. {}",
                                            name, remote_address, err
                                        );
                                    }
                                }
                            } else if name == Ping::name() {
                                if let Ok(ping) = Ping::deserialize(bytes) {
                                    if let Err(err) = self.receive_ping(environment, ping, channel.clone()).await {
                                        error!(
                                            "Receive handler errored when receiving a {} message from {}. {}",
                                            name, remote_address, err
                                        );
                                    }
                                }
                            } else if name == Pong::name() {
                                if let Ok(pong) = Pong::deserialize(bytes) {
                                    if let Err(err) = self.receive_pong(environment, pong, channel.clone()).await {
                                        error!(
                                            "Receive handler errored when receiving a {} message from {}. {}",
                                            name, remote_address, err
                                        );
                                    }
                                }
                            } else if name == Sync::name() {
                                if let Ok(sync) = Sync::deserialize(bytes) {
                                    if let Err(err) = self.receive_sync(environment, sync).await {
                                        error!(
                                            "Receive handler errored when receiving a {} message from {}. {}",
                                            name, remote_address, err
                                        );
                                    }
                                }
                            } else if name == SyncBlock::name() {
                                if let Ok(block) = Block::deserialize(bytes) {
                                    if let Err(err) = self
                                        .receive_block_message(environment, block, channel.clone(), false)
                                        .await
                                    {
                                        error!(
                                            "Receive handler errored when receiving a {} message from {}. {}",
                                            name, remote_address, err
                                        );
                                    }
                                }
                            } else if name == Transaction::name() {
                                if let Ok(transaction) = Transaction::deserialize(bytes) {
                                    if let Err(err) = self
                                        .receive_transaction(environment, transaction, channel.clone())
                                        .await
                                    {
                                        error!(
                                            "Receive handler errored when receiving a {} message from {}. {}",
                                            name, remote_address, err
                                        );
                                    }
                                }
                            } else if name == Version::name() {
                                if let Ok(version) = Version::deserialize(bytes) {
                                    // TODO (raychu86) Does `receive_version` need to return a channel?
                                    match self.receive_version(environment, version, channel.clone()).await {
                                        Ok(returned_channel) => channel = returned_channel,
                                        Err(err) => error!(
                                            "Receive handler errored when receiving a {} message from {}. {}",
                                            name, remote_address, err
                                        ),
                                    }
                                }
                            } else if name == Verack::name() {
                                if let Ok(verack) = Verack::deserialize(bytes) {
                                    if !self.receive_verack(environment, verack, channel.clone()).await {
                                        error!(
                                            "Receive handler errored when receiving a {} message from {}",
                                            name, remote_address
                                        );
                                    }
                                }
                            } else if name == MessageName::from("disconnect") {
                                info!("Disconnected from peer {:?}", remote_address);
                                {
                                    let mut peer_manager = environment.peer_manager_write().await;
                                    peer_manager.disconnect_from_peer(&remote_address).await.unwrap();
                                }
                            } else {
                                debug!("Message name not recognized {:?}", name.to_string());
                            }

                            // if let Err(error) = tx.send(channel) {
                            //     warn!("Error resetting connection thread ({:?})", error);
                            // }
                        }

                        // // Use a oneshot channel to give the channel control
                        // // to the message handler after reading from the channel.
                        // let (tx, rx) = oneshot::channel();

                        // // Send the successful read data to the message handler.
                        // if let Err(error) = peer_sender
                        //     .send((tx, message_name, message_bytes, channel.clone())) // TODO (howardwu): Remove this `tx` here
                        //     .await
                        // {
                        //     handle_failure(&mut failure, &mut failure_count, &mut disconnect_from_peer, error).await;
                        //     continue;
                        // };

                        // // Wait for the message handler to give back channel control.
                        // match rx.await {
                        //     Ok(peer_channel) => channel = peer_channel,
                        //     Err(error) => {
                        //         handle_failure(&mut failure, &mut failure_count, &mut disconnect_from_peer, error).await
                        //     }
                        // };

                        // TODO (howardwu): Remove this and rearchitect how disconnects are handled using the peer manager.
                        // TODO (howardwu): Implement a handler so the node does not lose state of undetected disconnects.
                        // Break out of the loop if the peer disconnects.
                        if disconnect_from_peer {
                            warn!("Disconnecting from an unreliable peer");
                            break;
                        }
                    }
                }
            };
        }
        // });

        Ok(())
    }

    /// A peer has sent us a new block to process.
    async fn receive_block_message(
        &self,
        environment: &Environment,
        message: Block,
        channel: Arc<Channel>,
        propagate: bool,
    ) -> Result<(), NetworkError> {
        let block = BlockStruct::deserialize(&message.data)?;

        info!(
            "Received a block from epoch {} with hash {:?}",
            block.header.time,
            hex::encode(block.header.get_hash().0)
        );

        // Verify the block and insert it into the storage.
        if !environment
            .storage_read()
            .await
            .block_hash_exists(&block.header.get_hash())
        {
            {
                let mut memory_pool = environment.memory_pool().lock().await;
                let inserted = environment
                    .consensus_parameters()
                    .receive_block(
                        environment.dpc_parameters(),
                        &*environment.storage_read().await,
                        &mut memory_pool,
                        &block,
                    )
                    .is_ok();

                if inserted && propagate {
                    // This is a new block, send it to our peers.
                    environment
                        .peer_manager_read()
                        .await
                        .propagate_block(message.data, channel.remote_address)
                        .await?;
                } else if !propagate {
                    if let Ok(mut sync_manager) = environment.sync_manager().await.try_lock() {
                        // TODO (howardwu): Implement this.
                        {
                            // sync_manager.clear_pending().await;
                            //
                            // if sync_manager.sync_state != SyncState::Idle {
                            //     // We are currently syncing with a node, ask for the next block.
                            //     if let Some(channel) = environment
                            //         .peer_manager_read()
                            //         .await
                            //         .get_channel(&sync_manager.sync_node_address)
                            //     {
                            //         sync_manager.increment(channel.clone()).await?;
                            //     }
                            // }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// A peer has requested a block.
    async fn receive_get_block(
        &self,
        environment: &Environment,
        message: GetBlock,
        channel: Arc<Channel>,
    ) -> Result<(), NetworkError> {
        if let Ok(block) = environment.storage_read().await.get_block(&message.block_hash) {
            channel.write(&SyncBlock::new(block.serialize()?)).await?;
        }

        Ok(())
    }

    /// A peer has requested our memory pool transactions.
    async fn receive_get_memory_pool(
        &self,
        environment: &Environment,
        _message: GetMemoryPool,
        channel: Arc<Channel>,
    ) -> Result<(), NetworkError> {
        let memory_pool = environment.memory_pool().lock().await;

        let mut transactions = vec![];

        for (_tx_id, entry) in &memory_pool.transactions {
            if let Ok(transaction_bytes) = to_bytes![entry.transaction] {
                transactions.push(transaction_bytes);
            }
        }

        if !transactions.is_empty() {
            channel.write(&MemoryPool::new(transactions)).await?;
        }

        Ok(())
    }

    /// A peer has sent us their memory pool transactions.
    async fn receive_memory_pool(&self, environment: &Environment, message: MemoryPool) -> Result<(), NetworkError> {
        let mut memory_pool = environment.memory_pool().lock().await;

        for transaction_bytes in message.transactions {
            let transaction: Tx = Tx::read(&transaction_bytes[..])?;
            let entry = Entry::<Tx> {
                size_in_bytes: transaction_bytes.len(),
                transaction,
            };

            if let Ok(inserted) = memory_pool.insert(&*environment.storage_read().await, entry) {
                if let Some(txid) = inserted {
                    debug!("Transaction added to memory pool with txid: {:?}", hex::encode(txid));
                }
            }
        }

        Ok(())
    }

    /// A node has requested our list of peer addresses.
    /// Send an Address message with our current peer list.
    async fn receive_get_peers(
        &self,
        environment: &Environment,
        _message: GetPeers,
        channel: Arc<Channel>,
    ) -> Result<(), NetworkError> {
        // If we received a message, but aren't connected to the peer,
        // inform the peer book that we found a peer.
        // The peer book will determine if we have seen the peer before,
        // and include the peer if it is new.
        let peer_manager = environment.peer_manager_write().await;
        if !peer_manager.is_connected(&channel.remote_address).await {
            peer_manager.found_peer(&channel.remote_address);
        }

        // Broadcast the sanitized list of connected peers back to requesting peer.
        let mut peers = HashMap::new();
        for (remote_address, peer_info) in peer_manager.connected_peers().await {
            // Skip the iteration if the requesting peer that we're sending the response to
            // appears in the list of peers.
            if remote_address == channel.remote_address {
                continue;
            }
            peers.insert(remote_address, *peer_info.last_seen());
        }
        channel.write(&Peers::new(peers)).await?;

        Ok(())
    }

    /// A miner has sent their list of peer addresses.
    /// Add all new/updated addresses to our disconnected.
    /// The connection handler will be responsible for sending out handshake requests to them.
    async fn receive_peers(
        &self,
        environment: &Environment,
        message: Peers,
        channel: Arc<Channel>,
    ) -> Result<(), NetworkError> {
        // If we received a message, but aren't connected to the peer,
        // inform the peer book that we found a peer.
        // The peer book will determine if we have seen the peer before,
        // and include the peer if it is new.
        let peer_manager = environment.peer_manager_write().await;
        if !peer_manager.is_connected(&channel.remote_address).await {
            peer_manager.found_peer(&channel.remote_address);
        }

        // Process all of the peers sent in the message,
        // by informing the peer book of that we found peers.
        let local_address = *environment.local_address();

        for (peer_address, _) in message.addresses.iter() {
            // Skip if the peer address is this node's local address.
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
            else if !peer_manager.is_connected(&channel.remote_address).await {
                peer_manager.found_peer(&channel.remote_address);
            }
        }

        Ok(())
    }

    /// A peer has sent us a ping message.
    /// Reply with a pong message.
    async fn receive_ping(
        &self,
        environment: &Environment,
        message: Ping,
        channel: Arc<Channel>,
    ) -> Result<(), NetworkError> {
        // If we received a ping, but aren't connected to the peer,
        // inform the peer book that we found a peer.
        // The peer book will determine if we have seen the peer before,
        // and include the peer if it is new.
        let peer_manager = environment.peer_manager_write().await;
        if !peer_manager.is_connected(&channel.remote_address).await {
            peer_manager.found_peer(&channel.remote_address);
        }

        PingPongManager::send_pong(message, channel).await?;
        Ok(())
    }

    /// A peer has sent us a pong message.
    /// Check if it matches a ping we sent out.
    async fn receive_pong(
        &self,
        environment: &Environment,
        message: Pong,
        channel: Arc<Channel>,
    ) -> Result<(), NetworkError> {
        // If we received a pong, but aren't connected to the peer,
        // inform the peer book that we found a peer.
        // The peer book will determine if we have seen the peer before,
        // and include the peer if it is new.
        let peer_manager = environment.peer_manager_write().await;
        if !peer_manager.is_connected(&channel.remote_address).await {
            peer_manager.found_peer(&channel.remote_address);
        }

        if let Err(error) = environment
            .ping_pong()
            .write()
            .await
            .accept_pong(channel.remote_address, message)
            .await
        {
            debug!(
                "Invalid pong message from: {:?}, Full error: {:?}",
                channel.remote_address, error
            )
        }

        Ok(())
    }

    /// A peer has requested our chain state to sync with.
    async fn receive_get_sync(
        &self,
        environment: &Environment,
        message: GetSync,
        channel: Arc<Channel>,
    ) -> Result<(), NetworkError> {
        let latest_shared_hash = environment
            .storage_read()
            .await
            .get_latest_shared_hash(message.block_locator_hashes)?;
        let current_height = environment.storage_read().await.get_current_block_height();

        if let Ok(height) = environment.storage_read().await.get_block_number(&latest_shared_hash) {
            if height < current_height {
                let mut max_height = current_height;

                // if the requester is behind more than 4000 blocks
                if height + 4000 < current_height {
                    // send the max 4000 blocks
                    max_height = height + 4000;
                }

                let mut block_hashes: Vec<BlockHeaderHash> = vec![];

                for block_num in height + 1..=max_height {
                    block_hashes.push(environment.storage_read().await.get_block_hash(block_num)?);
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
    async fn receive_sync(&self, environment: &Environment, message: Sync) -> Result<(), NetworkError> {
        let height = environment.storage_read().await.get_current_block_height();
        let mut sync_handler = environment.sync_manager().await.lock().await;

        sync_handler.receive_hashes(message.block_hashes, height);

        // TODO (howardwu): Implement this using the sync manager and send handler.
        {
            // // Received block headers
            // if let Some(channel) = environment
            //     .peer_manager_read()
            //     .await
            //     .get_channel(&sync_handler.sync_node_address)
            // {
            //     sync_handler.increment(channel.clone()).await?;
            // }
        }

        Ok(())
    }

    /// A peer has sent us a transaction.
    async fn receive_transaction(
        &self,
        environment: &Environment,
        message: Transaction,
        channel: Arc<Channel>,
    ) -> Result<(), NetworkError> {
        environment
            .peer_manager_read()
            .await
            .process_transaction_internal(
                &environment,
                environment.consensus_parameters(),
                environment.dpc_parameters(),
                environment.storage(),
                environment.memory_pool(),
                message.bytes,
                channel.remote_address,
            )
            .await?;

        Ok(())
    }

    /// A connected peer has acknowledged a handshake request.
    /// Check if the Verack matches the last handshake message we sent.
    /// Update our peer book and send a request for more peers.
    async fn receive_verack(&self, environment: &Environment, message: Verack, channel: Arc<Channel>) -> bool {
        {
            // // TODO (howardwu): Quarantined code. Inspect and integrate.
            // /// Accepts a handshake response from a connected peer.
            // pub async fn accept_response(
            //     &self,
            //     environment: &Environment,
            //     remote_address: SocketAddr,
            //     message: Verack,
            // ) -> bool {
            //     // ORIGINAL CODE
            //
            //     // match environment.handshakes().write().await.get_mut(&remote_address) {
            //     //     Some(handshake) => {
            //     //         debug!("New handshake with {:?}", remote_address);
            //     //         handshake.accept(message).await.is_ok()
            //     //     }
            //     //     None => false,
            //     // }
            //
            //     // RENDERED CODE
            //
            //     /// If the nonce matches, accepts a given verack message from a peer.
            //     /// Else, returns a `HandshakeError`.
            //     if self.nonce != message.nonce {
            //         self.state = HandshakeStatus::Rejected;
            //         return Err(HandshakeError::InvalidNonce(self.nonce, message.nonce));
            //     } else if self.state == HandshakeStatus::Waiting {
            //         self.state = HandshakeStatus::Accepted;
            //     }
            //     Ok(())
            // }
        }

        // TODO (howardwu): Implement this.
        {
            // match self.accept_response(environment, channel.remote_address, message).await {
            //     true => {
            //         // If we received a verack, but aren't connected to the peer,
            //         // inform the peer book that we found a peer.
            //         // The peer book will determine if we have seen the peer before,
            //         // and include the peer if it is new.
            //         let peer_manager = environment.peer_manager_write().await;
            //         if !peer_manager.is_connected(&channel.remote_address).await {
            //             peer_manager.found_peer(&channel.remote_address);
            //         }
            //         // Ask connected peer for more peers.
            //         channel.write(&GetPeers).await.is_ok()
            //     }
            //     false => {
            //         debug!("Received an invalid verack message from {:?}", channel.remote_address);
            //         false
            //     }
            // }
        }

        // TODO (howardwu): Remove this. Temporary solution.
        {
            // If we received a verack, but aren't connected to the peer,
            // inform the peer book that we found a peer.
            // The peer book will determine if we have seen the peer before,
            // and include the peer if it is new.
            let peer_manager = environment.peer_manager_write().await;
            if peer_manager.is_connecting(&channel.remote_address).await {
                // TODO (howardwu): Implement this in PeerManager.
                {
                    // peer_manager.set_connected(&channel.remote_address);
                }
            }
            // Ask connected peer for more peers.
            return channel.write(&GetPeers).await.is_ok();
        }
    }

    /// A connected peer has sent handshake request.
    /// Update peer's channel.
    /// If peer's block height is greater than ours, send a sync request.
    ///
    /// This method may seem redundant to handshake protocol functions but a peer can send additional
    /// Version messages if they want to update their ip address/port or want to share their chain height.
    async fn receive_version(
        &self,
        environment: &Environment,
        message: Version,
        channel: Arc<Channel>,
    ) -> Result<Arc<Channel>, NetworkError> {
        let peer_address = SocketAddr::new(channel.remote_address.ip(), message.sender.port());

        let peer_manager = environment.peer_manager_read().await;

        if *environment.local_address() != peer_address {
            if peer_manager.number_of_connected_peers().await < environment.maximum_number_of_peers() {
                self.receive_request(environment, message.clone(), peer_address).await;
            }

            // If our peer has a longer chain, send a sync message
            if message.height > environment.storage_read().await.get_current_block_height() {
                debug!("Received a version message with a greater height {}", message.height);
                // Update the sync node if the sync_handler is Idle and there are no requested block headers
                if let Ok(mut sync_handler) = environment.sync_manager().await.try_lock() {
                    if !sync_handler.is_syncing()
                        && (sync_handler.block_headers.len() == 0 && sync_handler.pending_blocks.is_empty())
                    {
                        debug!("Attempting to sync with peer {}", peer_address);
                        sync_handler.sync_node_address = peer_address;

                        if let Ok(block_locator_hashes) = environment.storage_read().await.get_block_locator_hashes() {
                            channel.write(&GetSync::new(block_locator_hashes)).await?;
                        }
                    } else {
                        // TODO (howardwu): Implement this.
                        {
                            // if let Some(channel) = environment
                            //     .peer_manager_read()
                            //     .await
                            //     .get_channel(&sync_handler.sync_node_address)
                            // {
                            //     sync_handler.increment(channel.clone()).await?;
                            // }
                        }
                    }
                }
            }
        }
        Ok(channel)
    }

    // MOVED FROM SEND HANDLER HERE.

    ///
    /// Receives a connection request with a given version message.
    ///
    /// Listens for the first message request from a remote peer.
    ///
    /// If the message is a Version:
    ///
    ///     1. Create a new handshake.
    ///     2. Send a handshake response.
    ///     3. If the response is sent successfully, store the handshake.
    ///     4. Return the handshake, your address as seen by sender, and the version message.
    ///
    /// If the message is a Verack:
    ///
    ///     1. Get the existing handshake.
    ///     2. Mark the handshake as accepted.
    ///     3. Send a request for peers.
    ///     4. Return the accepted handshake and your address as seen by sender.
    ///
    /// TODO (howardwu): Fix the return type so it does not return Result<Option<T>>.
    #[inline]
    pub async fn receive_connection_request(
        &self,
        environment: &Environment,
        version: u64,
        block_height: u32,
        remote_address: SocketAddr,
        reader: TcpStream,
        // ) -> Result<Option<(Handshake, SocketAddr, Option<Version>)>, NetworkError> {
    ) -> Result<Option<(Arc<Channel>, SocketAddr, Option<Version>)>, NetworkError> {
        // Read the first message or return `None`.
        let channel = Channel::new_reader(reader);
        // Parse the inbound message into the message name and message bytes.
        let (channel, (message_name, message_bytes)) = match channel {
            // Read the next message from the channel.
            // Note this is a blocking operation.
            Ok(channel) => match channel.read().await {
                Ok(inbound_message) => (channel, inbound_message),
                _ => return Ok(None),
            },
            _ => return Ok(None),
        };

        // Handles a version message request.
        // Create and store a new handshake in the manager.
        if message_name == Version::name() {
            // Deserialize the message bytes into a version message.
            let remote_version = match Version::deserialize(message_bytes) {
                Ok(remote_version) => remote_version,
                _ => return Ok(None),
            };
            let local_address = remote_version.receiver;
            // Create the remote address from the given peer address, and specified port from the version message.
            let remote_address = SocketAddr::new(remote_address.ip(), remote_version.sender.port());
            // Create the local version message.
            let local_version = Version::new_with_rng(version, block_height, local_address, remote_address);

            // TODO (howardwu): Quarantined code. Inspect and integrate.
            // Process the new version message and send a response to the remote peer.
            {
                // ORIGINAL CODE

                // let handshake = match Handshake::receive_new(channel, &local_version, &remote_version).await {
                //     Ok(handshake) => handshake,
                //     _ => return None,
                // };

                // RENDERED CODE

                // Connect to the remote address.
                let remote_address = local_version.receiver;
                let channel = channel.update_writer(remote_address).await?;
                // Write a verack response to the remote peer.
                let local_address = local_version.sender;
                let remote_nonce = remote_version.nonce;
                channel
                    .write(&Verack::new(remote_nonce, local_address, remote_address))
                    .await?;
                // Write version request to the remote peer.
                channel.write(&local_version).await?;

                // TODO (howardwu): Add the handshake nonce to the peer manager, or cross-reference it
                //  for validity.
                // Ok(Self {
                //     channel: Arc::new(channel),
                //     state: HandshakeStatus::Waiting,
                //     height: local_version.height,
                //     nonce: local_version.nonce,
                // })
            }

            debug!("Received handshake from {:?}", remote_address);

            // Acquire the channels write lock.
            let mut channels = self.channels.write().await;
            // Store the new channel.
            let channel = Arc::new(channel);
            channels.insert(local_address, channel.clone());

            // // Acquire the handshake write lock.
            // let mut handshakes = environment.handshakes().write().await;
            // // Store the new handshake.
            // handshakes.insert(remote_address, handshake.clone());
            // // Drop the handshakes write lock.
            // drop(handshakes);
            // return Ok(Some((handshake, local_address, Some(local_version))));
            return Ok(Some((channel, local_address, Some(local_version))));
        }

        // Handles a verack message request.
        // Establish the channel with the remote peer.
        if message_name == Verack::name() {
            // Deserialize the message bytes into a verack message.
            let verack = match Verack::deserialize(message_bytes) {
                Ok(verack) => verack,
                _ => return Ok(None),
            };
            {
                let local_address = verack.receiver;

                // TODO (howardwu): Check whether this remote address needs to
                //   be derive the same way as the version message case above
                //  (using a remote_address.ip() and address_sender.port()).
                let remote_address = verack.sender;

                // REPLACED THIS.

                // // Acquire the handshake write lock.
                // let mut handshakes = environment.handshakes().write().await;
                // // Accept the handshake with the remote address.
                // let result = match handshakes.get_mut(&remote_address) {
                //     Some(handshake) => match handshake.accept(verack).await {
                //         Ok(()) => {
                //             handshake.update_reader(channel);
                //             info!("New handshake with {:?}", remote_address);
                //             Some((handshake.clone(), local_address, None))
                //         }
                //         _ => Ok(None),
                //     },
                //     _ => Ok(None),
                // };
                // // Drop the handshakes write lock.
                // drop(handshakes);
                //
                // return Ok(result);

                // WITH THIS.

                // Acquire the channels write lock.
                let mut channels = self.channels.write().await;
                // Store the new channel.
                let channel = Arc::new(channel);
                channels.insert(remote_address, channel.clone());

                return Ok(Some((channel, local_address, None)));
            }
        }

        Ok(None)
    }

    // TODO (howardwu): Quarantined code. Inspect and integrate.
    /// Receives a handshake request from a connected peer.
    /// Updates the handshake channel address, if needed.
    /// Sends a handshake response back to the connected peer.
    pub async fn receive_request(
        &self,
        environment: &Environment,
        message: Version,
        remote_address: SocketAddr,
    ) -> bool {
        // ORIGINAL CODE

        // match environment.handshakes().write().await.get_mut(&remote_address) {
        //     Some(handshake) => {
        //         handshake.update_address(remote_address);
        //         handshake.receive(message).await.is_ok()
        //     }
        //     None => false,
        // }

        // RENDERED CODE

        /// Receives the version message from a connected peer,
        /// and sends a verack message to acknowledge back.
        // You are the new sender and your peer is the receiver
        let address_receiver = remote_address;
        let address_sender = message.receiver;
        // self.channel
        //     .write(&)
        //     .await
        //     .is_ok()
        environment
            .send_handler()
            .broadcast(&Request::Verack(Verack::new(
                message.nonce,
                address_sender,
                address_receiver,
            )))
            .await
            .is_ok()
    }

    // #[inline]
    // pub fn peer_listener() {
    //     task::spawn(async move {
    //         // TODO (howardwu): Move this to an outer scope controlled by a manager.
    //         // Determines the criteria for disconnecting from a peer.
    //         fn should_disconnect(failure_count: &u8) -> bool {
    //             // Tolerate up to 10 failed communications.
    //             *failure_count >= 10
    //         }
    //
    //         // TODO (howardwu): Move this to an outer scope controlled by a manager.
    //         // Logs the failure and determines whether to disconnect from a peer.
    //         async fn handle_failure<T: Display>(
    //             failure: &mut bool,
    //             failure_count: &mut u8,
    //             disconnect_from_peer: &mut bool,
    //             error: T,
    //         ) {
    //             // Only increment failure_count if we haven't seen a failure yet.
    //             if !*failure {
    //                 // Update the state to reflect a new failure.
    //                 *failure = true;
    //                 *failure_count += 1;
    //                 warn!(
    //                     "Connection errored {} time(s) (error message: {})",
    //                     failure_count, error
    //                 );
    //
    //                 // Determine if we should disconnect.
    //                 *disconnect_from_peer = should_disconnect(failure_count);
    //             } else {
    //                 debug!("Connection errored again in the same loop (error message: {})", error);
    //             }
    //
    //             // Sleep for 10 seconds
    //             tokio::time::delay_for(std::time::Duration::from_secs(10)).await;
    //         }
    //
    //         let mut failure_count = 0u8;
    //         let mut disconnect_from_peer = false;
    //
    //         loop {
    //             // Initialize the failure indicator.
    //             let mut failure = false;
    //
    //             // Read the next message from the channel. This is a blocking operation.
    //             let (message_name, message_bytes) = match channel.read().await {
    //                 Ok((message_name, message_bytes)) => (message_name, message_bytes),
    //                 Err(error) => {
    //                     handle_failure(&mut failure, &mut failure_count, &mut disconnect_from_peer, error).await;
    //
    //                     // Determine if we should send a disconnect message.
    //                     match disconnect_from_peer {
    //                         true => (MessageName::from("disconnect"), vec![]),
    //                         false => continue,
    //                     }
    //                 }
    //             };
    //
    //             // TODO (howardwu): Remove this and rearchitect for unidirectional progression.
    //             // Use a oneshot channel to give the channel control
    //             // to the message handler after reading from the channel.
    //             let (tx, rx) = oneshot::channel();
    //
    //             // Send the successful read data to the message handler.
    //             if let Err(error) = sender.send((tx, message_name, message_bytes, channel.clone())).await {
    //                 handle_failure(&mut failure, &mut failure_count, &mut disconnect_from_peer, error).await;
    //                 continue;
    //             };
    //
    //             // Wait for the message handler to give back channel control.
    //             match rx.await {
    //                 Ok(peer_channel) => channel = peer_channel,
    //                 Err(error) => {
    //                     handle_failure(&mut failure, &mut failure_count, &mut disconnect_from_peer, error).await
    //                 }
    //             };
    //
    //             // TODO (howardwu): Implement a handler so the node does not lose state of undetected disconnects.
    //             // Break out of the loop if the peer disconnects.
    //             if disconnect_from_peer {
    //                 warn!("Disconnecting from an unreliable peer");
    //                 break;
    //             }
    //         }
    //     });
    // }

    // pub async fn listener(environment: Environment) {
    //     let sender = self.sender.clone();
    //     let mut peer_manager = PeerManager::new(environment.clone()).await?;
    //     let sync_manager = self.environment.sync_manager().await.clone();
    //
    //     // Prepare to spawn the main loop.
    //     // let environment = environment.clone();
    //     // // let mut peer_manager = self.peer_manager.clone();
    //     // let peer_manager_og = PeerManager::new(environment.clone()).await?;
    //
    //     // TODO (howardwu): Find the actual address of this node.
    //     // 1. Initialize TCP listener and accept new TCP connections.
    //     let local_address = peer_manager_og.local_address();
    //     debug!("Starting listener at {:?}...", local_address);
    //     let mut listener = TcpListener::bind(&local_address).await?;
    //     info!("Listening at {:?}", local_address);
    //
    //     // 2. Spawn a new thread to handle new connections.
    //     task::spawn(async move {
    //         debug!("Starting thread for handling connection requests");
    //         loop {
    //             // Start listener for handling connection requests.
    //             let (reader, remote_address) = match listener.accept().await {
    //                 Ok((reader, remote_address)) => {
    //                     info!("Received connection request from {}", remote_address);
    //                     (reader, remote_address)
    //                 }
    //                 Err(error) => {
    //                     error!("Failed to accept connection request\n{}", error);
    //                     continue;
    //                 }
    //             };
    //
    //             // Fetch the current number of connected peers.
    //             let number_of_connected_peers = peer_manager.number_of_connected_peers().await;
    //             trace!("Connected with {} peers", number_of_connected_peers);
    //
    //             // Check that the maximum number of peers has not been reached.
    //             if number_of_connected_peers >= environment.maximum_number_of_peers() {
    //                 warn!("Maximum number of peers is reached, this connection request is being dropped");
    //                 match reader.shutdown(Shutdown::Write) {
    //                     Ok(_) => {
    //                         debug!("Closed connection with {}", remote_address);
    //                         continue;
    //                     }
    //                     // TODO (howardwu): Evaluate whether to return this error, or silently continue.
    //                     Err(error) => {
    //                         error!("Failed to close connection with {}\n{}", remote_address, error);
    //                         continue;
    //                     }
    //                 }
    //             }
    //
    //             // Follow handshake protocol and drop peer connection if unsuccessful.
    //             let height = environment.current_block_height().await;
    //
    //             // TODO (raychu86) Establish a formal node version
    //             if let Some((handshake, discovered_local_address, version_message)) = environment
    //                 .receive_handler()
    //                 .receive_connection_request(&environment, 1u64, height, remote_address, reader)
    //                 .await
    //             {
    //                 // Bootstrap discovery of local node IP via VERACK responses
    //                 {
    //                     let local_address = peer_manager.local_address();
    //                     if local_address != discovered_local_address {
    //                         peer_manager.set_local_address(discovered_local_address).await;
    //                         info!("Discovered local address: {:?}", local_address);
    //                     }
    //                 }
    //                 // Store the channel established with the handshake
    //                 peer_manager.add_channel(&handshake.channel);
    //
    //                 if let Some(version) = version_message {
    //                     // If our peer has a longer chain, send a sync message
    //                     if version.height > environment.current_block_height().await {
    //                         // Update the sync node if the sync_handler is Idle
    //                         if let Ok(mut sync_handler) = sync_manager.try_lock() {
    //                             if !sync_handler.is_syncing() {
    //                                 sync_handler.sync_node_address = handshake.channel.remote_address;
    //
    //                                 if let Ok(block_locator_hashes) =
    //                                     environment.storage_read().await.get_block_locator_hashes()
    //                                 {
    //                                     if let Err(err) =
    //                                         handshake.channel.write(&GetSync::new(block_locator_hashes)).await
    //                                     {
    //                                         error!(
    //                                             "Error sending GetSync message to {}, {}",
    //                                             handshake.channel.remote_address, err
    //                                         );
    //                                     }
    //                                 }
    //                             }
    //                         }
    //                     }
    //                 }
    //
    //                 // Inner loop spawns one thread per connection to read messages
    //                 Self::spawn_connection_thread(handshake.channel.clone(), sender.clone());
    //             }
    //         }
    //     });
    // }

    // /// This method handles all messages sent from connected peers.
    // ///
    // /// Messages are received by a single tokio MPSC receiver with
    // /// the message name, bytes, associated channel, and a tokio oneshot sender.
    // ///
    // /// The oneshot sender lets the connection thread know when the message is handled.
    // #[inline]
    // pub async fn message_handler(
    //     &self,
    //     environment: &Environment,
    //     receiver: &mut Receiver,
    // ) -> Result<(), NetworkError> {
    //     // TODO (raychu86) Create a macro to the handle the error messages.
    //     // TODO (howardwu): Come back and add error handlers to these.
    //     while let Some((tx, name, bytes, mut channel)) = receiver.recv().await {
    //         // if name == Block::name() {
    //         //     if let Ok(block) = Block::deserialize(bytes) {
    //         //         if let Err(err) = self
    //         //             .receive_block_message(environment, block, channel.clone(), true)
    //         //             .await
    //         //         {
    //         //             error!(
    //         //                 "Receive handler errored when receiving a {} message from {}. {}",
    //         //                 name, channel.remote_address, err
    //         //             );
    //         //         }
    //         //     }
    //         // } else if name == GetBlock::name() {
    //         //     if let Ok(getblock) = GetBlock::deserialize(bytes) {
    //         //         if let Err(err) = self.receive_get_block(environment, getblock, channel.clone()).await {
    //         //             error!(
    //         //                 "Receive handler errored when receiving a {} message from {}. {}",
    //         //                 name, channel.remote_address, err
    //         //             );
    //         //         }
    //         //     }
    //         // } else if name == GetMemoryPool::name() {
    //         //     if let Ok(getmemorypool) = GetMemoryPool::deserialize(bytes) {
    //         //         if let Err(err) = self
    //         //             .receive_get_memory_pool(environment, getmemorypool, channel.clone())
    //         //             .await
    //         //         {
    //         //             error!(
    //         //                 "Receive handler errored when receiving a {} message from {}. {}",
    //         //                 name, channel.remote_address, err
    //         //             );
    //         //         }
    //         //     }
    //         // } else if name == GetPeers::name() {
    //         //     if let Ok(getpeers) = GetPeers::deserialize(bytes) {
    //         //         if let Err(err) = self.receive_get_peers(environment, getpeers, channel.clone()).await {
    //         //             error!(
    //         //                 "Receive handler errored when receiving a {} message from {}. {}",
    //         //                 name, channel.remote_address, err
    //         //             );
    //         //         }
    //         //     }
    //         // } else if name == GetSync::name() {
    //         //     if let Ok(getsync) = GetSync::deserialize(bytes) {
    //         //         if let Err(err) = self.receive_get_sync(environment, getsync, channel.clone()).await {
    //         //             error!(
    //         //                 "Receive handler errored when receiving a {} message from {}. {}",
    //         //                 name, channel.remote_address, err
    //         //             );
    //         //         }
    //         //     }
    //         // } else if name == MemoryPool::name() {
    //         //     if let Ok(mempool) = MemoryPool::deserialize(bytes) {
    //         //         if let Err(err) = self.receive_memory_pool(environment, mempool).await {
    //         //             error!(
    //         //                 "Receive handler errored when receiving a {} message from {}. {}",
    //         //                 name, channel.remote_address, err
    //         //             );
    //         //         }
    //         //     }
    //         // } else if name == Peers::name() {
    //         //     if let Ok(peers) = Peers::deserialize(bytes) {
    //         //         if let Err(err) = self.receive_peers(environment, peers, channel.clone()).await {
    //         //             error!(
    //         //                 "Receive handler errored when receiving a {} message from {}. {}",
    //         //                 name, channel.remote_address, err
    //         //             );
    //         //         }
    //         //     }
    //         // } else if name == Ping::name() {
    //         //     if let Ok(ping) = Ping::deserialize(bytes) {
    //         //         if let Err(err) = self.receive_ping(environment, ping, channel.clone()).await {
    //         //             error!(
    //         //                 "Receive handler errored when receiving a {} message from {}. {}",
    //         //                 name, channel.remote_address, err
    //         //             );
    //         //         }
    //         //     }
    //         // } else if name == Pong::name() {
    //         //     if let Ok(pong) = Pong::deserialize(bytes) {
    //         //         if let Err(err) = self.receive_pong(environment, pong, channel.clone()).await {
    //         //             error!(
    //         //                 "Receive handler errored when receiving a {} message from {}. {}",
    //         //                 name, channel.remote_address, err
    //         //             );
    //         //         }
    //         //     }
    //         // } else if name == Sync::name() {
    //         //     if let Ok(sync) = Sync::deserialize(bytes) {
    //         //         if let Err(err) = self.receive_sync(environment, sync).await {
    //         //             error!(
    //         //                 "Receive handler errored when receiving a {} message from {}. {}",
    //         //                 name, channel.remote_address, err
    //         //             );
    //         //         }
    //         //     }
    //         // } else if name == SyncBlock::name() {
    //         //     if let Ok(block) = Block::deserialize(bytes) {
    //         //         if let Err(err) = self
    //         //             .receive_block_message(environment, block, channel.clone(), false)
    //         //             .await
    //         //         {
    //         //             error!(
    //         //                 "Receive handler errored when receiving a {} message from {}. {}",
    //         //                 name, channel.remote_address, err
    //         //             );
    //         //         }
    //         //     }
    //         // } else if name == Transaction::name() {
    //         //     if let Ok(transaction) = Transaction::deserialize(bytes) {
    //         //         if let Err(err) = self
    //         //             .receive_transaction(environment, transaction, channel.clone())
    //         //             .await
    //         //         {
    //         //             error!(
    //         //                 "Receive handler errored when receiving a {} message from {}. {}",
    //         //                 name, channel.remote_address, err
    //         //             );
    //         //         }
    //         //     }
    //         // } else if name == Version::name() {
    //         //     if let Ok(version) = Version::deserialize(bytes) {
    //         //         // TODO (raychu86) Does `receive_version` need to return a channel?
    //         //         match self.receive_version(environment, version, channel.clone()).await {
    //         //             Ok(returned_channel) => channel = returned_channel,
    //         //             Err(err) => error!(
    //         //                 "Receive handler errored when receiving a {} message from {}. {}",
    //         //                 name, channel.remote_address, err
    //         //             ),
    //         //         }
    //         //     }
    //         // } else if name == Verack::name() {
    //         //     if let Ok(verack) = Verack::deserialize(bytes) {
    //         //         if !self.receive_verack(environment, verack, channel.clone()).await {
    //         //             error!(
    //         //                 "Receive handler errored when receiving a {} message from {}",
    //         //                 name, channel.remote_address
    //         //             );
    //         //         }
    //         //     }
    //         // } else if name == MessageName::from("disconnect") {
    //         //     info!("Disconnected from peer {:?}", channel.remote_address);
    //         //     {
    //         //         let mut peer_manager = environment.peer_manager_write().await;
    //         //         peer_manager.disconnect_from_peer(&channel.remote_address).await?;
    //         //     }
    //         // } else {
    //         //     debug!("Message name not recognized {:?}", name.to_string());
    //         // }
    //         //
    //         // if let Err(error) = tx.send(channel) {
    //         //     warn!("Error resetting connection thread ({:?})", error);
    //         // }
    //     }
    //
    //     Ok(())
    // }
}
