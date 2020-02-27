use tokio::{
    net::TcpListener,
    sync::{mpsc, oneshot, Mutex},
    task,
};

use snarkos_consensus::{miner::MemoryPool as MemoryPoolStruct, ConsensusParameters};
use snarkos_errors::network::{ConnectError, ServerError};
use snarkos_storage::BlockStorage;

use crate::{
    context::Context,
    message::{Channel, MessageName},
    protocol::*,
};
use std::{net::SocketAddr, sync::Arc};

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

                let channel = self
                    .context
                    .connections
                    .write()
                    .await
                    .connect_and_store(bootnode_address)
                    .await?;

                info!("New connection to {:?}", bootnode_address);

                self.context
                    .handshakes
                    .write()
                    .await
                    .send_request(
                        channel.clone(),
                        1u64,
                        self.storage.get_latest_block_height(),
                        local_addr,
                    )
                    .await?;

                Self::spawn_connection_thread(channel, sender.clone());
            }
        }

        // Outer loop spawns one thread to accept new connections
        task::spawn(async move {
            loop {
                let (stream, peer_address) = listener.accept().await.unwrap();
                let connections = &mut context.connections.write().await;
                let channel = connections.store(peer_address, Channel::new(stream, peer_address).await.unwrap());

                info!("New connection to: {:?}", peer_address);

                // Inner loop spawns one thread per connection to read messages
                Self::spawn_connection_thread(channel, sender.clone());
            }
        });

        self.connection_handler().await;

        self.message_handler().await?;

        Ok(())
    }

    fn spawn_connection_thread(
        mut channel: Arc<Channel>,
        mut thread_sender: mpsc::Sender<(oneshot::Sender<Arc<Channel>>, MessageName, Vec<u8>, Arc<Channel>)>,
    ) {
        // Inner loop spawns one thread per connection to read messages
        task::spawn(async move {
            loop {
                let (tx, rx) = oneshot::channel();
                let (message_name, message_bytes) = channel.read().await.unwrap_or_else(silent_disconnect);
                if MessageName::from("disconnect") == message_name {
                    break;
                }
                thread_sender
                    .send((tx, message_name, message_bytes, channel.clone()))
                    .await
                    .expect("could not send to message handler");
                channel = rx.await.expect("message handler errored");

                fn silent_disconnect(error: ConnectError) -> (MessageName, Vec<u8>) {
                    info!("Peer node disconnected due to error: {:?}", error);

                    (MessageName::from("disconnect"), vec![])
                }
            }
        });
    }
}
