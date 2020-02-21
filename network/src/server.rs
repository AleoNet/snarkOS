use std::{net::SocketAddr, sync::Arc, time::Duration};

//use bincode;
use chrono::{Duration as ChronoDuration, Utc};
use tokio::{
    net::TcpListener,
    sync::{mpsc, oneshot, Mutex},
    task,
    time::delay_for,
};

use snarkos_consensus::{
    miner::{Entry, MemoryPool as MemoryPoolStruct},
    ConsensusParameters,
};
use snarkos_errors::network::ServerError;
use snarkos_objects::{Block as BlockStruct, BlockHeaderHash, Transaction as TransactionStruct};
use snarkos_storage::BlockStorage;

use crate::{
    base::{handshake::*, sync::*, Context},
    message::{types::*, Channel, Message, MessageName},
};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ServerStatus {
    /// Listening for a client request or a peer message.
    Listening,
    /// Awaiting a peer message we can interpret as a client request.
    AwaitingResponse(),
    /// A failure has occurred and we are shutting down the server.
    Failed,
}

pub struct Server {
    pub consensus: ConsensusParameters,
    pub context: Arc<Context>,
    pub storage: Arc<BlockStorage>,
    pub memory_pool_lock: Arc<Mutex<MemoryPoolStruct>>,
    pub receiver: mpsc::Receiver<(oneshot::Sender<()>, MessageName, Vec<u8>, Arc<Channel>)>,
    pub sender: mpsc::Sender<(oneshot::Sender<()>, MessageName, Vec<u8>, Arc<Channel>)>,
    pub status: ServerStatus,
    pub sync_handler_lock: Arc<Mutex<SyncHandler>>,
    pub connection_frequency: u64,
}

impl Server {
    pub fn new(
        context: Context,
        consensus: ConsensusParameters,
        storage: Arc<BlockStorage>,
        memory_pool_lock: Arc<Mutex<MemoryPoolStruct>>,
        sync_handler_lock: Arc<Mutex<SyncHandler>>,
        connection_frequency: u64,
    ) -> Self {
        let (sender, receiver) = mpsc::channel(512);
        Server {
            consensus,
            context: Arc::new(context),
            storage,
            memory_pool_lock,
            receiver,
            sender,
            status: ServerStatus::Listening,
            sync_handler_lock,
            connection_frequency,
        }
    }

    pub async fn listen(mut self) -> Result<(), ServerError> {
        let local_addr = self.context.local_addr;
        let sender = self.sender.clone();
        let context = self.context.clone();

        let mut listener = TcpListener::bind(&local_addr).await?;
        info!("listening at: {:?}", local_addr);

        for bootnode in self.context.bootnodes.clone() {
            let bootnode_address = bootnode.parse::<SocketAddr>()?;

            if local_addr != bootnode_address && !self.context.is_bootnode {
                info!("Connecting to bootnode: {:?}", bootnode_address);

                let connections = &mut self.context.connections.write().await;
                let channel = connections.connect_and_store(bootnode_address).await?;

                Self::spawn_connection_thread(channel.clone(), sender.clone());

                handshake_request(channel.clone(), 1, self.storage.get_latest_block_height(), local_addr).await?;
                channel.write(&GetPeers).await?;
            }
        }

        // Outer loop spawns one thread to accept new connections
        task::spawn(async move {
            loop {
                let (stream, peer_address) = listener.accept().await.unwrap();
                let connections = &mut context.connections.write().await;
                let channel = connections.store(peer_address, Channel::new(stream, peer_address).await.unwrap());

                // Inner loop spawns one thread per connection to read messages
                Self::spawn_connection_thread(channel.clone(), sender.clone());
            }
        });

        self.connection_handler().await;

        self.message_handler().await?;

        Ok(())
    }

    fn spawn_connection_thread(
        channel: Arc<Channel>,
        mut thread_sender: mpsc::Sender<(oneshot::Sender<()>, MessageName, Vec<u8>, Arc<Channel>)>,
    ) {
        // Inner loop spawns one thread per connection to read messages
        task::spawn(async move {
            loop {
                let (tx, rx) = oneshot::channel();
                let (message_name, message_bytes) = channel.read().await.expect("ERROR OK: peer closed connection");
                thread_sender
                    .send((tx, message_name, message_bytes, channel.clone()))
                    .await
                    .expect("could not send to message handler");
                rx.await.expect("message handler errored");
            }
        });
    }

