// Copyright (C) 2019-2021 Aleo Systems Inc.
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
    errors::ConnectError,
    external::message::*,
    ConnReader,
    ConnWriter,
    Environment,
    NetworkError,
    Receiver,
    Sender,
};

use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{atomic::AtomicU64, Arc},
};

use parking_lot::{Mutex, RwLock};
use tokio::{
    net::{TcpListener, TcpStream},
    task,
};

/// The map of remote addresses to their active writers.
pub type Channels = HashMap<SocketAddr, ConnWriter>;

/// A stateless component for handling inbound network traffic.
#[derive(Debug, Clone)]
pub struct Inbound {
    /// The producer for sending inbound messages to the server.
    sender: Sender,
    /// The consumer for receiving inbound messages to the server.
    receiver: Arc<Mutex<Option<Receiver>>>,
    /// The map of remote addresses to their active read channels.
    channels: Arc<RwLock<Channels>>,
    /// A counter for the number of received responses the handler processes.
    receive_response_count: Arc<AtomicU64>,
    /// A counter for the number of received responses that succeeded.
    receive_success_count: Arc<AtomicU64>,
    /// A counter for the number of received responses that failed.
    receive_failure_count: Arc<AtomicU64>,
}

impl Inbound {
    pub fn new(channels: Arc<RwLock<Channels>>) -> Self {
        // Initialize the sender and receiver.
        let (sender, receiver) = tokio::sync::mpsc::channel(1024);

        Self {
            sender,
            receiver: Arc::new(Mutex::new(Some(receiver))),
            channels,
            receive_response_count: Default::default(),
            receive_success_count: Default::default(),
            receive_failure_count: Default::default(),
        }
    }

