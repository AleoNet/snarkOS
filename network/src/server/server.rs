use crate::{
    bootnodes::MAINNET_BOOTNODES,
    context::Context,
    message::{Channel, MessageName},
    protocol::*,
};
use snarkos_consensus::{miner::MemoryPool as MemoryPoolStruct, ConsensusParameters};
use snarkos_errors::network::ServerError;
use snarkos_storage::BlockStorage;

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

/// The main networking component of a node.
pub struct Server {
    pub consensus: ConsensusParameters,
    pub context: Arc<Context>,
    pub storage: Arc<BlockStorage>,
    pub memory_pool_lock: Arc<Mutex<MemoryPoolStruct>>,
    pub sync_handler_lock: Arc<Mutex<SyncHandler>>,
    pub connection_frequency: u64,
    pub sender: mpsc::Sender<(oneshot::Sender<Arc<Channel>>, MessageName, Vec<u8>, Arc<Channel>)>,
    pub receiver: mpsc::Receiver<(oneshot::Sender<Arc<Channel>>, MessageName, Vec<u8>, Arc<Channel>)>,
}

impl Server {
    /// Constructs a new `Server`.
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
            sync_handler_lock,
            connection_frequency,
        }
    }

    /// Send a handshake request to a node at address without blocking the server listener.
    fn send_handshake_non_blocking(&self, address: SocketAddr) {
        let context = self.context.clone();
        let storage = self.storage.clone();

        task::spawn(async move {
            context
                .handshakes
                .write()
                .await
                .send_request(1u64, storage.get_latest_block_height(), context.local_address, address)
                .await
                .unwrap_or_else(|error| {
                    info!("Failed to connect to address: {:?}", error);
                    ()
                });
        });
    }

    /// Send a handshake request to all bootnodes from config.
    async fn connect_bootnodes(&mut self) -> Result<(), ServerError> {
        let local_address = self.context.local_address;
        let hardcoded_bootnodes = MAINNET_BOOTNODES
            .iter()
            .map(|node| (*node).to_string())
            .collect::<Vec<String>>();

        for bootnode in self.context.bootnodes.clone() {
            // Bootnodes should not connect to hardcoded bootnodes.
            if self.context.is_bootnode && hardcoded_bootnodes.contains(&bootnode) {
                continue;
            }

            let bootnode_address = bootnode.parse::<SocketAddr>()?;

            if local_address != bootnode_address {
                info!("Connecting to bootnode: {:?}", bootnode_address);

                self.send_handshake_non_blocking(bootnode_address);
            }
        }

        Ok(())
    }

    /// Send a handshake request to every peer this server previously connected to.
    async fn connect_peers_from_storage(&mut self) -> Result<(), ServerError> {
        if let Ok(serialized_peers_option) = self.storage.get_peer_book() {
            if let Some(serialized_peers) = serialized_peers_option {
                let stored_connected_peers: HashMap<SocketAddr, DateTime<Utc>> =
                    bincode::deserialize(&serialized_peers)?;

                for (stored_peer, _old_time) in stored_connected_peers {
                    info!("Connecting to stored peer: {:?}", stored_peer);

                    self.send_handshake_non_blocking(stored_peer);
                }
            }
        }

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
            loop {
                // Use a oneshot channel to give channel control to the message handler after reading from the channel.
                let (tx, rx) = oneshot::channel();

                // Read the next message from the channel. This is a blocking operation.
                let (message_name, message_bytes) = channel.read().await.unwrap_or_else(|error| {
                    info!("Peer node disconnected due to error: {:?}", error);

                    (MessageName::from("disconnect"), vec![])
                });

                // Break out of the loop if the peer disconnects.
                if MessageName::from("disconnect") == message_name {
                    break;
                }

                // Send the successful read data to the message handler.
                message_handler_sender
                    .send((tx, message_name, message_bytes, channel.clone()))
                    .await
                    .expect("could not send to message handler");

                // Wait for the message handler to give back channel control.
                channel = rx.await.expect("message handler errored");
            }
        });
    }

    /// Starts the server event loop.
    /// 1. Send a handshake request to all bootnodes.
    /// 2. Send a handshake request to all stored peers.
    /// 3. Listen for and accept new tcp connections at local_address.
    /// 4. Manage peers via handshake and ping protocols.
    /// 5. Handle all messages sent to this server.
    /// 6. Start connection handler.
    pub async fn listen(mut self) -> Result<(), ServerError> {
        let local_address = self.context.local_address;

        let mut listener = TcpListener::bind(&local_address).await?;
        info!("listening at: {:?}", local_address);

        self.connect_bootnodes().await?;
        self.connect_peers_from_storage().await?;

        let sender = self.sender.clone();
        let storage = self.storage.clone();
        let context = self.context.clone();

        // Outer loop spawns one thread to accept new connections.
        task::spawn(async move {
            loop {
                let (stream, peer_address) = listener.accept().await.expect("Listener failed to accept connection");

                // Check if we have too many connected peers
                if context.peer_book.read().await.connected_total() >= context.max_peers {
                    stream
                        .shutdown(Shutdown::Write)
                        .expect("Failed to shutdown peer stream");
                } else {
                    // Follow handshake protocol and drop peer connection if unsuccessful.
                    if let Ok(handshake) = context
                        .handshakes
                        .write()
                        .await
                        .receive_any(
                            1u64,
                            storage.get_latest_block_height(),
                            local_address,
                            peer_address,
                            stream,
                        )
                        .await
                    {
                        context.connections.write().await.store_channel(&handshake.channel);

                        // Inner loop spawns one thread per connection to read messages
                        Self::spawn_connection_thread(handshake.channel.clone(), sender.clone());
                    }
                }
            }
        });

        self.connection_handler().await;

        self.message_handler().await?;

        Ok(())
    }
}