    /// handle an incoming message
    async fn message_handler(&mut self) -> Result<(), ServerError> {
        while let Some((tx, name, bytes, channel)) = self.receiver.recv().await {
            info!("Message {:?}, Received from {:?}", name.to_string(), channel.address);
            if name == Block::name() {
                self.receive_block_message(Block::deserialize(bytes)?, channel, true)
                    .await?;
            } else if name == GetBlock::name() {
                self.receive_get_block(GetBlock::deserialize(bytes)?, channel).await?;
            } else if name == GetMemoryPool::name() {
                self.receive_get_memory_pool(GetMemoryPool::deserialize(bytes)?, channel)
                    .await?;
            } else if name == GetPeers::name() {
                self.receive_get_peers(GetPeers::deserialize(bytes)?, channel).await?;
            } else if name == GetSync::name() {
                self.receive_get_sync(GetSync::deserialize(bytes)?, channel).await?;
            } else if name == MemoryPool::name() {
                self.receive_memory_pool(MemoryPool::deserialize(bytes)?).await?;
            } else if name == Peers::name() {
                self.receive_peers(Peers::deserialize(bytes)?, channel).await?;
            } else if name == Ping::name() {
                self.receive_ping(Ping::deserialize(bytes)?, channel).await?;
            } else if name == Pong::name() {
                self.receive_pong(Pong::deserialize(bytes)?, channel).await?;
            } else if name == Sync::name() {
                self.receive_sync(Sync::deserialize(bytes)?).await?;
            } else if name == SyncBlock::name() {
                self.receive_block_message(Block::deserialize(bytes)?, channel, false)
                    .await?;
            } else if name == Transaction::name() {
                self.receive_transaction(Transaction::deserialize(bytes)?, channel)
                    .await?;
            } else if name == Version::name() {
                self.receive_version(Version::deserialize(bytes)?, channel).await?;
            } else if name == Verack::name() {
                self.receive_verack(Verack::deserialize(bytes)?, channel).await?;
            } else {
                info!("Name not recognized {:?}", name.to_string());
            }
            tx.send(()).expect("error resetting message handler");
        }
        Ok(())
    }

    /// A peer has sent us a new block to process
    async fn receive_block_message(
        &mut self,
        message: Block,
        channel: Arc<Channel>,
        propagate: bool,
    ) -> Result<(), ServerError> {
        let block = BlockStruct::deserialize(&message.data)?;

        if !self.storage.is_exist(&block.header.get_hash()) {
            let mut memory_pool = self.memory_pool_lock.lock().await;
            let inserted = self
                .consensus
                .receive_block(&self.storage, &mut memory_pool, &block)
                .is_ok();
            drop(memory_pool);

            // verify the block and insert it into the storage
            if inserted && propagate {
                propagate_block(self.context.clone(), message.data, channel.address).await?;
            }
        }

        Ok(())
    }

    /// A peer has requested a block
    async fn receive_get_block(&mut self, message: GetBlock, channel: Arc<Channel>) -> Result<(), ServerError> {
        if let Ok(block) = self.storage.get_block(message.block_hash) {
            channel.write(&SyncBlock::new(block.serialize()?)).await?;
        }

        Ok(())
    }

    /// A peer has requested our memory pool transactions
    async fn receive_get_memory_pool(
        &mut self,
        _message: GetMemoryPool,
        channel: Arc<Channel>,
    ) -> Result<(), ServerError> {
        let memory_pool = self.memory_pool_lock.lock().await;

        let mut transactions = vec![];

        for (_tx_id, entry) in &memory_pool.transactions {
            if let Ok(transaction_bytes) = entry.transaction.serialize() {
                transactions.push(transaction_bytes);
            }
        }
        drop(memory_pool);

        if !transactions.is_empty() {
            channel.write(&MemoryPool::new(transactions)).await?;
        }

        Ok(())
    }

    /// A peer has sent us their memory pool transactions
    async fn receive_memory_pool(&mut self, message: MemoryPool) -> Result<(), ServerError> {
        let mut memory_pool = self.memory_pool_lock.lock().await;

        for transaction_bytes in message.transactions {
            let entry = Entry {
                size: transaction_bytes.len(),
                transaction: TransactionStruct::deserialize(&transaction_bytes)?,
            };

            if let Ok(inserted) = memory_pool.insert(&self.storage, entry) {
                if let Some(txid) = inserted {
                    info!("Transaction added to memory pool with txid: {:?}", hex::encode(txid));
                }
            }
        }

        Ok(())
    }

