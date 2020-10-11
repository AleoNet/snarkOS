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
    external::{message::Message, message_types::*, Channel, Handshake, MessageName, PingPongManager},
    Environment, NetworkError, Receiver, SyncManager, SyncState,
};
use snarkos_consensus::memory_pool::Entry;
use snarkos_dpc::{
    instantiated::{Components, Tx},
    PublicParameters,
};
use snarkos_errors::network::ServerError;
use snarkos_objects::{Block as BlockStruct, BlockHeaderHash};
use snarkos_utilities::{
    bytes::{FromBytes, ToBytes},
    to_bytes,
};

use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};
use tokio::{
    net::TcpStream,
    sync::{mpsc, oneshot, Mutex, RwLock},
};

/// A stateless component for handling inbound network traffic.
#[derive(Debug, Clone)]
pub struct ReceiveHandler {
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
            receive_response_count: Arc::new(AtomicU64::new(0)),
            receive_success_count: Arc::new(AtomicU64::new(0)),
            receive_failure_count: Arc::new(AtomicU64::new(0)),
        }
    }

    /// This method handles all messages sent from connected peers.
    ///
    /// Messages are received by a single tokio MPSC receiver with
    /// the message name, bytes, associated channel, and a tokio oneshot sender.
    ///
    /// The oneshot sender lets the connection thread know when the message is handled.
    #[inline]
    pub async fn message_handler(&self, environment: &Environment, receiver: &mut Receiver) {
        // TODO (raychu86) Create a macro to the handle the error messages.
        // TODO (howardwu): Come back and add error handlers to these.
        while let Some((tx, name, bytes, mut channel)) = receiver.recv().await {
            if name == Block::name() {
                if let Ok(block) = Block::deserialize(bytes) {
                    if let Err(err) = self
                        .receive_block_message(environment, block, channel.clone(), true)
                        .await
                    {
                        error!(
                            "Receive handler errored when receiving a {} message from {}. {}",
                            name, channel.address, err
                        );
                    }
                }
            } else if name == GetBlock::name() {
                if let Ok(getblock) = GetBlock::deserialize(bytes) {
                    if let Err(err) = self.receive_get_block(environment, getblock, channel.clone()).await {
                        error!(
                            "Receive handler errored when receiving a {} message from {}. {}",
                            name, channel.address, err
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
                            name, channel.address, err
                        );
                    }
                }
            } else if name == GetPeers::name() {
                if let Ok(getpeers) = GetPeers::deserialize(bytes) {
                    if let Err(err) = self.receive_get_peers(environment, getpeers, channel.clone()).await {
                        error!(
                            "Receive handler errored when receiving a {} message from {}. {}",
                            name, channel.address, err
                        );
                    }
                }
            } else if name == GetSync::name() {
                if let Ok(getsync) = GetSync::deserialize(bytes) {
                    if let Err(err) = self.receive_get_sync(environment, getsync, channel.clone()).await {
                        error!(
                            "Receive handler errored when receiving a {} message from {}. {}",
                            name, channel.address, err
                        );
                    }
                }
            } else if name == MemoryPool::name() {
                if let Ok(mempool) = MemoryPool::deserialize(bytes) {
                    if let Err(err) = self.receive_memory_pool(environment, mempool).await {
                        error!(
                            "Receive handler errored when receiving a {} message from {}. {}",
                            name, channel.address, err
                        );
                    }
                }
            } else if name == Peers::name() {
                if let Ok(peers) = Peers::deserialize(bytes) {
                    if let Err(err) = self.receive_peers(environment, peers, channel.clone()).await {
                        error!(
                            "Receive handler errored when receiving a {} message from {}. {}",
                            name, channel.address, err
                        );
                    }
                }
            } else if name == Ping::name() {
                if let Ok(ping) = Ping::deserialize(bytes) {
                    if let Err(err) = self.receive_ping(environment, ping, channel.clone()).await {
                        error!(
                            "Receive handler errored when receiving a {} message from {}. {}",
                            name, channel.address, err
                        );
                    }
                }
            } else if name == Pong::name() {
                if let Ok(pong) = Pong::deserialize(bytes) {
                    if let Err(err) = self.receive_pong(environment, pong, channel.clone()).await {
                        error!(
                            "Receive handler errored when receiving a {} message from {}. {}",
                            name, channel.address, err
                        );
                    }
                }
            } else if name == Sync::name() {
                if let Ok(sync) = Sync::deserialize(bytes) {
                    if let Err(err) = self.receive_sync(environment, sync).await {
                        error!(
                            "Receive handler errored when receiving a {} message from {}. {}",
                            name, channel.address, err
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
                            name, channel.address, err
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
                            name, channel.address, err
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
                            name, channel.address, err
                        ),
                    }
                }
            } else if name == Verack::name() {
                if let Ok(verack) = Verack::deserialize(bytes) {
                    if !self.receive_verack(environment, verack, channel.clone()).await {
                        error!(
                            "Receive handler errored when receiving a {} message from {}",
                            name, channel.address
                        );
                    }
                }
            } else if name == MessageName::from("disconnect") {
                info!("Disconnected from peer {:?}", channel.address);
                {
                    let mut peer_manager = environment.peer_manager_write().await;
                    peer_manager.disconnect_from_peer(&channel.address).await;
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
                        .send_handler()
                        .propagate_block(environment.clone(), message.data, channel.address)
                        .await?;
                } else if !propagate {
                    if let Ok(mut sync_manager) = environment.sync_manager().await.try_lock() {
                        sync_manager.clear_pending().await;

                        if sync_manager.sync_state != SyncState::Idle {
                            // We are currently syncing with a node, ask for the next block.
                            if let Some(channel) = environment
                                .peer_manager_read()
                                .await
                                .get_channel(&sync_manager.sync_node_address)
                                .await
                            {
                                sync_manager.increment(channel.clone()).await?;
                            }
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
        let mut peer_manager = environment.peer_manager_write().await;
        if !peer_manager.is_connected(&channel.address).await {
            peer_manager.found_peer(&channel.address);
        }

        // Broadcast the sanitized list of connected peers back to requesting peer.
        let mut peers = HashMap::new();
        for (remote_address, peer_info) in peer_manager.get_all_connected().await {
            // Skip the iteration if the requesting peer that we're sending the response to
            // appears in the list of peers.
            if remote_address == channel.address {
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
        let mut peer_manager = environment.peer_manager_write().await;
        if !peer_manager.is_connected(&channel.address).await {
            peer_manager.found_peer(&channel.address);
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
            else if !peer_manager.is_connected(&channel.address).await {
                peer_manager.found_peer(&channel.address);
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
        let mut peer_manager = environment.peer_manager_write().await;
        if !peer_manager.is_connected(&channel.address).await {
            peer_manager.found_peer(&channel.address);
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
        let mut peer_manager = environment.peer_manager_write().await;
        if !peer_manager.is_connected(&channel.address).await {
            peer_manager.found_peer(&channel.address);
        }

        if let Err(error) = environment
            .ping_pong()
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

        // Received block headers
        if let Some(channel) = environment
            .peer_manager_read()
            .await
            .get_channel(&sync_handler.sync_node_address)
            .await
        {
            sync_handler.increment(channel.clone()).await?;
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
            .send_handler()
            .process_transaction_internal(
                &environment,
                environment.consensus_parameters(),
                environment.dpc_parameters(),
                environment.storage(),
                environment.memory_pool(),
                message.bytes,
                channel.address,
            )
            .await?;

        Ok(())
    }

    /// A connected peer has acknowledged a handshake request.
    /// Check if the Verack matches the last handshake message we sent.
    /// Update our peer book and send a request for more peers.
    async fn receive_verack(&self, environment: &Environment, message: Verack, channel: Arc<Channel>) -> bool {
        match self.accept_response(environment, channel.address, message).await {
            true => {
                // If we received a verack, but aren't connected to the peer,
                // inform the peer book that we found a peer.
                // The peer book will determine if we have seen the peer before,
                // and include the peer if it is new.
                let mut peer_manager = environment.peer_manager_write().await;
                if !peer_manager.is_connected(&channel.address).await {
                    peer_manager.found_peer(&channel.address);
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
    async fn receive_version(
        &self,
        environment: &Environment,
        message: Version,
        channel: Arc<Channel>,
    ) -> Result<Arc<Channel>, NetworkError> {
        let peer_address = SocketAddr::new(channel.address.ip(), message.address_sender.port());

        let peer_manager = environment.peer_manager_read().await;

        if *environment.local_address() != peer_address {
            if peer_manager.num_connected().await < environment.max_peers() {
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
                        if let Some(channel) = environment
                            .peer_manager_read()
                            .await
                            .get_channel(&sync_handler.sync_node_address)
                            .await
                        {
                            sync_handler.increment(channel.clone()).await?;
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
    #[inline]
    pub async fn receive_connection_request(
        &self,
        environment: &Environment,
        version: u64,
        block_height: u32,
        remote_address: SocketAddr,
        reader: TcpStream,
    ) -> Option<(Handshake, SocketAddr, Option<Version>)> {
        // Read the first message or return `None`.
        let channel = Channel::new_read_only(reader);
        // Parse the inbound message into the message name and message bytes.
        let (channel, (message_name, message_bytes)) = match channel {
            // Read the next message from the channel.
            // Note this is a blocking operation.
            Ok(channel) => match channel.read().await {
                Ok(inbound_message) => (channel, inbound_message),
                _ => return None,
            },
            _ => return None,
        };

        // Handles a version message request.
        // Create and store a new handshake in the manager.
        if message_name == Version::name() {
            // Deserialize the message bytes into a version message.
            let remote_version = match Version::deserialize(message_bytes) {
                Ok(remote_version) => remote_version,
                _ => return None,
            };
            let local_address = remote_version.address_receiver;
            // Create the remote address from the given peer address, and specified port from the version message.
            let remote_address = SocketAddr::new(remote_address.ip(), remote_version.address_sender.port());
            // Create the local version message.
            let local_version = Version::new(version, block_height, remote_address, local_address);
            // Process the new version message and send a response to the remote peer.
            let handshake = match Handshake::receive_new(channel, &local_version, &remote_version).await {
                Ok(handshake) => handshake,
                _ => return None,
            };
            debug!("Received handshake from {:?}", remote_address);
            // Acquire the handshake write lock.
            let mut handshakes = environment.handshakes().write().await;
            // Store the new handshake.
            handshakes.insert(remote_address, handshake.clone());
            // Drop the handshakes write lock.
            drop(handshakes);
            return Some((handshake, local_address, Some(local_version)));
        }

        // Handles a verack message request.
        // Establish the channel with the remote peer.
        if message_name == Verack::name() {
            // Deserialize the message bytes into a verack message.
            let verack = match Verack::deserialize(message_bytes) {
                Ok(verack) => verack,
                _ => return None,
            };
            let local_address = verack.address_receiver;
            // TODO (howardwu): Check whether this remote address needs to
            //   be derive the same way as the version message case above
            //  (using a remote_address.ip() and address_sender.port()).
            let remote_address = verack.address_sender;
            // Acquire the handshake write lock.
            let mut handshakes = environment.handshakes().write().await;
            // Accept the handshake with the remote address.
            let result = match handshakes.get_mut(&remote_address) {
                Some(handshake) => match handshake.accept(verack).await {
                    Ok(()) => {
                        handshake.update_reader(channel);
                        info!("New handshake with {:?}", remote_address);
                        Some((handshake.clone(), local_address, None))
                    }
                    _ => None,
                },
                _ => None,
            };
            // Drop the handshakes write lock.
            drop(handshakes);
            return result;
        }

        None
    }

    // TODO (howardwu): Review this again.
    /// Receives a handshake request from a connected peer.
    /// Updates the handshake channel address, if needed.
    /// Sends a handshake response back to the connected peer.
    pub async fn receive_request(
        &self,
        environment: &Environment,
        message: Version,
        remote_address: SocketAddr,
    ) -> bool {
        match environment.handshakes().write().await.get_mut(&remote_address) {
            Some(handshake) => {
                handshake.update_address(remote_address);
                handshake.receive(message).await.is_ok()
            }
            None => false,
        }
    }

    // TODO (howardwu): Review this again.
    /// Accepts a handshake response from a connected peer.
    pub async fn accept_response(
        &self,
        environment: &Environment,
        remote_address: SocketAddr,
        message: Verack,
    ) -> bool {
        match environment.handshakes().write().await.get_mut(&remote_address) {
            Some(handshake) => {
                debug!("New handshake with {:?}", remote_address);
                handshake.accept(message).await.is_ok()
            }
            None => false,
        }
    }
}
