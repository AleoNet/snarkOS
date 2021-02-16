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

use crate::{errors::NetworkError, message::*, ConnReader, ConnWriter, Environment, Receiver, Sender};

use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{atomic::AtomicU64, Arc},
};

use parking_lot::{Mutex, RwLock};
use rand::{thread_rng, Rng};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    task::{self, JoinHandle},
};

/// The map of remote addresses to their active writers.
pub type Channels = HashMap<SocketAddr, Arc<ConnWriter>>;

/// A stateless component for handling inbound network traffic.
#[derive(Debug, Clone)]
pub struct Inbound {
    /// The producer for sending inbound messages to the server.
    pub(crate) sender: Sender,
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
    /// The tasks dedicated to handling inbound messages.
    pub(crate) tasks: Arc<Mutex<HashMap<SocketAddr, JoinHandle<()>>>>,
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
            tasks: Default::default(),
        }
    }

    pub async fn listen(&self, environment: &mut Environment) -> Result<(), NetworkError> {
        // Generate the node name.
        let mut rng = thread_rng();
        let name = rng.gen();

        let (listener_address, listener) = if let Some(addr) = environment.local_address() {
            let listener = TcpListener::bind(&addr).await?;
            (listener.local_addr()?, listener)
        } else {
            let listener = TcpListener::bind("0.0.0.0:0").await?;
            let listener_address = listener.local_addr()?;
            (listener_address, listener)
        };
        environment.set_local_address(listener_address);
        environment.name = Some(name);
        info!("Node {:?} listening at {}", name, listener_address);

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
                                inbound.channels.write().insert(remote_address, Arc::new(channel));

                                let inbound_clone = inbound.clone();
                                let task = tokio::spawn(async move {
                                    inbound_clone.listen_for_messages(&mut reader).await;
                                });

                                inbound.tasks.lock().insert(remote_address, task);
                            }
                            Err(e) => {
                                error!("Failed to accept a connection: {}", e);
                                // FIXME(ljedrz/nkls): this should be done immediately, bypassing the message channel
                                let _ = inbound
                                    .sender
                                    .send(Message::new(Direction::Internal, Payload::Disconnect(remote_address)))
                                    .await;
                            }
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
                            // FIXME(ljedrz/nkls): this should be done immediately, bypassing the message channel
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
        error: NetworkError,
    ) {
        // Only increment failure_count if we haven't seen a failure yet.
        if !*failure {
            // Update the state to reflect a new failure.
            *failure = true;
            *failure_count += 1;
            error!("Network error: {}", error);

            // Determine if we should disconnect.
            *disconnect_from_peer = error.is_fatal() || *failure_count >= 10;

            if *disconnect_from_peer {
                debug!("Should disconnect from peer");
            }
        } else {
            debug!("A connection errored again in the same loop (error message: {})", error);
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
    /// Handles an incoming connection request, performing a secure handshake and establishing packet encryption.
    ///
    pub async fn connection_request(
        &self,
        listener_address: SocketAddr,
        remote_address: SocketAddr,
        stream: TcpStream,
    ) -> Result<(ConnWriter, ConnReader), NetworkError> {
        self.sender
            .send(Message::new(Direction::Internal, Payload::ConnectingTo(remote_address)))
            .await?;

        let (mut reader, mut writer) = stream.into_split();

        let builder = snow::Builder::with_resolver(
            crate::HANDSHAKE_PATTERN
                .parse()
                .expect("Invalid noise handshake pattern!"),
            Box::new(snow::resolvers::SodiumResolver),
        );
        let static_key = builder.generate_keypair()?.private;
        let noise_builder = builder.local_private_key(&static_key).psk(3, crate::HANDSHAKE_PSK);
        let mut noise = noise_builder.build_responder()?;
        let mut buffer: Box<[u8]> = vec![0u8; crate::MAX_MESSAGE_SIZE].into();
        let mut buf = [0u8; crate::NOISE_BUF_LEN]; // a temporary intermediate buffer to decrypt from

        // <- e
        reader.read_exact(&mut buf[..1]).await?;
        let len = buf[0] as usize;
        if len == 0 {
            return Err(NetworkError::InvalidHandshake);
        }
        let len = reader.read_exact(&mut buf[..len]).await?;
        noise.read_message(&buf[..len], &mut buffer)?;
        trace!("received e (XX handshake part 1/3)");

        // -> e, ee, s, es
        let own_version = Version::serialize(&Version::new(1u64, listener_address.port())).unwrap(); // TODO (raychu86): Establish a formal node version.
        let len = noise.write_message(&own_version, &mut buffer)?;
        writer.write_all(&[len as u8]).await?;
        writer.write_all(&buffer[..len]).await?;
        trace!("sent e, ee, s, es (XX handshake part 2/3)");

        // <- s, se, psk
        reader.read_exact(&mut buf[..1]).await?;
        let len = buf[0] as usize;
        if len == 0 {
            return Err(NetworkError::InvalidHandshake);
        }
        let len = reader.read_exact(&mut buf[..len]).await?;
        let len = noise.read_message(&buf[..len], &mut buffer)?;
        let peer_version = Version::deserialize(&buffer[..len])?;
        trace!("received s, se, psk (XX handshake part 3/3)");

        // the remote listening address
        let remote_listener = SocketAddr::from((remote_address.ip(), peer_version.listening_port));

        self.sender
            .send(Message::new(
                Direction::Internal,
                Payload::ConnectedTo(remote_address, Some(remote_listener)),
            ))
            .await?;

        let noise = Arc::new(Mutex::new(noise.into_transport_mode()?));
        let reader = ConnReader::new(remote_listener, reader, buffer.clone(), Arc::clone(&noise));
        let writer = ConnWriter::new(remote_listener, writer, buffer, noise);

        Ok((writer, reader))
    }
}