    /// A node has requested our list of peer addresses
    /// send an Address message with our current peer list
    async fn receive_get_peers(&mut self, _message: GetPeers, channel: Arc<Channel>) -> Result<(), ServerError> {
        channel
            .write(&Peers::new(self.context.peer_book.read().await.peers.addresses.clone()))
            .await?;

        Ok(())
    }

    /// A miner has sent their list of peer addresses
    /// send a Version message to each peer in the list
    /// this is going to be a lot of awaits in a loop...
    /// can look at futures crate to handle multiple futures
    /// set server status to listening
    async fn receive_peers(&mut self, message: Peers, channel: Arc<Channel>) -> Result<(), ServerError> {
        let mut peer_book = self.context.peer_book.write().await;
        for (addr, time) in message.addresses.iter() {
            if &self.context.local_addr == addr {
                continue;
            } else if peer_book.peer_contains(addr) {
                peer_book.peers.update(addr.clone(), time.clone());
            } else if peer_book.disconnected_contains(addr) {
                peer_book.disconnected.remove(addr);
                peer_book.gossiped.update(addr.clone(), time.clone());
            } else {
                peer_book.gossiped.update(addr.clone(), time.clone());
            }
        }

        peer_book.peers.update(channel.address, Utc::now());

        Ok(())
    }

    async fn receive_ping(&mut self, message: Ping, channel: Arc<Channel>) -> Result<(), ServerError> {
        channel.write(&Pong::new(message)).await?;

        Ok(())
    }

    async fn receive_pong(&mut self, _message: Pong, channel: Arc<Channel>) -> Result<(), ServerError> {
        self.context
            .peer_book
            .write()
            .await
            .peers
            .update(channel.address, Utc::now());

        Ok(())
    }

    /// A peer has requested our chain state to sync with
    async fn receive_get_sync(&mut self, message: GetSync, channel: Arc<Channel>) -> Result<(), ServerError> {
        let latest_shared_hash = self.storage.get_latest_shared_hash(message.block_locator_hashes)?;
        let current_height = self.storage.get_latest_block_height();

        if let Ok(height) = self.storage.get_block_num(&latest_shared_hash) {
            if height < current_height {
                let mut max_height = current_height;

                // if the requester is behind more than 100 blocks
                if height + 100 < current_height {
                    // send the max 100 blocks
                    max_height = height + 100;
                }

                let mut block_hashes: Vec<BlockHeaderHash> = vec![];

                for block_num in height + 1..=max_height {
                    block_hashes.push(self.storage.get_block_hash(block_num)?);
                }

                // send serialized blocks to requester
                channel.write(&Sync::new(block_hashes)).await?;
            }
        }
        //        }
        Ok(())
    }

    /// A peer has sent us their chain state
    async fn receive_sync(&mut self, message: Sync) -> Result<(), ServerError> {
        let mut sync_handler = self.sync_handler_lock.lock().await;

        for block_hash in message.block_hashes {
            if !sync_handler.block_headers.contains(&block_hash) {
                sync_handler.block_headers.push(block_hash.clone());
            }
            sync_handler.update_syncing(self.storage.get_latest_block_height());
        }

        if let Some(channel) = self.context.connections.read().await.get(&sync_handler.sync_node) {
            drop(sync_handler); //Todo: look at again
            increment_sync_handler(channel, Arc::clone(&self.sync_handler_lock), Arc::clone(&self.storage)).await?;
        }

        Ok(())
    }

    /// A peer has sent us a transaction
    async fn receive_transaction(&mut self, message: Transaction, channel: Arc<Channel>) -> Result<(), ServerError> {
        process_transaction_internal(
            self.context.clone(),
            self.storage.clone(),
            self.memory_pool_lock.clone(),
            message.bytes,
            channel.address,
        )
        .await?;

        Ok(())
    }

    /// A miner has acknowledged our Version message
    /// add them to our peer book
    async fn receive_verack(&mut self, _message: Verack, channel: Arc<Channel>) -> Result<(), ServerError> {
        let peer_book = &mut self.context.peer_book.write().await;

        if &self.context.local_addr != &channel.address {
            peer_book.disconnected.remove(&channel.address);
            peer_book.gossiped.remove(&channel.address);
            peer_book.peers.update(channel.address, Utc::now());
        }

        Ok(())
    }

