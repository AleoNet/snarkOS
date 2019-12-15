use crate::{
    base::{
        handshake_request,
        handshake_response,
        send_memory_pool_request,
        send_peers_request,
        send_peers_response,
        send_ping,
        send_pong,
        send_sync_request,
        Context,
        Message,
    },
    sync::*,
};
use snarkos_consensus::{
    miner::{Entry, MemoryPool},
    ConsensusParameters,
};
use snarkos_errors::network::ServerError;
use snarkos_objects::{Block, BlockHeaderHash, Transaction};
use snarkos_storage::BlockStorage;

use crate::base::{
    send_block,
    send_memory_pool_response,
    send_propagate_block,
    send_sync_block,
    send_sync_response,
    send_transaction,
};
use bincode;
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use std::{collections::HashMap, net::SocketAddr, sync::Arc, time::Duration};
use tokio::{
    net::TcpListener,
    prelude::*,
    sync::{mpsc, Mutex},
    task,
    time::delay_for,
};

//pub const ALEO_SERVER_PORT: u16 = 4130;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ServerStatus {
    /// Listening for a client request or a peer message.
    Listening,
    /// Awaiting a peer message we can interpret as a client request.
    AwaitingResponse(),
    /// A failure has occurred and we are shutting down the server.
    Failed,
}

//#[derive(Debug)]
pub struct Server {
    pub consensus: ConsensusParameters,
    pub context: Arc<Context>,
    pub storage: Arc<BlockStorage>,
    pub memory_pool_lock: Arc<Mutex<MemoryPool>>,
    pub receiver: mpsc::Receiver<(Message, SocketAddr)>,
    pub sender: mpsc::Sender<(Message, SocketAddr)>,
    pub status: ServerStatus,
    pub sync_handler_lock: Arc<Mutex<SyncHandler>>,
    pub connection_frequency: u64,
}

impl Server {
    pub fn new(
        context: Context,
        consensus: ConsensusParameters,
        storage: Arc<BlockStorage>,
        memory_pool_lock: Arc<Mutex<MemoryPool>>,
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
        let mut listener = TcpListener::bind(&local_addr).await?;
        info!("listening at: {:?}", local_addr);

        for bootnode in self.context.bootnodes.clone() {
            let bootnode_socket = bootnode.parse::<SocketAddr>()?;

            if local_addr != bootnode_socket && !self.context.is_bootnode {
                handshake_request(self.storage.get_latest_block_height(), bootnode_socket).await?;

                send_peers_request(bootnode_socket).await?;
            }
        }

        let sender = self.sender.clone();

        task::spawn(async move {
            loop {
                let (mut stream, address) = listener.accept().await.unwrap();
                let mut buf = vec![];
                let mut thread_sender = sender.clone();

                task::spawn(async move {
                    loop {
                        let n = match stream.read_to_end(&mut buf).await {
                            // socket closed
                            Ok(n) if n == 0 => return,
                            Ok(n) => n,
                            Err(e) => {
                                error!("failed to read from socket; err = {:?}", e);
                                return;
                            }
                        };

                        let decoded_message: Message = bincode::deserialize(&buf[0..n]).unwrap();

                        // Forward the received message to the message handler
                        thread_sender.send((decoded_message, address)).await.unwrap();

                        return;
                    }
                });
            }
        });

        self.connection_handler().await;

        self.message_handler().await?;

        Ok(())
    }

    /// handle an incoming message
    async fn message_handler(&mut self) -> Result<(), ServerError> {
        while let Some((message, peer_address)) = self.receiver.recv().await {
            match message {
                Message::Block { block_serialized } => {
                    self.handle_block(peer_address, block_serialized, true).await?;
                }
                Message::BlockRequest { block_hash } => {
                    self.handle_block_request(peer_address, block_hash).await?;
                }
                Message::MemoryPoolRequest => {
                    self.handle_memory_pool_request(peer_address).await?;
                }
                Message::MemoryPoolResponse {
                    memory_pool_transactions,
                } => {
                    self.handle_memory_pool_response(memory_pool_transactions).await?;
                }
                Message::PeersRequest => {
                    self.handle_peers_request(peer_address).await?;
                }
                Message::PeersResponse { addresses } => {
                    self.handle_peers_response(peer_address, addresses).await?;
                }
                Message::Ping => {
                    info!("Received Ping from:    {:?}", peer_address);

                    send_pong(peer_address).await?;
                }
                Message::Pong => {
                    info!("Received Pong from:    {:?}", peer_address);

                    let mut peer_book = self.context.peer_book.write().await;
                    peer_book.peers.update(peer_address, Utc::now());
                }
                Message::PropagateBlock { block_serialized } => {
                    self.handle_propagate_block(peer_address, block_serialized).await?;
                }
                Message::Reject => {
                    // A peer has rejected our request
                }
                Message::SyncBlock { block_serialized } => {
                    self.handle_block(peer_address, block_serialized, false).await?;

                    increment_sync_handler(Arc::clone(&self.sync_handler_lock), Arc::clone(&self.storage)).await?;
                }
                Message::SyncRequest { block_locator_hashes } => {
                    self.handle_sync_request(peer_address, block_locator_hashes).await?;
                }
                Message::SyncResponse { block_hashes } => {
                    self.handle_sync_response(block_hashes).await?;
                }

                Message::Transaction { transaction_bytes } => {
                    self.handle_transaction(peer_address, transaction_bytes).await?;
                }

                Message::Verack => {
                    self.handle_verack(peer_address).await?;
                }
                Message::Version {
                    version: _,
                    timestamp: _,
                    height,
                    address_receiver: _,
                } => {
                    self.handle_version(peer_address, height).await?;
                }
            }
        }
        Ok(())
    }

