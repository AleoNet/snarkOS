// Copyright (C) 2019-2022 Aleo Systems Inc.
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

use super::*;

impl<N: Network, E: Environment> Peer<N, E> {
    /// Initializes a handshake to connect with a peer.
    pub(crate) async fn handshake(state: State<N, E>, stream: TcpStream, connection_result: Option<ConnectionResult>) {
        spawn_task!(E::resources().procure_id(), {
            // Register our peer with state which internally sets up some channels.
            match Peer::initialize(&state, stream).await {
                Ok(peer) => {
                    // If the optional connection result router is given, report a successful connection result.
                    if let Some(router) = connection_result {
                        if router.send(Ok(())).is_err() {
                            warn!("Failed to report a successful connection");
                        }
                    }
                }
                Err(error) => {
                    trace!("{}", error);
                    // If the optional connection result router is given, report a failed connection result.
                    if let Some(router) = connection_result {
                        if router.send(Err(error)).is_err() {
                            warn!("Failed to report a failed connection");
                        }
                    }
                }
            };
        })
    }

    /// Initializes a new instance of `Peer`.
    async fn initialize(state: &State<N, E>, stream: TcpStream) -> Result<Self> {
        // Perform the handshake before proceeding.
        let (mut outbound_socket, peer_ip, node_type, status) = Self::perform_handshake(stream, *state.local_ip()).await?;

        // Initialize an MPSC channel for sending requests to the `Peer` struct.
        let (peer_router, peer_handler) = mpsc::channel(1024);

        // Construct the peer.
        let peer = Peer {
            state: state.clone(),
            peer_router,
            listener_ip: Arc::new(peer_ip),
            version: Arc::new(RwLock::new(0)),
            node_type: Arc::new(RwLock::new(node_type)),
            status: Arc::new(RwLock::new(status)),
            block_height: Arc::new(RwLock::new(0)),
            last_seen: Arc::new(RwLock::new(Instant::now())),
            seen_inbound_blocks: Default::default(),
            seen_inbound_transactions: Default::default(),
            seen_outbound_blocks: Default::default(),
            seen_outbound_transactions: Default::default(),
        };

        // Initialize the peer handler.
        peer.clone().handler(outbound_socket, peer_handler).await;

        // Add an entry for this `Peer` in the connected peers.
        state
            .peers()
            .router()
            .send(PeersRequest::PeerConnected(*peer.ip(), peer.clone()))
            .await?;

        Ok(peer)
    }