    /// A miner is trying to connect with us
    /// check if sending node is a new peer
    async fn receive_version(&mut self, message: Version, channel: Arc<Channel>) -> Result<(), ServerError> {
        let peer_address = message.address_sender;
        let peer_book = &mut self.context.peer_book.read().await;

        if peer_book.peers.addresses.len() < self.context.max_peers as usize && self.context.local_addr != peer_address
        {
            let mut connections = self.context.connections.write().await;
            let channel = connections.update(channel.address, peer_address)?;
            drop(connections);

            let new_peer = !peer_book.peers.addresses.contains_key(&peer_address);

            let latest_height = self.storage.get_latest_block_height();

            handshake_response(channel.clone(), new_peer, 1, latest_height, self.context.local_addr).await?;

            // if our peer has a longer chain, send a sync message
            if message.height > latest_height {
                let mut sync_handler = self.sync_handler_lock.lock().await;
                sync_handler.sync_node = peer_address;

                if let Ok(block_locator_hashes) = self.storage.get_block_locator_hashes() {
                    channel.write(&GetSync::new(block_locator_hashes)).await?;
                }
            }
        }
        Ok(())
    }

    /// Manage number of active connections according to the connection frequency
    async fn connection_handler(&self) {
        let context = Arc::clone(&self.context);
        let memory_pool_lock = Arc::clone(&self.memory_pool_lock);
        let sync_handler_lock = Arc::clone(&self.sync_handler_lock);
        let storage = Arc::clone(&self.storage);
        let connection_frequency = self.connection_frequency;

        task::spawn(async move {
            let mut interval_ticker: u8 = 0;

            loop {
                delay_for(Duration::from_millis(connection_frequency)).await;

                let peer_book = &mut context.peer_book.write().await;

                // We have less peers than our minimum peer requirement. Look for more peers
                if peer_book.peers.addresses.len() < context.min_peers as usize {
                    for (socket_addr, _last_seen) in peer_book.peers.addresses.clone() {
                        if let Some(channel) = context.connections.read().await.get(&socket_addr) {
                            if let Err(_) = channel.write(&GetPeers).await {
                                //Todo: impl protocol
                                peer_book.disconnect_peer(&socket_addr);
                            }
                        }
                    }

                    for (socket_addr, _last_seen) in peer_book.gossiped.addresses.clone() {
                        if let Some(channel) = context.connections.read().await.get(&socket_addr) {
                            if socket_addr != context.local_addr {
                                if let Err(_) = handshake_request(
                                    channel,
                                    1u64,
                                    storage.get_latest_block_height(),
                                    context.local_addr,
                                )
                                .await
                                {
                                    peer_book.disconnect_peer(&socket_addr);
                                }
                            }
                        }
                    }
                }

                // Maintain a connection with existing peers and update last seen
                for (socket_addr, _last_seen) in peer_book.peers.addresses.clone() {
                    // Ping peers and update last seen if there is a response
                    if socket_addr != context.local_addr {
                        if let Some(channel) = context.connections.read().await.get(&socket_addr) {
                            if let Err(_) = channel.write(&Ping::new()).await {
                                //Todo: impl ping protocol
                                peer_book.disconnect_peer(&socket_addr);
                            }
                        }
                    }
                }

                // Purge peers that haven't responded in 2 loops

                let response_timeout = ChronoDuration::milliseconds((connection_frequency * 2) as i64);

                for (socket_addr, last_seen) in peer_book.peers.addresses.clone() {
                    if Utc::now() - last_seen.clone() > response_timeout {
                        peer_book.disconnect_peer(&socket_addr);
                    }
                }

                let mut sync_handler = sync_handler_lock.lock().await;
                if peer_book.disconnected_contains(&sync_handler.sync_node) {
                    match peer_book.peers.addresses.iter().max_by(|a, b| a.1.cmp(&b.1)) {
                        Some(peer) => sync_handler.sync_node = peer.0.clone(),
                        None => continue,
                    };
                }

                if interval_ticker >= context.memory_pool_interval {
                    // Also request memory pool and cleanse necessary values
                    let mut memory_pool = memory_pool_lock.lock().await;

                    match (memory_pool.cleanse(&storage), memory_pool.store(&storage)) {
                        (_, _) => {}
                    };

                    if context.local_addr != sync_handler.sync_node {
                        if let Some(channel) = context.connections.read().await.get(&sync_handler.sync_node) {
                            if let Err(_) = channel.write(&GetMemoryPool).await {
                                //Todo: impl memory pool protocol
                                peer_book.disconnect_peer(&sync_handler.sync_node);
                            }
                        }
                    }

                    interval_ticker = 0;
                } else {
                    interval_ticker += 1;
                }

                drop(sync_handler);
            }
        });
    }
}
