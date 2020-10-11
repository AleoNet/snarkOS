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

use crate::SyncManager;
use crate::{
    environment::Environment,
    external::{message::MessageName, message_types::GetSync, protocol::*, Channel, GetMemoryPool},
    peer_manager::PeerManager,
    ReceiveHandler, SendHandler,
};
use snarkos_errors::{
    network::{ConnectError, PingProtocolError, SendError, ServerError},
    objects::BlockError,
    storage::StorageError,
};

use std::{fmt, net::Shutdown, sync::Arc};
use tokio::{
    net::TcpListener,
    sync::{mpsc, oneshot, Mutex},
    task,
};
use tracing_futures::Instrument;

pub type Sender = mpsc::Sender<(oneshot::Sender<Arc<Channel>>, MessageName, Vec<u8>, Arc<Channel>)>;
pub type Receiver = mpsc::Receiver<(oneshot::Sender<Arc<Channel>>, MessageName, Vec<u8>, Arc<Channel>)>;

#[derive(Debug)]
pub enum NetworkError {
    Bincode(Box<bincode::ErrorKind>),
    Bincode2(bincode::ErrorKind),
    BlockError(BlockError),
    ConnectError(ConnectError),
    IOError(std::io::Error),
    PeerAlreadyConnected,
    PeerAlreadyDisconnected,
    PeerBookFailedToLoad,
    PeerCountInvalid,
    PeerIsDisconnected,
    PingProtocolError(PingProtocolError),
    SendError(SendError),
    StorageError(StorageError),
    SyncIntervalInvalid,
    TryLockError(tokio::sync::TryLockError),
}

impl From<BlockError> for NetworkError {
    fn from(error: BlockError) -> Self {
        NetworkError::BlockError(error)
    }
}

impl From<ConnectError> for NetworkError {
    fn from(error: ConnectError) -> Self {
        NetworkError::ConnectError(error)
    }
}

impl From<PingProtocolError> for NetworkError {
    fn from(error: PingProtocolError) -> Self {
        NetworkError::PingProtocolError(error)
    }
}

impl From<SendError> for NetworkError {
    fn from(error: SendError) -> Self {
        NetworkError::SendError(error)
    }
}

impl From<StorageError> for NetworkError {
    fn from(error: StorageError) -> Self {
        NetworkError::StorageError(error)
    }
}

impl From<Box<bincode::ErrorKind>> for NetworkError {
    fn from(error: Box<bincode::ErrorKind>) -> Self {
        NetworkError::Bincode(error)
    }
}

impl From<bincode::ErrorKind> for NetworkError {
    fn from(error: bincode::ErrorKind) -> Self {
        NetworkError::Bincode2(error)
    }
}

impl fmt::Display for NetworkError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<std::io::Error> for NetworkError {
    fn from(error: std::io::Error) -> Self {
        NetworkError::IOError(error)
    }
}

impl From<tokio::sync::TryLockError> for NetworkError {
    fn from(error: tokio::sync::TryLockError) -> Self {
        NetworkError::TryLockError(error)
    }
}

/// A core data structure for operating the networking stack of this node.
pub struct Server {
    environment: Environment,
    sender: Sender,
    receiver: Receiver,
    // peer_manager: PeerManager,
    // sync_manager: Arc<Mutex<SyncManager>>,
}

impl Server {
    /// Creates a new instance of `Server`.
    // pub fn new(environment: &mut Environment, sync_manager: Arc<Mutex<SyncManager>>) -> Self {
    pub fn new(environment: &mut Environment) -> Self {
        let (sender, receiver) = mpsc::channel(1024);

        environment.set_managers();

        Self {
            environment: environment.clone(),
            receiver,
            sender,
            // peer_manager,
            // sync_manager,
        }
    }

