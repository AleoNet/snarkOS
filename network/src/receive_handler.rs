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
    external::{message::Message, message_types::*, Channel, MessageName},
    peer_manager::{PeerMessage, PeerSender},
    peers::PeerBook,
    Environment,
    NetworkError,
    Receiver,
    SyncManager,
    SyncState,
};
use snarkos_consensus::memory_pool::Entry;
use snarkos_dpc::{
    instantiated::{Components, Tx},
    PublicParameters,
};
use snarkos_objects::BlockHeaderHash;
use snarkos_utilities::{
    bytes::{FromBytes, ToBytes},
    to_bytes,
};

use std::{
    collections::HashMap,
    fmt::Display,
    net::SocketAddr,
    sync::{atomic::AtomicU64, Arc},
};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::RwLock,
    task,
};

/// The map of remote addresses to their active read channels.
pub type Channels = HashMap<SocketAddr, Arc<Channel>>;

/// A stateless component for handling inbound network traffic.
#[derive(Debug, Clone)]
pub struct ReceiveHandler {
    /// The map of remote addresses to their active read channels.
    channels: Arc<RwLock<Channels>>,
    /// The producer for sending inbound messages to the peer manager.
    peer_sender: Arc<PeerSender>,
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
    pub fn new(peer_sender: PeerSender) -> Self {
        Self {
            channels: Arc::new(RwLock::new(HashMap::new())),
            peer_sender: Arc::new(peer_sender),
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

    // TODO (howardwu): Remove environment from function inputs.
    #[inline]
    pub async fn listen(self, environment: Environment) -> Result<(), NetworkError> {
        // TODO (howardwu): Find the actual address of this node.
        // 1. Initialize TCP listener and accept new TCP connections.
        let local_address = environment.local_address();
        debug!("Starting listener at {:?}...", local_address);
        let listener = TcpListener::bind(&local_address).await?;
        info!("Listening at {:?}", local_address);

        let environment = environment.clone();
        let sender = self.peer_sender.clone();
        let receive_handler = self.clone();

        task::spawn(async move {
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

                // Follow handshake protocol and drop peer connection if unsuccessful.
                let height = environment.current_block_height().await;

                // TODO (raychu86) Establish a formal node version
                if let Some((channel, discovered_local_address)) = self
                    .receive_connection_request(sender.clone(), 1u64, height, remote_address, channel)
                    .await
                    .unwrap()
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
                        tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                    }

                    let mut failure_count = 0u8;
                    let mut disconnect_from_peer = false;

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
                            Ok((message_name, message_bytes)) => {
                                trace!("Received a {} message from channel", message_name);
                                (message_name, message_bytes)
                            }
                            Err(error) => {
                                error!("Failed to read message from channel\n{}", error);
                                handle_failure(&mut failure, &mut failure_count, &mut disconnect_from_peer, error)
                                    .await;
                                // Determine if we should send a disconnect message.
                                match disconnect_from_peer {
                                    true => (MessageName::from("disconnect"), vec![]),
                                    false => continue,
                                }
                            }
                        };

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

                        // match name {
                        //     Block::name() => {
                        //         if let Ok(block) = Block::deserialize(bytes) {
                        //             if let Err(err) = sender
                        //                 .send(PeerMessage::Block(channel.remote_address, block, true))
                        //                 .await
                        //             {
                        //                 error!(
                        //                     "Receive handler errored on a {} message from {}. {}",
                        //                     name, remote_address, err
                        //                 );
                        //             }
                        //         }
                        //     }
                        //     _ => (),
                        // };

                        if name == Block::name() {
                            if let Ok(block) = Block::deserialize(bytes) {
                                if let Err(err) = sender
                                    .send(PeerMessage::Block(channel.remote_address, block, true))
                                    .await
                                {
                                    error!(
                                        "Receive handler errored on a {} message from {}. {}",
                                        name, remote_address, err
                                    );
                                }
                            }
                        } else if name == SyncBlock::name() {
                            if let Ok(block) = Block::deserialize(bytes) {
                                if let Err(err) = sender
                                    .send(PeerMessage::Block(channel.remote_address, block, false))
                                    .await
                                {
                                    error!(
                                        "Receive handler errored on a {} message from {}. {}",
                                        name, remote_address, err
                                    );
                                }
                            }
                        } else if name == GetBlock::name() {
                            if let Ok(getblock) = GetBlock::deserialize(bytes) {
                                if let Err(err) = receive_handler
                                    .clone()
                                    .receive_get_block(environment, getblock, channel.clone())
                                    .await
                                {
                                    error!(
                                        "Receive handler errored on a {} message from {}. {}",
                                        name, remote_address, err
                                    );
                                }
                            }
                        } else if name == GetMemoryPool::name() {
                            if let Ok(getmemorypool) = GetMemoryPool::deserialize(bytes) {
                                if let Err(err) = receive_handler
                                    .clone()
                                    .receive_get_memory_pool(environment, getmemorypool, channel.clone())
                                    .await
                                {
                                    error!(
                                        "Receive handler errored on a {} message from {}. {}",
                                        name, remote_address, err
                                    );
                                }
                            }
                        } else if name == GetPeers::name() {
                            if let Ok(_) = GetPeers::deserialize(bytes) {
                                if let Err(err) = sender.send(PeerMessage::GetPeers(channel.remote_address)).await {
                                    error!(
                                        "Receive handler errored on a {} message from {}. {}",
                                        name, remote_address, err
                                    );
                                }
                            }
                        } else if name == Peers::name() {
                            if let Ok(peers) = Peers::deserialize(bytes) {
                                if let Err(err) = sender.send(PeerMessage::Peers(channel.remote_address, peers)).await {
                                    error!(
                                        "Receive handler errored on a {} message from {}. {}",
                                        name, remote_address, err
                                    );
                                }
                            }
                        } else if name == GetSync::name() {
                            if let Ok(getsync) = GetSync::deserialize(bytes) {
                                if let Err(err) = receive_handler
                                    .clone()
                                    .receive_get_sync(environment, getsync, channel.clone())
                                    .await
                                {
                                    error!(
                                        "Receive handler errored on a {} message from {}. {}",
                                        name, remote_address, err
                                    );
                                }
                            }
                        } else if name == MemoryPool::name() {
                            if let Ok(mempool) = MemoryPool::deserialize(bytes) {
                                if let Err(err) =
                                    receive_handler.clone().receive_memory_pool(environment, mempool).await
                                {
                                    error!(
                                        "Receive handler errored on a {} message from {}. {}",
                                        name, remote_address, err
                                    );
                                }
                            }
                        } else if name == Sync::name() {
                            if let Ok(sync) = Sync::deserialize(bytes) {
                                if let Err(err) = receive_handler.clone().receive_sync(environment, sync).await {
                                    error!(
                                        "Receive handler errored on a {} message from {}. {}",
                                        name, remote_address, err
                                    );
                                }
                            }
                        } else if name == Transaction::name() {
                            if let Ok(transaction) = Transaction::deserialize(bytes) {
                                if let Err(err) = sender
                                    .send(PeerMessage::Transaction(channel.remote_address, transaction))
                                    .await
                                {
                                    error!(
                                        "Receive handler errored on a {} message from {}. {}",
                                        name, remote_address, err
                                    );
                                }
                            }
                        } else if name == Version::name() {
                            if let Ok(version) = Version::deserialize(bytes) {
                                // TODO (raychu86) Does `receive_version` need to return a channel?
                                match receive_handler
                                    .clone()
                                    .receive_version(environment, version, channel.clone())
                                    .await
                                {
                                    Ok(returned_channel) => channel = returned_channel,
                                    Err(err) => error!(
                                        "Receive handler errored on a {} message from {}. {}",
                                        name, remote_address, err
                                    ),
                                }
                            }
                        } else if name == Verack::name() {
                            if let Ok(verack) = Verack::deserialize(bytes) {
                                if let Err(err) = sender
                                    .send(PeerMessage::ConnectedTo(channel.remote_address, verack.nonce))
                                    .await
                                {
                                    error!(
                                        "Receive handler errored on a {} message from {}. {}",
                                        name, remote_address, err
                                    );
                                }
                            }
                        } else if name == MessageName::from("disconnect") {
                            info!("Disconnected from peer {:?}", remote_address);
                            if let Err(err) = sender.send(PeerMessage::DisconnectFrom(remote_address)).await {
                                error!(
                                    "Receive handler errored on a {} message from {}. {}",
                                    name, remote_address, err
                                );
                            }
                        } else {
                            debug!("Message name not recognized {:?}", name.to_string());
                        }
                    }

                    // TODO (howardwu): Remove this and rearchitect how disconnects are handled using the peer manager.
                    // TODO (howardwu): Implement a handler so the node does not lose state of undetected disconnects.
                    // Break out of the loop if the peer disconnects.
                    if disconnect_from_peer {
                        warn!("Disconnecting from an unreliable peer");
                        break;
                    }
                }

                warn!("RECEIVE HANDLER: END LISTEN");
            }
        });

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

    /// A connected peer has sent handshake request.
    /// Update peer's channel.
    /// If peer's block height is greater than ours, send a sync request.
    ///
    /// This method may seem redundant to handshake protocol functions but a peer can send additional
    /// Version messages if they want to update their ip address/port or want to share their chain height.
    async fn receive_version(
        &self,
        environment: &Environment,
        version: Version,
        channel: Arc<Channel>,
    ) -> Result<Arc<Channel>, NetworkError> {
        let remote_address = SocketAddr::new(channel.remote_address.ip(), version.sender.port());

        let sender = self.peer_sender.clone();

        if *environment.local_address() != remote_address {
            // Route version message to peer manager.
            warn!("RECEIVEVERSIONCOMPARE {} {}", channel.remote_address, remote_address);
            sender
                .send(PeerMessage::VersionToVerack(remote_address, version.clone()))
                .await?;

            // TODO (howardwu): Implement this.
            {
                // // If our peer has a longer chain, send a sync message
                // if version.height > environment.storage_read().await.get_current_block_height() {
                //     debug!("Received a version message with a greater height {}", version.height);
                //     // Update the sync node if the sync_handler is idle and there are no requested block headers
                //     if let Ok(mut sync_handler) = environment.sync_manager().await.try_lock() {
                //         if !sync_handler.is_syncing()
                //             && (sync_handler.block_headers.len() == 0 && sync_handler.pending_blocks.is_empty())
                //         {
                //             debug!("Attempting to sync with peer {}", remote_address);
                //             sync_handler.sync_node_address = remote_address;
                //
                //             if let Ok(block_locator_hashes) = environment.storage_read().await.get_block_locator_hashes() {
                //                 channel.write(&GetSync::new(block_locator_hashes)).await?;
                //             }
                //         } else {
                //             // TODO (howardwu): Implement this.
                //             {
                //                 // if let Some(channel) = environment
                //                 //     .peer_manager_read()
                //                 //     .await
                //                 //     .get_channel(&sync_handler.sync_node_address)
                //                 // {
                //                 //     sync_handler.increment(channel.clone()).await?;
                //                 // }
                //             }
                //         }
                //     }
                // }
            }
        }
        Ok(channel)
    }

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
        sender: Arc<PeerSender>,
        version: u64,
        block_height: u32,
        remote_address: SocketAddr,
        reader: TcpStream,
    ) -> Result<Option<(Arc<Channel>, SocketAddr)>, NetworkError> {
        trace!("Received connection request from {}", remote_address);

        // Parse the inbound message into the message name and message bytes.
        let (channel, (message_name, message_bytes)) = match Channel::new_reader(reader) {
            // Read the next message from the channel.
            // Note this is a blocking operation.
            Ok(channel) => match channel.read().await {
                Ok(inbound_message) => (channel, inbound_message),
                _ => return Ok(None),
            },
            _ => return Ok(None),
        };

        trace!("Received a {} message", message_name);

        // Handles a version message request.
        // Create and store a new handshake in the manager.
        if message_name == Version::name() {
            warn!("IN VERSION CASE");

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

            // Connect to the remote address.
            let remote_address = local_version.receiver;
            let channel = channel.update_writer(remote_address).await?;
            // Write a verack response to the remote peer.
            let local_address = local_version.sender;
            warn!("RECEIVEHANDLERNUMBER {}", channel.remote_address);
            channel
                .write(&Verack::new(remote_version.nonce, local_address, remote_address))
                .await?;
            // Write version request to the remote peer.
            channel.write(&local_version).await?;
            sender
                .send(PeerMessage::ConnectingTo(local_version.receiver, local_version.nonce))
                .await?;

            trace!("Received handshake from {}", remote_address);

            {
                // Acquire the channels write lock.
                let mut channels = self.channels.write().await;
                // Store the new channel.
                let channel = Arc::new(channel.clone());
                channels.insert(local_address, channel.clone());
            }

            {
                // Parse the inbound message into the message name and message bytes.
                let (channel, (message_name, message_bytes)) = match channel.read().await {
                    Ok(inbound_message) => (channel, inbound_message),
                    _ => return Ok(None),
                };

                trace!("Received a {} message", message_name);

                warn!("IN VERACK CASE {}", channel.remote_address);

                // Deserialize the message bytes into a verack message.
                let verack = match Verack::deserialize(message_bytes) {
                    Ok(verack) => verack,
                    _ => return Ok(None),
                };

                let local_address = verack.receiver;

                // TODO (howardwu): Check whether this remote address needs to
                //   be derive the same way as the version message case above
                //  (using a remote_address.ip() and address_sender.port()).
                let remote_address = verack.sender;

                // Acquire the channels write lock.
                let mut channels = self.channels.write().await;
                // Store the new channel.
                let channel = Arc::new(channel);
                channels.insert(remote_address, channel.clone());

                sender
                    .send(PeerMessage::ConnectedTo(remote_address, verack.nonce))
                    .await?;

                trace!("Established connection with {}", remote_address);

                return Ok(Some((channel, local_address)));
            }
        }

        // Handles a verack message request.
        // Establish the channel with the remote peer.
        if message_name == Verack::name() {
            warn!("IN VERACK CASE {}", channel.remote_address);

            // Deserialize the message bytes into a verack message.
            let verack = match Verack::deserialize(message_bytes) {
                Ok(verack) => verack,
                _ => return Ok(None),
            };

            let local_address = verack.receiver;

            // TODO (howardwu): Check whether this remote address needs to
            //   be derive the same way as the version message case above
            //  (using a remote_address.ip() and address_sender.port()).
            let remote_address = verack.sender;

            // Acquire the channels write lock.
            let mut channels = self.channels.write().await;
            // Store the new channel.
            let channel = Arc::new(channel);
            channels.insert(remote_address, channel.clone());

            sender
                .send(PeerMessage::ConnectedTo(remote_address, verack.nonce))
                .await?;

            trace!("Established connection with {}", remote_address);

            return Ok(Some((channel, local_address)));
        }

        Ok(None)
    }
}
