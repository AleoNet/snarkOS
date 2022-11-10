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

use crate::{Peer, Router, ALEO_MAXIMUM_FORK_DEPTH};
use snarkos_node_executor::{Executor, NodeType, Status};
use snarkos_node_messages::{ChallengeRequest, ChallengeResponse, Data, DisconnectReason, Message, MessageCodec, Ping};
use snarkvm::prelude::{Block, FromBytes, Network};

use anyhow::{bail, Result};
use core::time::Duration;
use futures::SinkExt;
use tokio::{net::TcpStream, time::timeout};
use tokio_stream::StreamExt;
use tokio_util::codec::Framed;

#[async_trait]
pub trait Handshake: Executor {
    /// The minimum number of peers required to maintain connections with.
    const MINIMUM_NUMBER_OF_PEERS: usize = 1;
    /// The maximum number of peers permitted to maintain connections with.
    const MAXIMUM_NUMBER_OF_PEERS: usize = 21;

    /// Performs the handshake protocol, returning the peer upon success.
    async fn handshake<N: Network>(router: Router<N>, stream: TcpStream) -> Result<Peer<N>> {
        // Construct the socket.
        let mut outbound_socket = Framed::<TcpStream, MessageCodec<N>>::new(stream, Default::default());

        // Get the IP address of the peer.
        let mut peer_ip = outbound_socket.get_ref().peer_addr()?;

        // TODO (howardwu): Make this step more efficient (by not deserializing every time).
        // Retrieve the genesis block header.
        let genesis_header = *Block::<N>::from_bytes_le(N::genesis_bytes())?.header();

        // Send a challenge request to the peer.
        let message = Message::<N>::ChallengeRequest(ChallengeRequest {
            version: Message::<N>::VERSION,
            fork_depth: ALEO_MAXIMUM_FORK_DEPTH,
            node_type: Self::node_type(),
            status: Self::status().get(),
            listener_port: router.local_ip().port(),
        });
        trace!("Sending '{}-A' to '{peer_ip}'", message.name());
        outbound_socket.send(message).await?;

        // Wait for the counterparty challenge request to come in.
        let (node_type, status) = match outbound_socket.next().await {
            Some(Ok(message)) => {
                // Process the message.
                trace!("Received '{}-B' from '{peer_ip}'", message.name());
                match message {
                    Message::ChallengeRequest(ChallengeRequest {
                        version,
                        fork_depth,
                        node_type,
                        status: peer_status,
                        listener_port,
                    }) => {
                        // Ensure the message protocol version is not outdated.
                        if version < Message::<N>::VERSION {
                            warn!("Dropping {peer_ip} on version {version} (outdated)");

                            // Send the disconnect message.
                            outbound_socket
                                .send(Message::Disconnect(DisconnectReason::OutdatedClientVersion.into()))
                                .await?;

                            bail!("Dropping {peer_ip} on version {version} (outdated)");
                        }
                        // Ensure the maximum fork depth is correct.
                        if fork_depth != ALEO_MAXIMUM_FORK_DEPTH {
                            // Send the disconnect message.
                            outbound_socket
                                .send(Message::Disconnect(DisconnectReason::InvalidForkDepth.into()))
                                .await?;

                            bail!("Dropping {peer_ip} for an incorrect maximum fork depth of {fork_depth}");
                        }
                        // If this node is not a beacon node and is syncing, the peer is a beacon node, and this node is ahead, proceed to disconnect.
                        if Self::NODE_TYPE != NodeType::Beacon
                            && Self::status().is_syncing()
                            && node_type == NodeType::Beacon
                        {
                            // Send the disconnect message.
                            outbound_socket
                                .send(Message::Disconnect(DisconnectReason::YouNeedToSyncFirst.into()))
                                .await?;

                            bail!("Dropping {peer_ip} as this node is ahead");
                        }
                        // If this node is a beacon node, the peer is not a beacon node and is syncing, and the peer is ahead, proceed to disconnect.
                        if Self::NODE_TYPE == NodeType::Beacon
                            && node_type != NodeType::Beacon
                            && peer_status == Status::Syncing
                        {
                            // Send the disconnect message.
                            outbound_socket
                                .send(Message::Disconnect(DisconnectReason::INeedToSyncFirst.into()))
                                .await?;

                            bail!("Dropping {peer_ip} as this node is ahead");
                        }
                        // Verify the listener port.
                        if peer_ip.port() != listener_port {
                            // Update the peer IP to the listener port.
                            peer_ip.set_port(listener_port);

                            // Ensure the claimed listener port is open.
                            if let Err(error) = timeout(
                                Duration::from_millis(Router::<N>::CONNECTION_TIMEOUT_IN_MILLIS),
                                TcpStream::connect(peer_ip),
                            )
                            .await
                            {
                                // Send the disconnect message.
                                let message =
                                    Message::Disconnect(DisconnectReason::YourPortIsClosed(listener_port).into());
                                outbound_socket.send(message).await?;

                                bail!("Unable to reach '{peer_ip}': '{:?}'", error);
                            }
                        }
                        // Send the challenge response.
                        let message =
                            Message::ChallengeResponse(ChallengeResponse { header: Data::Object(genesis_header) });
                        trace!("Sending '{}-B' to '{peer_ip}'", message.name());
                        outbound_socket.send(message).await?;

                        (node_type, peer_status)
                    }
                    Message::Disconnect(reason) => {
                        bail!("Peer {peer_ip} disconnected for the following reason: {:?}", reason);
                    }
                    message => {
                        bail!("Expected challenge request, received '{}' from '{peer_ip}'", message.name());
                    }
                }
            }
            // An error occurred.
            Some(Err(error)) => bail!("Failed to get challenge request from '{peer_ip}': {:?}", error),
            // Did not receive anything.
            None => bail!("Dropped prior to challenge request of {peer_ip}"),
        };

        // Wait for the challenge response to come in.
        match outbound_socket.next().await {
            Some(Ok(message)) => {
                // Process the message.
                trace!("Received '{}-A' from '{peer_ip}'", message.name());
                match message {
                    Message::ChallengeResponse(message) => {
                        // Perform the deferred non-blocking deserialization of the block header.
                        let block_header = match message.header.deserialize().await {
                            Ok(block_header) => block_header,
                            Err(error) => bail!("Handshake with {peer_ip} failed (incorrect block header): {error}"),
                        };
                        match block_header == genesis_header {
                            true => {
                                // Send the first `Ping` message to the peer.
                                let message = Message::Ping(Ping {
                                    version: Message::<N>::VERSION,
                                    fork_depth: ALEO_MAXIMUM_FORK_DEPTH,
                                    node_type: Self::NODE_TYPE,
                                    status: Self::status().get(),
                                });
                                trace!("Sending '{}' to '{peer_ip}'", message.name());
                                outbound_socket.send(message).await?;

                                // Initialize the peer.
                                Peer::initialize::<Self>(peer_ip, node_type, status, router, outbound_socket).await
                            }
                            false => bail!("Challenge response from '{peer_ip}' failed, received '{block_header}'"),
                        }
                    }
                    Message::Disconnect(reason) => {
                        bail!("Peer {peer_ip} disconnected for the following reason: {:?}", reason)
                    }
                    message => bail!("Expected challenge response, received '{}' from '{peer_ip}'", message.name()),
                }
            }
            // An error occurred.
            Some(Err(error)) => bail!("Failed to get challenge response from '{peer_ip}': {:?}", error),
            // Did not receive anything.
            None => bail!("Failed to get challenge response from '{peer_ip}', peer has disconnected"),
        }
    }
}