    pub async fn listen(&self, environment: &mut Environment) -> Result<(), NetworkError> {
        let (listener_address, listener) = if let Some(addr) = environment.local_address() {
            let listener = TcpListener::bind(&addr).await?;
            (addr, listener)
        } else {
            let listener = TcpListener::bind("0.0.0.0:0").await?;
            let listener_address = listener.local_addr()?;
            environment.set_local_address(listener_address);
            (listener_address, listener)
        };
        info!("Listening at {}", listener_address);

        let inbound = self.clone();
        task::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((stream, remote_address)) => {
                        info!("Got a connection request from {}", remote_address);

                        match inbound
                            .connection_request(listener_address, remote_address, stream)
                            .await
                        {
                            Ok((channel, mut reader)) => {
                                // update the remote address to be the peer's listening address
                                let remote_address = channel.addr;
                                // Save the channel under the provided remote address
                                inbound.channels.write().insert(remote_address, channel);

                                let inbound = inbound.clone();
                                tokio::spawn(async move {
                                    inbound.listen_for_messages(&mut reader).await;
                                });
                            }
                            Err(e) => error!("Failed to accept a connection: {}", e),
                        }
                    }
                    Err(e) => error!("Failed to accept a connection: {}", e),
                }
            }
        });

        Ok(())
    }

    pub async fn listen_for_messages(&self, reader: &mut ConnReader) {
        let mut failure_count = 0u8;
        let mut disconnect_from_peer = false;
        let mut failure;

        loop {
            // Reset the failure indicator.
            failure = false;

            // Read the next message from the channel. This is a blocking operation.
            let message = match reader.read_message().await {
                Ok(message) => message,
                Err(error) => {
                    Self::handle_failure(&mut failure, &mut failure_count, &mut disconnect_from_peer, error);

                    // Determine if we should send a disconnect message.
                    match disconnect_from_peer {
                        true => {
                            self.route(Message::new(Direction::Internal, Payload::Disconnect(reader.addr)))
                                .await;

                            // TODO (howardwu): Remove this and rearchitect how disconnects are handled using the peer manager.
                            // TODO (howardwu): Implement a handler so the node does not lose state of undetected disconnects.
                            warn!("Disconnecting from an unreliable peer");
                            break; // the error has already been handled and reported
                        }
                        false => {
                            // Sleep for 10 seconds
                            tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                            continue;
                        }
                    }
                }
            };

            // Messages are received by a single tokio MPSC receiver with
            // the message name, bytes, and associated channel.
            //
            // The oneshot sender lets the connection task know when the message is handled.

            self.route(message).await
        }
    }

    /// Logs the failure and determines whether to disconnect from a peer.
    fn handle_failure(
        failure: &mut bool,
        failure_count: &mut u8,
        disconnect_from_peer: &mut bool,
        error: ConnectError,
    ) {
        // Only increment failure_count if we haven't seen a failure yet.
        if !*failure {
            // Update the state to reflect a new failure.
            *failure = true;
            *failure_count += 1;
            error!("Connection error: {}", error);

            // Determine if we should disconnect.
            *disconnect_from_peer = error.is_fatal() || *failure_count >= 10;

            if *disconnect_from_peer {
                debug!("Should disconnect from peer");
            }
        } else {
            debug!("Connection errored again in the same loop (error message: {})", error);
        }
    }

    #[inline]
    pub(crate) async fn route(&self, response: Message) {
        if let Err(err) = self.sender.send(response).await {
            error!("Failed to route a response for a message: {}", err);
        }
    }

    #[inline]
    pub(crate) fn take_receiver(&self) -> Receiver {
        self.receiver
            .lock()
            .take()
            .expect("The Inbound Receiver had already been taken!")
    }

    ///
    /// Receives a connection request with a given version message.
    ///
    /// Listens for the first message request from a remote peer.
    ///
    /// If the message is a Version:
    ///     1. Create a new handshake.
    ///     2. Send a handshake response.
    ///     3. If the response is sent successfully, store the handshake.
    ///     4. Return the handshake, your address as seen by sender, and the version message.
    ///
    /// If the message is a Verack:
    ///     1. Get the existing handshake.
    ///     2. Mark the handshake as accepted.
    ///     3. Send a request for peers.
    ///     4. Return the accepted handshake and your address as seen by sender.
    pub async fn connection_request(
        &self,
        listener_address: SocketAddr,
        remote_address: SocketAddr,
        stream: TcpStream,
    ) -> Result<(ConnWriter, ConnReader), NetworkError> {
        let (reader, writer) = stream.into_split();
        let mut writer = ConnWriter::new(remote_address, writer);
        let mut reader = ConnReader::new(remote_address, reader);

        // Read the next message from the channel.
        // Note: this is a blocking operation.
        let message = match reader.read_message().await {
            Ok(message) => message,
            Err(e) => {
                error!("An error occurred while handshaking with {}: {}", remote_address, e);
                return Err(NetworkError::InvalidHandshake);
            }
        };

        // Create and store a new handshake in the manager.
        if let Payload::Version(remote_version) = message.payload {
            // Create the remote address from the given peer address, and specified port from the version message.
            let remote_address = SocketAddr::new(remote_address.ip(), remote_version.listening_port);

            writer.addr = remote_address;
            reader.addr = remote_address;

            // TODO (raychu86): Establish a formal node version.
            let local_version = Version::new_with_rng(1u64, listener_address.port());

            // notify the server that the peer is being connected to
            self.sender
                .send(Message::new(
                    Direction::Internal,
                    Payload::ConnectingTo(remote_address, local_version.nonce),
                ))
                .await?;

            // Write a verack response to the remote peer.
            writer.write_message(&Payload::Verack(remote_version.nonce)).await?;

            // Write a version request to the remote peer.
            writer.write_message(&Payload::Version(local_version.clone())).await?;

            // Parse the inbound message into the message name and message bytes.
            let message = reader.read_message().await?;

            // Deserialize the message bytes into a verack message.
            if !matches!(message.payload, Payload::Verack(..)) {
                error!("{} didn't respond with a Verack during the handshake", remote_address);
                return Err(NetworkError::InvalidHandshake);
            }

            self.sender
                .send(Message::new(
                    Direction::Internal,
                    Payload::ConnectedTo(remote_address, local_version.nonce),
                ))
                .await?;

            debug!("Successfully handshaken with {}", remote_address);

            Ok((writer, reader))
        } else {
            error!("{} didn't send their Version during the handshake", remote_address);
            Err(NetworkError::InvalidHandshake)
        }
    }
}