    ///
    /// Starts the server event loop.
    ///
    /// 1. Initialize TCP listener at `local_address` and accept new TCP connections.
    /// 2. Spawn a new thread to handle new connections.
    /// 3. Start the connection handler.
    /// 4. Start the message handler.
    ///
    pub async fn listen(mut self) -> Result<(), NetworkError> {
        // Prepare to spawn the main loop.
        let environment = self.environment.clone();
        let sender = self.sender.clone();
        // let mut peer_manager = self.peer_manager.clone();
        let peer_manager_og = PeerManager::new(environment.clone()).await?;
        let mut peer_manager = PeerManager::new(environment.clone()).await?;
        let sync_manager = self.environment.sync_manager().await.clone();
        let sync_manager2 = sync_manager.clone();

        // TODO (howardwu): Find the actual address of this node.
        // 1. Initialize TCP listener and accept new TCP connections.
        let local_address = peer_manager_og.local_address();
        debug!("Starting listener at {:?}...", local_address);
        let mut listener = TcpListener::bind(&local_address).await?;
        info!("Listening at {:?}", local_address);

        // 2. Spawn a new thread to handle new connections.
        let future = async move {
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
                if peer_manager.num_connected().await >= environment.max_peers() {
                    warn!("Rejected a connection request as this exceeds the maximum number of peers allowed");
                    if let Err(error) = reader.shutdown(Shutdown::Write) {
                        error!("Failed to shutdown peer reader ({})", error);
                    }
                    continue;
                }

                // Follow handshake protocol and drop peer connection if unsuccessful.
                let height = environment.current_block_height().await;

                // TODO (raychu86) Establish a formal node version
                if let Some((handshake, discovered_local_address, version_message)) = environment
                    .receive_handler()
                    .receive_connection_request(&environment, 1u64, height, remote_address, reader)
                    .await
                {
                    // Bootstrap discovery of local node IP via VERACK responses
                    {
                        let local_address = peer_manager.local_address();
                        if local_address != discovered_local_address {
                            peer_manager.set_local_address(discovered_local_address).await;
                            info!("Discovered local address: {:?}", local_address);
                        }
                    }
                    // Store the channel established with the handshake
                    peer_manager.add_channel(&handshake.channel);

                    if let Some(version) = version_message {
                        // If our peer has a longer chain, send a sync message
                        if version.height > environment.current_block_height().await {
                            // Update the sync node if the sync_handler is Idle
                            if let Ok(mut sync_handler) = sync_manager.try_lock() {
                                if !sync_handler.is_syncing() {
                                    sync_handler.sync_node_address = handshake.channel.address;

                                    if let Ok(block_locator_hashes) =
                                        environment.storage_read().await.get_block_locator_hashes()
                                    {
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
        };
        task::spawn(future.instrument(debug_span!("new_conn_handler")));

        // 3. Start the connection handler.
        debug!("Starting connection handler");
        let peer_manager_2 = peer_manager_og.clone();
        task::spawn(async move {
            sync_manager2
                .try_lock()
                .unwrap()
                .connection_handler(peer_manager_2)
                .await;
        });

        task::spawn(async move {
            // self.peer_manager.handler().await;
            peer_manager_og.handler().await;
        });

        self.environment
            .receive_handler()
            .message_handler(&self.environment, &mut self.receiver)
            .await;

        // 4. Start the message handler.
        debug!("Starting message handler");
        // self.message_handler().await;

        Ok(())
    }

    /// Spawns one thread per peer tcp connection to read messages.
    /// Each thread is given a handle to the channel and a handle to the server mpsc sender.
    /// To ensure concurrency, each connection thread sends a tokio oneshot sender handle with every message to the server mpsc receiver.
    /// The thread then waits for the oneshot receiver to receive a signal from the server before reading again.
    #[allow(clippy::type_complexity)]
    fn spawn_connection_thread(
        mut channel: Arc<Channel>,
        mut message_handler_sender: mpsc::Sender<(oneshot::Sender<Arc<Channel>>, MessageName, Vec<u8>, Arc<Channel>)>,
    ) {
        let peer_address = channel.address;
        let future = async move {
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
        };
        task::spawn(future.instrument(debug_span!("connection", addr = %peer_address)));
    }
}