    /// Performs the handshake protocol, returning the listener IP of the peer upon success.
    async fn perform_handshake(
        stream: TcpStream,
        local_ip: SocketAddr,
    ) -> Result<(Framed<TcpStream, MessageCodec<N>>, SocketAddr, NodeType, Status)> {
        // Construct the socket.
        let mut outbound_socket = Framed::<TcpStream, MessageCodec<N>>::new(stream, Default::default());

        // Get the IP address of the peer.
        let mut peer_ip = outbound_socket.get_ref().peer_addr()?;

        // Retrieve the genesis block header.
        let genesis_header = BlockHeader::<N>::genesis();

        // Send a challenge request to the peer.
        let message = Message::<N>::ChallengeRequest(
            E::MESSAGE_VERSION,
            ALEO_MAXIMUM_FORK_DEPTH,
            E::NODE_TYPE,
            E::status().get(),
            local_ip.port(),
        );
        trace!("Sending '{}-A' to {}", message.name(), peer_ip);
        outbound_socket.send(message).await?;

        // Wait for the counterparty challenge request to come in.
        let (node_type, status) = match outbound_socket.next().await {
            Some(Ok(message)) => {
                // Process the message.
                trace!("Received '{}-B' from {}", message.name(), peer_ip);
                match message {
                    Message::ChallengeRequest(version, fork_depth, node_type, peer_status, listener_port) => {
                        // Ensure the message protocol version is not outdated.
                        if version < E::MESSAGE_VERSION {
                            warn!("Dropping {peer_ip} on version {version} (outdated)");

                            // Send the disconnect message.
                            outbound_socket
                                .send(Message::Disconnect(DisconnectReason::OutdatedClientVersion))
                                .await?;

                            bail!("Dropping {peer_ip} on version {version} (outdated)");
                        }
                        // Ensure the maximum fork depth is correct.
                        if fork_depth != ALEO_MAXIMUM_FORK_DEPTH {
                            // Send the disconnect message.
                            outbound_socket
                                .send(Message::Disconnect(DisconnectReason::InvalidForkDepth))
                                .await?;

                            bail!("Dropping {peer_ip} for an incorrect maximum fork depth of {fork_depth}");
                        }
                        // If this node is not a beacon node and is syncing, the peer is a beacon node, and this node is ahead, proceed to disconnect.
                        if E::NODE_TYPE != NodeType::Beacon && E::status().is_syncing() && node_type == NodeType::Beacon {
                            // Send the disconnect message.
                            outbound_socket
                                .send(Message::Disconnect(DisconnectReason::YouNeedToSyncFirst))
                                .await?;

                            bail!("Dropping {peer_ip} as this node is ahead");
                        }
                        // If this node is a beacon node, the peer is not a beacon node and is syncing, and the peer is ahead, proceed to disconnect.
                        if E::NODE_TYPE == NodeType::Beacon && node_type != NodeType::Beacon && peer_status == Status::Syncing {
                            // Send the disconnect message.
                            outbound_socket
                                .send(Message::Disconnect(DisconnectReason::INeedToSyncFirst))
                                .await?;

                            bail!("Dropping {peer_ip} as this node is ahead");
                        }
                        // Verify the listener port.
                        if peer_ip.port() != listener_port {
                            // Update the peer IP to the listener port.
                            peer_ip.set_port(listener_port);

                            // Ensure the claimed listener port is open.
                            if let Err(error) =
                                timeout(Duration::from_millis(E::CONNECTION_TIMEOUT_IN_MILLIS), TcpStream::connect(peer_ip)).await
                            {
                                // Send the disconnect message.
                                let message = Message::Disconnect(DisconnectReason::YourPortIsClosed(listener_port));
                                outbound_socket.send(message).await?;

                                bail!("Unable to reach '{peer_ip}': '{:?}'", error);
                            }
                        }
                        // Send the challenge response.
                        let message = Message::ChallengeResponse(Data::Object(genesis_header.clone()));
                        trace!("Sending '{}-B' to {peer_ip}", message.name());
                        outbound_socket.send(message).await?;

                        (node_type, peer_status)
                    }
                    Message::Disconnect(reason) => {
                        bail!("Peer {peer_ip} disconnected for the following reason: {:?}", reason);
                    }
                    message => {
                        bail!("Expected challenge request, received '{}' from {peer_ip}", message.name());
                    }
                }
            }
            // An error occurred.
            Some(Err(error)) => bail!("Failed to get challenge request from {peer_ip}: {:?}", error),
            // Did not receive anything.
            None => bail!("Dropped prior to challenge request of {peer_ip}"),
        };

        // Wait for the challenge response to come in.
        match outbound_socket.next().await {
            Some(Ok(message)) => {
                // Process the message.
                trace!("Received '{}-A' from {peer_ip}", message.name());
                match message {
                    Message::ChallengeResponse(block_header) => {
                        // Perform the deferred non-blocking deserialization of the block header.
                        let block_header = block_header.deserialize().await?;
                        match block_header == genesis_header {
                            true => {
                                // Send the first `Ping` message to the peer.
                                let message = Message::Ping(E::MESSAGE_VERSION, ALEO_MAXIMUM_FORK_DEPTH, E::NODE_TYPE, E::status().get());
                                trace!("Sending '{}' to {}", message.name(), peer_ip);
                                outbound_socket.send(message).await?;

                                Ok((outbound_socket, peer_ip, node_type, status))
                            }
                            false => bail!("Challenge response from {peer_ip} failed, received '{block_header}'"),
                        }
                    }
                    Message::Disconnect(reason) => {
                        bail!("Peer {peer_ip} disconnected for the following reason: {:?}", reason)
                    }
                    message => bail!("Expected challenge response, received '{}' from {peer_ip}", message.name()),
                }
            }
            // An error occurred.
            Some(Err(error)) => bail!("Failed to get challenge response from {peer_ip}: {:?}", error),
            // Did not receive anything.
            None => bail!("Failed to get challenge response from {peer_ip}, peer has disconnected"),
        }
    }
}