    /// A peer has sent us a new block to process
    async fn handle_block(
        &mut self,
        peer_address: SocketAddr,
        block_serialized: Vec<u8>,
        propagate: bool,
    ) -> Result<(), ServerError> {
        let block = Block::deserialize(&block_serialized)?;

        info!(
            "Received new block!  {:?}\n From peer  {:?}",
            block.header.get_hash(),
            peer_address
        );
        let mut memory_pool = self.memory_pool_lock.lock().await;

        if !self.storage.is_exist(&block.header.get_hash()) {
            // verify the block and insert it into the storage
            if self
                .consensus
                .receive_block(&self.storage, &mut memory_pool, &block)
                .is_ok()
                && propagate
            {
                send_propagate_block(self.context.local_addr, block_serialized).await?;
            }
        }

        Ok(())
    }

    /// A peer has requested a block
    async fn handle_block_request(
        &mut self,
        peer_address: SocketAddr,
        block_hash: BlockHeaderHash,
    ) -> Result<(), ServerError> {
        info!("Received block request from: {:?}", peer_address);

        if let Ok(block) = self.storage.get_block(block_hash) {
            send_sync_block(peer_address, block.serialize().unwrap()).await?;
        }

        Ok(())
    }

    /// A peer has requested our memory pool transactions
    pub async fn handle_memory_pool_request(&mut self, peer_address: SocketAddr) -> Result<(), ServerError> {
        info!("Received memory pool request");

        let memory_pool = self.memory_pool_lock.lock().await;

        let mut transactions = vec![];

        for (_tx_id, entry) in &memory_pool.transactions {
            if let Ok(transaction_bytes) = entry.transaction.serialize() {
                transactions.push(transaction_bytes);
            }
        }

        if !transactions.is_empty() {
            send_memory_pool_response(peer_address, transactions).await?;
        }

        Ok(())
    }

