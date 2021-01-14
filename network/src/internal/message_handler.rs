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
    errors::ServerError,
    external::{
        message::{Message, MessageName},
        message_types::*,
        propagate_block,
        protocol::SyncState,
        Channel,
        Pings,
    },
    internal::process_transaction_internal,
    Server,
};
use snarkos_consensus::memory_pool::Entry;
use snarkvm_dpc::base_dpc::instantiated::Tx;
use snarkvm_objects::Block as BlockStruct;
use snarkvm_utilities::{
    bytes::{FromBytes, ToBytes},
    to_bytes,
};

use chrono::Utc;
use std::{net::SocketAddr, sync::Arc};

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
            } else if name == MemoryPool::name() {
                if let Ok(mempool) = MemoryPool::deserialize(bytes) {
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
                    if let Err(err) = self.receive_verack(verack, channel.clone()).await {
                        error!(
                            "Message handler errored when receiving a {} message from {}. {}",
                            name, channel.address, err
                        );
                    }
                }
            } else if name == MessageName::from("disconnect") {
                info!("Disconnected from peer {:?}", channel.address);
                {
                    let mut peer_book = self.context.peer_book.write().await;
                    peer_book.disconnect_peer(channel.address);
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
                            if let Some(channel) = self.context.connections.read().await.get(&sync_handler.sync_node) {
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

        for entry in memory_pool.transactions.values() {
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
    async fn receive_memory_pool(&mut self, message: MemoryPool) -> Result<(), ServerError> {
        let mut memory_pool = self.memory_pool_lock.lock().await;

        for transaction_bytes in message.transactions {
            let transaction: Tx = Tx::read(&transaction_bytes[..])?;
            let entry = Entry::<Tx> {
                size: transaction_bytes.len(),
                transaction,
            };

            if let Ok(Some(txid)) = memory_pool.insert(&self.storage, entry) {
                debug!("Transaction added to memory pool with txid: {:?}", hex::encode(&txid));
            }
        }

        Ok(())
    }

    /// A node has requested our list of peer addresses.
    /// Send an Address message with our current peer list.
    async fn receive_get_peers(&mut self, _message: GetPeers, channel: Arc<Channel>) -> Result<(), ServerError> {
        let mut connected_peers = self.context.peer_book.read().await.get_connected();

        // Remove the requester from list of peers
        connected_peers.remove(&channel.address);

        channel.write(&Peers::new(connected_peers)).await?;

        Ok(())
    }

    /// A miner has sent their list of peer addresses.
    /// Add all new/updated addresses to our gossiped.
    /// The connection handler will be responsible for sending out handshake requests to them.
    async fn receive_peers(&mut self, message: Peers, channel: Arc<Channel>) -> Result<(), ServerError> {
        let peer_book = &mut self.context.peer_book.write().await;
        for (addr, time) in message.addresses.iter() {
            if &*self.context.local_address.read().await == addr {
                continue;
            } else if peer_book.connected_contains(addr) {
                peer_book.update_connected(*addr, *time);
            } else {
                peer_book.update_gossiped(*addr, *time);
            }
        }

        peer_book.update_connected(channel.address, Utc::now());

        Ok(())
    }

    /// A peer has sent us a ping message.
    /// Reply with a pong message.
    async fn receive_ping(&mut self, message: Ping, channel: Arc<Channel>) -> Result<(), ServerError> {
        let mut peer_book = self.context.peer_book.write().await;

        if peer_book.connected_contains(&channel.address) {
            peer_book.update_connected(channel.address, Utc::now());
        }

        Pings::send_pong(message, channel).await?;

        Ok(())
    }

    /// A peer has sent us a pong message.
    /// See if it matches a ping we sent out.
    async fn receive_pong(&mut self, message: Pong, channel: Arc<Channel>) -> Result<(), ServerError> {
        match self
            .context
            .pings
            .write()
            .await
            .accept_pong(channel.address, message)
            .await
        {
            Ok(()) => {
                self.context
                    .peer_book
                    .write()
                    .await
                    .update_connected(channel.address, Utc::now());
            }
            Err(error) => debug!(
                "Invalid Pong message from: {:?}, Full error: {:?}",
                channel.address, error
            ),
        }

        Ok(())
    }

    /// A peer has requested our chain state to sync with.
    async fn receive_get_sync(&mut self, message: GetSync, channel: Arc<Channel>) -> Result<(), ServerError> {
        let latest_shared_hash = self.storage.get_latest_shared_hash(message.block_locator_hashes)?;
        let current_height = self.storage.get_latest_block_height();

        if let Ok(height) = self.storage.get_block_number(&latest_shared_hash) {
            if height < current_height {
                let mut max_height = current_height;

                // if the requester is behind more than 4000 blocks
                if height + 4000 < current_height {
                    // send the max 4000 blocks
                    max_height = height + 4000;
                }

                let mut block_hashes = Vec::with_capacity(max_height.saturating_sub(height + 1) as usize);

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
        let height = self.storage.get_latest_block_height();
        let mut sync_handler = self.sync_handler_lock.lock().await;

        sync_handler.receive_hashes(message.block_hashes, height);

        // Received block headers
        if let Some(channel) = self.context.connections.read().await.get(&sync_handler.sync_node) {
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
    async fn receive_verack(&mut self, message: Verack, channel: Arc<Channel>) -> Result<(), ServerError> {
        match self
            .context
            .handshakes
            .write()
            .await
            .accept_response(channel.address, message)
            .await
        {
            Ok(()) => {
                // Add connected peer.
                self.context
                    .peer_book
                    .write()
                    .await
                    .update_connected(channel.address, Utc::now());

                // Ask connected peer for more peers.
                channel.write(&GetPeers).await?;
            }
            Err(error) => {
                let error = ServerError::HandshakeError(error);
                debug!(
                    "Invalid Verack message from: {:?} Full error: {:?}",
                    channel.address, &error
                );
            }
        }
        Ok(())
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
            if peer_book.connected_total() < self.context.max_peers {
                self.context
                    .handshakes
                    .write()
                    .await
                    .receive_request(message.clone(), peer_address)
                    .await?;
            }

            // If our peer has a longer chain, send a sync message
            if message.height > self.storage.get_latest_block_height() {
                debug!("Received a version message with a greater height {}", message.height);
                // Update the sync node if the sync_handler is Idle and there are no requested block headers
                if let Ok(mut sync_handler) = self.sync_handler_lock.try_lock() {
                    if !sync_handler.is_syncing()
                        && (sync_handler.block_headers.is_empty() && sync_handler.pending_blocks.is_empty())
                    {
                        debug!("Attempting to sync with peer {}", peer_address);
                        sync_handler.sync_node = peer_address;

                        if let Ok(block_locator_hashes) = self.storage.get_block_locator_hashes() {
                            channel.write(&GetSync::new(block_locator_hashes)).await?;
                        }
                    } else if let Some(channel) = self.context.connections.read().await.get(&sync_handler.sync_node) {
                        sync_handler.increment(channel, Arc::clone(&self.storage)).await?;
                    }
                }
            }
        }
        Ok(channel)
    }
}
