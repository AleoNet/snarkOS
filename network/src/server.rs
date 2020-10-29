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
    environment::Environment,
    external::{message::MessageName, message_types::GetSync, protocol::*, Channel, GetMemoryPool},
    peer_manager::PeerManager,
    ReceiveHandler,
    SendHandler,
    SyncManager,
};
use snarkos_errors::{
    consensus::ConsensusError,
    network::{ConnectError, PingProtocolError, SendError, ServerError},
    objects::BlockError,
    storage::StorageError,
};

use std::{fmt, net::Shutdown, sync::Arc, time::Duration};
use tokio::{
    net::TcpListener,
    sync::{mpsc, oneshot, Mutex, RwLock},
    task,
    time::sleep,
};

pub type Sender = mpsc::Sender<(oneshot::Sender<Arc<Channel>>, MessageName, Vec<u8>, Arc<Channel>)>;
pub type Receiver = mpsc::Receiver<(oneshot::Sender<Arc<Channel>>, MessageName, Vec<u8>, Arc<Channel>)>;

#[derive(Debug)]
pub enum NetworkError {
    Bincode(Box<bincode::ErrorKind>),
    Bincode2(bincode::ErrorKind),
    BlockError(BlockError),
    ConnectError(ConnectError),
    ConsensusError(ConsensusError),
    IOError(std::io::Error),
    Error(anyhow::Error),
    PeerAddressIsLocalAddress,
    PeerAlreadyConnected,
    PeerAlreadyDisconnected,
    PeerAlreadyExists,
    PeerBookFailedToLoad,
    PeerBookIsCorrupt,
    PeerBookMissingPeer,
    PeerCountInvalid,
    PeerHasNeverConnected,
    PeerIsDisconnected,
    PeerIsMissingNonce,
    PeerIsReusingNonce,
    PeerUnauthorized,
    PeerWasNotSetToConnecting,
    PingProtocolError(PingProtocolError),
    ReceiveHandlerAlreadySetPeerSender,
    ReceiveHandlerMissingPeerManager,
    ReceiveHandlerMissingPeerSender,
    SendError(SendError),
    SendHandlerPendingRequestsMissing,
    SendRequestUnauthorized,
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

impl From<ConsensusError> for NetworkError {
    fn from(error: ConsensusError) -> Self {
        NetworkError::ConsensusError(error)
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

impl From<anyhow::Error> for NetworkError {
    fn from(error: anyhow::Error) -> Self {
        NetworkError::Error(error)
    }
}

impl From<NetworkError> for anyhow::Error {
    fn from(error: NetworkError) -> Self {
        error!("{}", error);
        Self::msg(error.to_string())
    }
}

/// A core data structure for operating the networking stack of this node.
pub struct Server {
    /// The parameters and settings of this node server.
    environment: Environment,
    /// The send handler of this node server.
    send_handler: SendHandler,
    /// The receive handler of this node server.
    receive_handler: ReceiveHandler,

    // TODO (howardwu): Uncomment this.
    peer_manager: PeerManager,
    // sync_manager: Arc<Mutex<SyncManager>>,

    // TODO (howardwu): Remove this.
    sender: Sender,
    receiver: Receiver,
}

impl Server {
    /// Creates a new instance of `Server`.
    // pub fn new(environment: &mut Environment, sync_manager: Arc<Mutex<SyncManager>>) -> Self {
    pub async fn new(environment: &mut Environment) -> Result<Self, NetworkError> {
        // Create a send handler.
        let send_handler = SendHandler::new();
        // Create a receive handler.
        let receive_handler = ReceiveHandler::new(send_handler.clone());

        let (sender, receiver) = mpsc::channel(1024);

        let peer_manager = PeerManager::new(environment, send_handler.clone(), receive_handler.clone())?;
        peer_manager.initialize().await?;

        environment.set_managers(peer_manager.clone());

        Ok(Self {
            environment: environment.clone(),
            send_handler,
            receive_handler,
            peer_manager,
            sender,
            receiver,
            // peer_manager,
            // sync_manager,
        })
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
        let environment = self.environment.clone();
        let receive_handler = self.receive_handler.clone();

        task::spawn(async move {
            loop {
                info!("Hello a?");
                if let Err(error) = receive_handler.clone().listen(environment.clone()).await {
                    // TODO: Handle receiver error appropriately with tracing and server state updates.
                    error!("Receive handler errored with {}", error);
                    sleep(Duration::from_secs(10)).await;
                }
            }
        });

        loop {
            info!("Hello b?");
            self.peer_manager.clone().update().await?;

            sleep(Duration::from_secs(10)).await;
        }

        // TODO (howardwu): Delete this.
        // Prepare to spawn the main loop.
        // let environment = self.environment.clone();
        // let sender = self.sender.clone();
        // let mut peer_manager = self.peer_manager.clone();
        // let peer_manager_og = PeerManager::new(environment.clone()).await?;
        // let mut peer_manager = PeerManager::new(environment.clone()).await?;
        // let sync_manager = self.environment.sync_manager().await.clone();
        // let sync_manager2 = sync_manager.clone();

        // TODO (howardwu): Delete this.
        // // TODO (howardwu): Find the actual address of this node.
        // // 1. Initialize TCP listener and accept new TCP connections.
        // let local_address = peer_manager_og.local_address();
        // debug!("Starting listener at {:?}...", local_address);
        // let mut listener = TcpListener::bind(&local_address).await?;
        // info!("Listening at {:?}", local_address);

        // TODO (howardwu): Delete this.
        // // 2. Spawn a new thread to handle new connections.
        // task::spawn(async move {
        //     debug!("Starting thread for handling connection requests");
        //     loop {
        //         // // Start listener for handling connection requests.
        //         // let (reader, remote_address) = match listener.accept().await {
        //         //     Ok((reader, remote_address)) => {
        //         //         info!("Received connection request from {}", remote_address);
        //         //         (reader, remote_address)
        //         //     }
        //         //     Err(error) => {
        //         //         error!("Failed to accept connection request\n{}", error);
        //         //         continue;
        //         //     }
        //         // };
        //
        //         // // Fetch the current number of connected peers.
        //         // let number_of_connected_peers = peer_manager.number_of_connected_peers().await;
        //         // trace!("Connected with {} peers", number_of_connected_peers);
        //         //
        //         // // Check that the maximum number of peers has not been reached.
        //         // if number_of_connected_peers >= environment.maximum_number_of_peers() {
        //         //     warn!("Maximum number of peers is reached, this connection request is being dropped");
        //         //     match reader.shutdown(Shutdown::Write) {
        //         //         Ok(_) => {
        //         //             debug!("Closed connection with {}", remote_address);
        //         //             continue;
        //         //         }
        //         //         // TODO (howardwu): Evaluate whether to return this error, or silently continue.
        //         //         Err(error) => {
        //         //             error!("Failed to close connection with {}\n{}", remote_address, error);
        //         //             continue;
        //         //         }
        //         //     }
        //         // }
        //
        //         // // Follow handshake protocol and drop peer connection if unsuccessful.
        //         // let height = environment.current_block_height().await;
        //         //
        //         // // TODO (raychu86) Establish a formal node version
        //         // if let Some((handshake, discovered_local_address, version_message)) = environment
        //         //     .receive_handler()
        //         //     .receive_connection_request(&environment, 1u64, height, remote_address, reader)
        //         //     .await
        //         // {
        //         //     // Bootstrap discovery of local node IP via VERACK responses
        //         //     {
        //         //         let local_address = peer_manager.local_address();
        //         //         if local_address != discovered_local_address {
        //         //             peer_manager.set_local_address(discovered_local_address).await;
        //         //             info!("Discovered local address: {:?}", local_address);
        //         //         }
        //         //     }
        //         //     // Store the channel established with the handshake
        //         //     peer_manager.add_channel(&handshake.channel);
        //         //
        //         //     if let Some(version) = version_message {
        //         //         // If our peer has a longer chain, send a sync message
        //         //         if version.height > environment.current_block_height().await {
        //         //             // Update the sync node if the sync_handler is Idle
        //         //             if let Ok(mut sync_handler) = sync_manager.try_lock() {
        //         //                 if !sync_handler.is_syncing() {
        //         //                     sync_handler.sync_node_address = handshake.channel.address;
        //         //
        //         //                     if let Ok(block_locator_hashes) =
        //         //                         environment.storage_read().await.get_block_locator_hashes()
        //         //                     {
        //         //                         if let Err(err) =
        //         //                             handshake.channel.write(&GetSync::new(block_locator_hashes)).await
        //         //                         {
        //         //                             error!(
        //         //                                 "Error sending GetSync message to {}, {}",
        //         //                                 handshake.channel.address, err
        //         //                             );
        //         //                         }
        //         //                     }
        //         //                 }
        //         //             }
        //         //         }
        //         //     }
        //         //
        //         //     // Inner loop spawns one thread per connection to read messages
        //         //     Self::spawn_connection_thread(handshake.channel.clone(), sender.clone());
        //         // }
        //     }
        // });

        // TODO (howardwu): Save and migrate this.
        {
            // let peer_manager_og = self
            //     .environment
            //     .peer_manager
            //     .ok_or(NetworkError::ReceiveHandlerMissingPeerSender)?;
            // let sync_manager = self.environment.sync_manager().await.clone();
            //
            // // 3. Start the connection handler.
            // debug!("Starting connection handler");
            // let peer_manager_2 = peer_manager_og.clone();
            // task::spawn(async move {
            //     sync_manager
            //         .try_lock()
            //         .unwrap()
            //         .connection_handler(peer_manager_2.read().await)
            //         .await;
            // });
            //
            // task::spawn(async move {
            //     // self.peer_manager.handler().await;
            //     peer_manager_og.read().await.handler().await;
            // });
        }

        // TODO (howardwu): Delete this.
        // // 4. Start the message handler.
        // debug!("Starting message handler");
        // self.environment
        //     .receive_handler()
        //     .message_handler(&self.environment, &mut self.receiver)
        //     .await;

        Ok(())
    }

    // TODO (howardwu): Delete this.
    // /// Spawns one thread per peer tcp connection to read messages.
    // /// Each thread is given a handle to the channel and a handle to the server mpsc sender.
    // /// To ensure concurrency, each connection thread sends a tokio oneshot sender handle with every message to the server mpsc receiver.
    // /// The thread then waits for the oneshot receiver to receive a signal from the server before reading again.
    // fn spawn_connection_thread(
    //     mut channel: Arc<Channel>,
    //     mut message_handler_sender: mpsc::Sender<(oneshot::Sender<Arc<Channel>>, MessageName, Vec<u8>, Arc<Channel>)>,
    // ) {
    //     task::spawn(async move {
    //         // // Determines the criteria for disconnecting from a peer.
    //         // fn should_disconnect(failure_count: &u8) -> bool {
    //         //     // Tolerate up to 10 failed communications.
    //         //     *failure_count >= 10
    //         // }
    //         //
    //         // // Logs the failure and determines whether to disconnect from a peer.
    //         // async fn handle_failure<T: std::fmt::Display>(
    //         //     failure: &mut bool,
    //         //     failure_count: &mut u8,
    //         //     disconnect_from_peer: &mut bool,
    //         //     error: T,
    //         // ) {
    //         //     // Only increment failure_count if we haven't seen a failure yet.
    //         //     if !*failure {
    //         //         // Update the state to reflect a new failure.
    //         //         *failure = true;
    //         //         *failure_count += 1;
    //         //         warn!(
    //         //             "Connection errored {} time(s) (error message: {})",
    //         //             failure_count, error
    //         //         );
    //         //
    //         //         // Determine if we should disconnect.
    //         //         *disconnect_from_peer = should_disconnect(failure_count);
    //         //     } else {
    //         //         debug!("Connection errored again in the same loop (error message: {})", error);
    //         //     }
    //         //
    //         //     // Sleep for 10 seconds
    //         //     tokio::time::delay_for(std::time::Duration::from_secs(10)).await;
    //         // }
    //         //
    //         // let mut failure_count = 0u8;
    //         // let mut disconnect_from_peer = false;
    //         //
    //         // loop {
    //         //     // // Initialize the failure indicator.
    //         //     // let mut failure = false;
    //         //     //
    //         //     // // Read the next message from the channel. This is a blocking operation.
    //         //     // let (message_name, message_bytes) = match channel.read().await {
    //         //     //     Ok((message_name, message_bytes)) => (message_name, message_bytes),
    //         //     //     Err(error) => {
    //         //     //         handle_failure(&mut failure, &mut failure_count, &mut disconnect_from_peer, error).await;
    //         //     //
    //         //     //         // Determine if we should send a disconnect message.
    //         //     //         match disconnect_from_peer {
    //         //     //             true => (MessageName::from("disconnect"), vec![]),
    //         //     //             false => continue,
    //         //     //         }
    //         //     //     }
    //         //     // };
    //         //     //
    //         //     // // Use a oneshot channel to give the channel control
    //         //     // // to the message handler after reading from the channel.
    //         //     // let (tx, rx) = oneshot::channel();
    //         //     //
    //         //     // // Send the successful read data to the message handler.
    //         //     // if let Err(error) = message_handler_sender
    //         //     //     .send((tx, message_name, message_bytes, channel.clone()))
    //         //     //     .await
    //         //     // {
    //         //     //     handle_failure(&mut failure, &mut failure_count, &mut disconnect_from_peer, error).await;
    //         //     //     continue;
    //         //     // };
    //         //     //
    //         //     // // Wait for the message handler to give back channel control.
    //         //     // match rx.await {
    //         //     //     Ok(peer_channel) => channel = peer_channel,
    //         //     //     Err(error) => {
    //         //     //         handle_failure(&mut failure, &mut failure_count, &mut disconnect_from_peer, error).await
    //         //     //     }
    //         //     // };
    //         //     //
    //         //     // // Break out of the loop if the peer disconnects.
    //         //     // if disconnect_from_peer {
    //         //     //     warn!("Disconnecting from an unreliable peer");
    //         //     //     break;
    //         //     // }
    //         // }
    //     });
    // }
}
