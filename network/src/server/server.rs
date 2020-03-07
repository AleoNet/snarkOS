use crate::{
    context::Context,
    message::{Channel, MessageName},
    protocol::*,
};
use snarkos_consensus::{miner::MemoryPool as MemoryPoolStruct, ConsensusParameters};
use snarkos_errors::network::ServerError;
use snarkos_storage::BlockStorage;

use std::{
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

    /// Starts the server event loop.
    /// 1. Send a handshake request to all bootnodes.
    /// 2. Listen for and accept new tcp connections at local_address.
    /// 3. Manage peers via handshake and ping protocols.
    /// 4. Handle all messages sent to this server.
    /// 5. Start connection handler.
    pub async fn listen(mut self) -> Result<(), ServerError> {
        let local_address = self.context.local_address;
        let sender = self.sender.clone();

        let mut listener = TcpListener::bind(&local_address).await?;
        info!("listening at: {:?}", local_address);

        for bootnode in self.context.bootnodes.clone() {
            let bootnode_address = bootnode.parse::<SocketAddr>()?;

            if local_address != bootnode_address && !self.context.is_bootnode {
                info!("Connecting to bootnode: {:?}", bootnode_address);

                self.context
                    .handshakes
                    .write()
                    .await
                    .send_request(
                        1u64,
                        self.storage.get_latest_block_height(),
                        local_address,
                        bootnode_address,
                    )
                    .await?;
            }
        }

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
}
