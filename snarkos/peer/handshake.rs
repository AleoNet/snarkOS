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

impl<N: Network> Peer<N> {
    /// Create a new instance of `Peer`.
    pub(super) async fn new<E: Environment>(state: &State<N, E>, stream: TcpStream) -> Result<Self> {
        // Construct the socket.
        let mut outbound_socket = Framed::new(stream, Default::default());

        // Perform the handshake before proceeding.
        let (peer_ip, node_type, status) = Peer::handshake::<E>(&mut outbound_socket, *state.local_ip()).await?;

        // Send the first `Ping` message to the peer.
        let message = Message::Ping(E::MESSAGE_VERSION, ALEO_MAXIMUM_FORK_DEPTH, E::NODE_TYPE, E::status().get());
        trace!("Sending '{}' to {}", message.name(), peer_ip);
        outbound_socket.send(message).await?;

        // Create a channel for this peer.
        let (outbound_router, outbound_handler) = mpsc::channel(1024);

        // Add an entry for this `Peer` in the connected peers.
        state
            .peers()
            .router()
            .send(PeersRequest::PeerConnected(peer_ip, outbound_router))
            .await?;

        Ok(Peer {
            listener_ip: peer_ip,
            version: 0,
            node_type,
            status,
            block_height: 0,
            last_seen: Instant::now(),
            outbound_socket,
            outbound_handler,
            seen_inbound_blocks: Default::default(),
            seen_inbound_transactions: Default::default(),
            seen_outbound_blocks: Default::default(),
            seen_outbound_transactions: Default::default(),
        })
    }

    /// Performs the handshake protocol, returning the listener IP of the peer upon success.
    async fn handshake<E: Environment>(
        outbound_socket: &mut Framed<TcpStream, MessageCodec<N>>,
        local_ip: SocketAddr,
    ) -> Result<(SocketAddr, NodeType, Status)> {
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
                        // If this node is not a sync node and is syncing, the peer is a sync node, and this node is ahead, proceed to disconnect.
                        if E::NODE_TYPE != NodeType::Beacon && E::status().is_syncing() && node_type == NodeType::Beacon {
                            // Send the disconnect message.
                            outbound_socket
                                .send(Message::Disconnect(DisconnectReason::YouNeedToSyncFirst))
                                .await?;

                            bail!("Dropping {peer_ip} as this node is ahead");
                        }
                        // If this node is a sync node, the peer is not a sync node and is syncing, and the peer is ahead, proceed to disconnect.
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
                            true => Ok((peer_ip, node_type, status)),
                            false => Err(anyhow!("Challenge response from {peer_ip} failed, received '{block_header}'")),
                        }
                    }
                    Message::Disconnect(reason) => {
                        bail!("Peer {peer_ip} disconnected for the following reason: {:?}", reason);
                    }
                    message => Err(anyhow!("Expected challenge response, received '{}' from {peer_ip}", message.name(),)),
                }
            }
            // An error occurred.
            Some(Err(error)) => Err(anyhow!("Failed to get challenge response from {peer_ip}: {:?}", error)),
            // Did not receive anything.
            None => Err(anyhow!("Failed to get challenge response from {peer_ip}, peer has disconnected")),
        }
    }
}