    /// A peer has sent us their memory pool transactions
    async fn handle_memory_pool_response(&mut self, memory_pool_transactions: Vec<Vec<u8>>) -> Result<(), ServerError> {
        info!("Received memory pool response");

        let mut memory_pool = self.memory_pool_lock.lock().await;

        for transaction_bytes in memory_pool_transactions {
            let transaction = Transaction::deserialize(&transaction_bytes)?;
            let entry = Entry {
                size: transaction_bytes.len(),
                transaction,
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
    async fn handle_peers_request(&mut self, peer_address: SocketAddr) -> Result<(), ServerError> {
        info!("Received GetAddresses from: {:?}", peer_address);

        send_peers_response(
            peer_address,
            self.context.peer_book.read().await.peers.addresses.clone(),
        )
        .await?;

        Ok(())
    }

    /// A miner has sent their list of peer addresses
    /// send a Version message to each peer in the list
    /// this is going to be a lot of awaits in a loop...
    /// can look at futures crate to handle multiple futures
    /// set server status to listening
    async fn handle_peers_response(
        &mut self,
        peer_address: SocketAddr,
        addresses: HashMap<SocketAddr, DateTime<Utc>>,
    ) -> Result<(), ServerError> {
        info!("Received Address from:    {:?}", peer_address);

        let mut peer_book = self.context.peer_book.write().await;
        for (addr, time) in addresses.iter() {
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

        peer_book.peers.update(peer_address, Utc::now());

        Ok(())
    }

    /// Announce block to peers
    async fn handle_propagate_block(
        &mut self,
        peer_address: SocketAddr,
        block_serialized: Vec<u8>,
    ) -> Result<(), ServerError> {
        info!("Propagating block to peers");

        for (socket, _) in &self.context.peer_book.read().await.peers.addresses {
            if *socket != peer_address && *socket != self.context.local_addr {
                send_block(*socket, block_serialized.clone()).await?;
            }
        }

        Ok(())
    }

    /// A peer has requested our chain state to sync with
    async fn handle_sync_request(
        &mut self,
        peer_address: SocketAddr,
        block_locator_hashes: Vec<BlockHeaderHash>,
    ) -> Result<(), ServerError> {
        info!("Received sync request from: {:?}", peer_address);

        let latest_shared_hash = self.storage.get_latest_shared_hash(block_locator_hashes)?;
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
                send_sync_response(peer_address, block_hashes).await?;
            }
        }
        //        }
        Ok(())
    }

    /// A peer has sent us their chain state
    async fn handle_sync_response(&mut self, block_hashes: Vec<BlockHeaderHash>) -> Result<(), ServerError> {
        info!("Received sync response");

        let mut sync_handler = self.sync_handler_lock.lock().await;

        for block_hash in block_hashes {
            if !sync_handler.block_headers.contains(&block_hash) {
                sync_handler.block_headers.push(block_hash.clone());
            }
            sync_handler.update_syncing(self.storage.get_latest_block_height());
        }

        drop(sync_handler);

        increment_sync_handler(Arc::clone(&self.sync_handler_lock), Arc::clone(&self.storage)).await?;

        Ok(())
    }

    /// A peer has sent us a transaction
    async fn handle_transaction(
        &mut self,
        peer_address: SocketAddr,
        transaction_bytes: Vec<u8>,
    ) -> Result<(), ServerError> {
        info!("Received Transaction from:    {:?}", peer_address);

        if let Ok(transaction) = Transaction::deserialize(&transaction_bytes) {
            let mut memory_pool = self.memory_pool_lock.lock().await;

            let entry = Entry {
                size: transaction_bytes.len(),
                transaction,
            };

            if let Ok(inserted) = memory_pool.insert(&self.storage, entry) {
                if inserted.is_some() {
                    info!("Transaction added to mempool. Propagating transaction to peers");

                    for (socket, _) in &self.context.peer_book.read().await.peers.addresses {
                        if *socket != peer_address && *socket != self.context.local_addr {
                            send_transaction(*socket, transaction_bytes.clone()).await?;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// A miner has acknowledged our Version message
    /// add them to our peer book
    async fn handle_verack(&mut self, peer_address: SocketAddr) -> Result<(), ServerError> {
        info!("Received Verack from:  {:?}", peer_address);

        let peer_book = &mut self.context.peer_book.write().await;

        if &self.context.local_addr != &peer_address {
            peer_book.disconnected.remove(&peer_address);
            peer_book.gossiped.remove(&peer_address);
            peer_book.peers.update(peer_address, Utc::now());
        }

        Ok(())
    }

    //TODO: add check to verify that this version message is for us
    /// A miner is trying to connect with us
    /// check if sending miner is a new peer
    async fn handle_version(&mut self, peer_address: SocketAddr, height: u32) -> Result<(), ServerError> {
        info!("Received Version from: {:?}", peer_address);

        let peer_book = &mut self.context.peer_book.read().await;

        if peer_book.peers.addresses.len() < self.context.max_peers as usize && self.context.local_addr != peer_address
        {
            let latest_height = self.storage.get_latest_block_height();

            let new_peer = !peer_book.peers.addresses.contains_key(&peer_address);

            handshake_response(latest_height, peer_address, new_peer).await?;

            // if our peer has a longer chain, send a sync message
            if height > latest_height {
                let mut sync_handler = self.sync_handler_lock.lock().await;
                sync_handler.sync_node = peer_address;

                if let Ok(block_locator_hashes) = self.storage.get_block_locator_hashes() {
                    send_sync_request(peer_address, block_locator_hashes).await?;
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
                        if let Err(_) = send_peers_request(socket_addr).await {
                            peer_book.disconnect_peer(&socket_addr);
                        }
                    }

                    for (socket_addr, _last_seen) in peer_book.gossiped.addresses.clone() {
                        if socket_addr != context.local_addr {
                            if let Err(_) = handshake_request(storage.get_latest_block_height(), socket_addr).await {
                                peer_book.disconnect_peer(&socket_addr);
                            }
                        }
                    }
                }

                // Maintain a connection with existing peers and update last seen
                for (socket_addr, _last_seen) in peer_book.peers.addresses.clone() {
                    // Ping peers and update last seen if there is a response
                    if socket_addr != context.local_addr {
                        if let Err(_) = send_ping(socket_addr).await {
                            peer_book.disconnect_peer(&socket_addr);
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
                        if let Err(_) = send_memory_pool_request(sync_handler.sync_node).await {
                            peer_book.disconnect_peer(&sync_handler.sync_node);
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
