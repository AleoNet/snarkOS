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

use crate::{Account, CLI};
use snarkos_consensus::BlockHeader;
use snarkos_environment::{
    helpers::{NodeType, Status},
    Environment,
};
use snarkvm::prelude::*;

#[cfg(feature = "rpc")]
use snarkos_rpc::{initialize_rpc_node, RpcContext};

#[cfg(any(feature = "test", feature = "prometheus"))]
use snarkos_metrics as metrics;

use anyhow::Result;
use once_cell::race::OnceBox;
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tokio::{net::TcpListener, sync::oneshot, task};
use tokio_util::codec::Framed;

#[macro_export]
macro_rules! spawn_task {
    // Spawns a new task, without a task ID.
    ($logic:block) => {{
        let (router, handler) = tokio::sync::oneshot::channel();
        // Register the task with the environment.
        // No need to provide an id, as the task will run indefinitely.
        E::resources().register_task(None, tokio::task::spawn(async move {
            // Notify the outer function that the task is ready.
            let _ = router.send(());
            $logic
        }));
        // Wait until the task is ready.
        let _ = handler.await;
    }};

    // Spawns a new task, without a task ID.
    ($logic:expr) => {{ $crate::spawn_task!(None, { $logic }) }};

    // Spawns a new task, with a task ID.
    ($id:expr, $logic:block) => {{
        let (router, handler) = tokio::sync::oneshot::channel();
        // Register the task with the environment.
        E::resources().register_task(Some($id), tokio::task::spawn(async move {
            // Notify the outer function that the task is ready.
            let _ = router.send(());
            $logic
            E::resources().deregister($id);
        }));
        // Wait until the task is ready.
        let _ = handler.await;
    }};

    // Spawns a new task, with a task ID.
    ($id:expr, $logic:expr) => {{ $crate::spawn_task!($id, { $logic }) }};
}

// TODO (raychu86): Move this declaration.
const ALEO_MAXIMUM_FORK_DEPTH: u32 = 4096;

use crate::message::{Data, DisconnectReason, Message, MessageCodec};

use ::rand::{prelude::IteratorRandom, rngs::OsRng, thread_rng, Rng};
use futures::SinkExt;
use std::{
    collections::{HashMap, HashSet},
    time::{Instant, SystemTime},
};
use tokio::{
    net::TcpStream,
    sync::{mpsc, RwLock},
    time::timeout,
};
use tokio_stream::StreamExt;

/// Shorthand for the parent half of the `Peer` outbound message channel.
pub(crate) type OutboundRouter<N> = mpsc::Sender<Message<N>>;
/// Shorthand for the child half of the `Peer` outbound message channel.
type OutboundHandler<N> = mpsc::Receiver<Message<N>>;

///
/// The state for each connected client.
///
pub(crate) struct Peer<N: Network> {
    /// The IP address of the peer, with the port set to the listener port.
    listener_ip: SocketAddr,
    /// The message version of the peer.
    version: u32,
    /// The node type of the peer.
    node_type: NodeType,
    /// The node type of the peer.
    status: Status,
    /// The block height of the peer.
    block_height: u32,
    /// The timestamp of the last message received from this peer.
    last_seen: Instant,
    /// The TCP socket that handles sending and receiving data with this peer.
    outbound_socket: Framed<TcpStream, MessageCodec<N>>,
    /// The `outbound_handler` half of the MPSC message channel, used to receive messages from peers.
    /// When a message is received on this `OutboundHandler`, it will be written to the socket.
    outbound_handler: OutboundHandler<N>,
    /// The map of block hashes to their last seen timestamp.
    seen_inbound_blocks: HashMap<N::BlockHash, SystemTime>,
    /// The map of transaction IDs to their last seen timestamp.
    seen_inbound_transactions: HashMap<N::TransactionID, SystemTime>,
    /// The map of peers to a map of block hashes to their last seen timestamp.
    seen_outbound_blocks: HashMap<N::BlockHash, SystemTime>,
    /// The map of peers to a map of transaction IDs to their last seen timestamp.
    seen_outbound_transactions: HashMap<N::TransactionID, SystemTime>,
}

impl<N: Network> Peer<N> {
    /// Create a new instance of `Peer`.
    async fn new<E: Environment>(state: &State<N, E>, stream: TcpStream) -> Result<Self> {
        // Construct the socket.
        let mut outbound_socket = Framed::new(stream, Default::default());

        // Perform the handshake before proceeding.
        let (peer_ip, node_type, status) = Peer::handshake::<E>(&mut outbound_socket, *state.local_ip).await?;

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

    /// Returns the IP address of the peer, with the port set to the listener port.
    fn peer_ip(&self) -> SocketAddr {
        self.listener_ip
    }

    /// Sends the given message to this peer.
    async fn send(&mut self, message: Message<N>) -> Result<()> {
        trace!("Sending '{}' to {}", message.name(), self.peer_ip());
        self.outbound_socket.send(message).await?;
        Ok(())
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
                            warn!("Dropping {} on version {} (outdated)", peer_ip, version);

                            // Send the disconnect message.
                            let message = Message::Disconnect(DisconnectReason::OutdatedClientVersion);
                            outbound_socket.send(message).await?;

                            bail!("Dropping {} on version {} (outdated)", peer_ip, version);
                        }
                        // Ensure the maximum fork depth is correct.
                        if fork_depth != ALEO_MAXIMUM_FORK_DEPTH {
                            // Send the disconnect message.
                            let message = Message::Disconnect(DisconnectReason::InvalidForkDepth);
                            outbound_socket.send(message).await?;

                            bail!("Dropping {} for an incorrect maximum fork depth of {}", peer_ip, fork_depth);
                        }
                        // If this node is not a sync node and is syncing, the peer is a sync node, and this node is ahead, proceed to disconnect.
                        if E::NODE_TYPE != NodeType::Beacon && E::status().is_syncing() && node_type == NodeType::Beacon {
                            // Send the disconnect message.
                            let message = Message::Disconnect(DisconnectReason::YouNeedToSyncFirst);
                            outbound_socket.send(message).await?;

                            bail!("Dropping {} as this node is ahead", peer_ip);
                        }
                        // If this node is a sync node, the peer is not a sync node and is syncing, and the peer is ahead, proceed to disconnect.
                        if E::NODE_TYPE == NodeType::Beacon && node_type != NodeType::Beacon && peer_status == Status::Syncing {
                            // Send the disconnect message.
                            let message = Message::Disconnect(DisconnectReason::INeedToSyncFirst);
                            outbound_socket.send(message).await?;

                            bail!("Dropping {} as this node is ahead", peer_ip);
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

                                bail!("Unable to reach '{}': '{:?}'", peer_ip, error);
                            }
                        }
                        // Send the challenge response.
                        let message = Message::ChallengeResponse(Data::Object(genesis_header.clone()));
                        trace!("Sending '{}-B' to {}", message.name(), peer_ip);
                        outbound_socket.send(message).await?;

                        (node_type, peer_status)
                    }
                    Message::Disconnect(reason) => {
                        bail!("Peer {} disconnected for the following reason: {:?}", peer_ip, reason);
                    }
                    message => {
                        bail!("Expected challenge request, received '{}' from {}", message.name(), peer_ip);
                    }
                }
            }
            // An error occurred.
            Some(Err(error)) => bail!("Failed to get challenge request from {}: {:?}", peer_ip, error),
            // Did not receive anything.
            None => bail!("Dropped prior to challenge request of {}", peer_ip),
        };

        // Wait for the challenge response to come in.
        match outbound_socket.next().await {
            Some(Ok(message)) => {
                // Process the message.
                trace!("Received '{}-A' from {}", message.name(), peer_ip);
                match message {
                    Message::ChallengeResponse(block_header) => {
                        // Perform the deferred non-blocking deserialization of the block header.
                        let block_header = block_header.deserialize().await?;
                        match block_header == genesis_header {
                            true => Ok((peer_ip, node_type, status)),
                            false => Err(anyhow!("Challenge response from {} failed, received '{}'", peer_ip, block_header)),
                        }
                    }
                    Message::Disconnect(reason) => {
                        bail!("Peer {} disconnected for the following reason: {:?}", peer_ip, reason);
                    }
                    message => Err(anyhow!(
                        "Expected challenge response, received '{}' from {}",
                        message.name(),
                        peer_ip
                    )),
                }
            }
            // An error occurred.
            Some(Err(error)) => Err(anyhow!("Failed to get challenge response from {}: {:?}", peer_ip, error)),
            // Did not receive anything.
            None => Err(anyhow!("Failed to get challenge response from {}, peer has disconnected", peer_ip)),
        }
    }

    /// A handler to process an individual peer.
    pub(super) async fn handler<E: Environment>(state: State<N, E>, stream: TcpStream, connection_result: Option<ConnectionResult>) {
        // Retrieve the peers router.
        let peers_router = state.peers().router().clone();

        // Procure a resource id to register the task with, as it might be terminated at any point in time.
        let peer_resource_id = E::resources().procure_id();
        E::resources().register_task(Some(peer_resource_id), task::spawn(async move {
            // Register our peer with state which internally sets up some channels.
            let mut peer = match Peer::new(&state, stream).await {
                Ok(peer) => {
                    // If the optional connection result router is given, report a successful connection result.
                    if let Some(router) = connection_result {
                        if router.send(Ok(())).is_err() {
                            warn!("Failed to report a successful connection");
                        }
                    }
                    peer
                }
                Err(error) => {
                    trace!("{}", error);
                    // If the optional connection result router is given, report a failed connection result.
                    if let Some(router) = connection_result {
                        if router.send(Err(error)).is_err() {
                            warn!("Failed to report a failed connection");
                        }
                    }
                    E::resources().deregister(peer_resource_id);
                    return;
                }
            };

            // Retrieve the peer IP.
            let peer_ip = peer.peer_ip();
            info!("Connected to {}", peer_ip);

            // Process incoming messages until this stream is disconnected.
            loop {
                tokio::select! {
                    // Message channel is routing a message outbound to the peer.
                    Some(mut message) = peer.outbound_handler.recv() => {
                        // Disconnect if the peer has not communicated back within the predefined time.
                        if peer.last_seen.elapsed() > Duration::from_secs(E::RADIO_SILENCE_IN_SECS) {
                            warn!("Peer {} has not communicated in {} seconds", peer_ip, peer.last_seen.elapsed().as_secs());
                            break;
                        } else {
                            // Ensure sufficient time has passed before needing to send the message.
                            let is_ready_to_send = match message {
                                Message::UnconfirmedBlock(block_height, block_hash, ref mut data) => {
                                    // Retrieve the last seen timestamp of this block for this peer.
                                    let last_seen = peer.seen_outbound_blocks.entry(block_hash).or_insert(SystemTime::UNIX_EPOCH);
                                    let is_ready_to_send = last_seen.elapsed().unwrap().as_secs() > E::RADIO_SILENCE_IN_SECS;

                                    // Update the timestamp for the peer and sent block.
                                    peer.seen_outbound_blocks.insert(block_hash, SystemTime::now());
                                    // Report the unconfirmed block height.
                                    if is_ready_to_send {
                                        trace!("Preparing to send 'UnconfirmedBlock {}' to {}", block_height, peer_ip);
                                    }

                                    // Perform non-blocking serialization of the block (if it hasn't been serialized yet).
                                    let serialized_block = Data::serialize(data.clone()).await.expect("Block serialization is bugged");
                                    let _ = std::mem::replace(data, Data::Buffer(serialized_block));

                                    is_ready_to_send
                                }
                                Message::UnconfirmedTransaction(ref mut data) => {
                                    let transaction = if let Data::Object(transaction) = data {
                                        transaction
                                    } else {
                                        panic!("Logic error: the transaction shouldn't have been serialized yet.");
                                    };

                                    // Retrieve the last seen timestamp of this transaction for this peer.
                                    let last_seen = peer
                                        .seen_outbound_transactions
                                        .entry(transaction.id())
                                        .or_insert(SystemTime::UNIX_EPOCH);
                                    let is_ready_to_send = last_seen.elapsed().unwrap().as_secs() > E::RADIO_SILENCE_IN_SECS;

                                    // Update the timestamp for the peer and sent transaction.
                                    peer.seen_outbound_transactions.insert(transaction.id(), SystemTime::now());
                                    // Report the unconfirmed block height.
                                    if is_ready_to_send {
                                        trace!(
                                            "Preparing to send 'UnconfirmedTransaction {}' to {}",
                                            transaction.id(),
                                            peer_ip
                                        );
                                    }

                                    // Perform non-blocking serialization of the transaction.
                                    let serialized_transaction = Data::serialize(data.clone()).await.expect("Transaction serialization is bugged");
                                    let _ = std::mem::replace(data, Data::Buffer(serialized_transaction));

                                    is_ready_to_send
                                }
                                Message::PeerResponse(_, _rtt_start) => {
                                    // Stop the clock on internal RTT.
                                    #[cfg(any(feature = "test", feature = "prometheus"))]
                                    metrics::histogram!(metrics::internal_rtt::PEER_REQUEST, _rtt_start.expect("rtt should be present with metrics enabled").elapsed());

                                    true
                                }
                                _ => true,
                            };
                            // Send the message if it is ready.
                            if is_ready_to_send {
                                // Route a message to the peer.
                                if let Err(error) = peer.send(message).await {
                                    warn!("[OutboundRouter] {}", error);
                                }
                            }
                        }
                    }
                    result = peer.outbound_socket.next() => match result {
                        // Received a message from the peer.
                        Some(Ok(message)) => {
                            // Disconnect if the peer has not communicated back within the predefined time.
                            match peer.last_seen.elapsed() > Duration::from_secs(E::RADIO_SILENCE_IN_SECS) {
                                true => {
                                    let last_seen = peer.last_seen.elapsed().as_secs();
                                    warn!("Failed to receive a message from {} in {} seconds", peer_ip, last_seen);
                                    break;
                                },
                                false => {
                                    // Update the last seen timestamp.
                                    peer.last_seen = Instant::now();
                                }
                            }

                            #[cfg(any(feature = "test", feature = "prometheus"))]
                            let rtt_start = Instant::now();

                            // Process the message.
                            trace!("Received '{}' from {}", message.name(), peer_ip);
                            match message {
                                Message::BlockRequest(start_block_height, end_block_height) => {
                                    #[cfg(any(feature = "test", feature = "prometheus"))]
                                    metrics::increment_counter!(metrics::message_counts::BLOCK_REQUEST);

                                    // // Ensure the request is within the accepted limits.
                                    // let number_of_blocks = end_block_height.saturating_sub(start_block_height);
                                    // if number_of_blocks > E::MAXIMUM_BLOCK_REQUEST {
                                    //     // Route a `Failure` to the ledger.
                                    //     let failure = format!("Attempted to request {} blocks", number_of_blocks);
                                    //     if let Err(error) = state.ledger().router().send(LedgerRequest::Failure(peer_ip, failure)).await {
                                    //         warn!("[Failure] {}", error);
                                    //     }
                                    //     continue;
                                    // }
                                    // // Retrieve the requested blocks.
                                    // let blocks = match state.ledger().reader().get_blocks(start_block_height, end_block_height) {
                                    //     Ok(blocks) => blocks,
                                    //     Err(error) => {
                                    //         // Route a `Failure` to the ledger.
                                    //         if let Err(error) = state.ledger().router().send(LedgerRequest::Failure(peer_ip, format!("{}", error))).await {
                                    //             warn!("[Failure] {}", error);
                                    //         }
                                    //         continue;
                                    //     }
                                    // };
                                    // // Send a `BlockResponse` message for each block to the peer.
                                    // for block in blocks {
                                    //     debug!("Sending 'BlockResponse {}' to {}", block.height(), peer_ip);
                                    //     if let Err(error) = peer.outbound_socket.send(Message::BlockResponse(Data::Object(block))).await {
                                    //         warn!("[BlockResponse] {}", error);
                                    //         break;
                                    //     }
                                    // }

                                    // Stop the clock on internal RTT.
                                    #[cfg(any(feature = "test", feature = "prometheus"))]
                                    metrics::histogram!(metrics::internal_rtt::BLOCK_REQUEST, rtt_start.elapsed());
                                },
                                Message::BlockResponse(block) => {
                                    #[cfg(any(feature = "test", feature = "prometheus"))]
                                    metrics::increment_counter!(metrics::message_counts::BLOCK_RESPONSE);

                                    // // Perform the deferred non-blocking deserialization of the block.
                                    // match block.deserialize().await {
                                    //     Ok(block) => {
                                    //         // TODO (howardwu): TEMPORARY - Remove this after testnet2.
                                    //         // Sanity check for a V12 ledger.
                                    //         if N::ID == 3
                                    //             && block.height() > snarkvm::dpc::testnet2::V12_UPGRADE_BLOCK_HEIGHT
                                    //             && block.header().proof().is_hiding()
                                    //         {
                                    //             warn!("Peer {} is not V12-compliant, proceeding to disconnect", peer_ip);
                                    //             break;
                                    //         }
                                    //
                                    //         // Route the `BlockResponse` to the ledger.
                                    //         if let Err(error) = state.ledger().router().send(LedgerRequest::BlockResponse(peer_ip, block)).await {
                                    //             warn!("[BlockResponse] {}", error);
                                    //         }
                                    //     },
                                    //     // Route the `Failure` to the ledger.
                                    //     Err(error) => if let Err(error) = state.ledger().router().send(LedgerRequest::Failure(peer_ip, format!("{}", error))).await {
                                    //         warn!("[Failure] {}", error);
                                    //     }
                                    // }
                                }
                                Message::ChallengeRequest(..) | Message::ChallengeResponse(..) => {
                                    // Peer is not following the protocol.
                                    warn!("Peer {} is not following the protocol", peer_ip);
                                    break;
                                },
                                Message::Disconnect(reason) => {
                                    #[cfg(any(feature = "test", feature = "prometheus"))]
                                    metrics::increment_counter!(metrics::message_counts::DISCONNECT);

                                    debug!("Peer {} disconnected for the following reason: {:?}", peer_ip, reason);
                                    break;
                                },
                                Message::PeerRequest => {
                                    #[cfg(any(feature = "test", feature = "prometheus"))]
                                    metrics::increment_counter!(metrics::message_counts::PEER_REQUEST);

                                    // Unfortunately can't be feature-flagged because of the enum
                                    // it's passed around in.
                                    let _rtt_start_instant: Option<Instant> = None;

                                    #[cfg(any(feature = "test", feature = "prometheus"))]
                                    let _rtt_start_instant = Some(rtt_start);

                                    // Send a `PeerResponse` message.
                                    if let Err(error) = peers_router.send(PeersRequest::SendPeerResponse(peer_ip, _rtt_start_instant)).await {
                                        warn!("[PeerRequest] {}", error);
                                    }
                                }
                                Message::PeerResponse(peer_ips, _) => {
                                    #[cfg(any(feature = "test", feature = "prometheus"))]
                                    metrics::increment_counter!(metrics::message_counts::PEER_RESPONSE);

                                    // Adds the given peer IPs to the list of candidate peers.
                                    if let Err(error) = peers_router.send(PeersRequest::ReceivePeerResponse(peer_ips)).await {
                                        warn!("[PeerResponse] {}", error);
                                    }
                                }
                                Message::Ping(version, fork_depth, node_type, status) => {
                                    #[cfg(any(feature = "test", feature = "prometheus"))]
                                    metrics::increment_counter!(metrics::message_counts::PING);

                                    // Ensure the message protocol version is not outdated.
                                    if version < E::MESSAGE_VERSION {
                                        warn!("Dropping {} on version {} (outdated)", peer_ip, version);
                                        break;
                                    }
                                    // Ensure the maximum fork depth is correct.
                                    if fork_depth != ALEO_MAXIMUM_FORK_DEPTH {
                                        warn!("Dropping {} for an incorrect maximum fork depth of {}", peer_ip, fork_depth);
                                        break;
                                    }
                                    // // Perform the deferred non-blocking deserialization of the block header.
                                    // match block_header.deserialize().await {
                                    //     Ok(block_header) => {
                                    //         // If this node is not a sync node and is syncing, the peer is a sync node, and this node is ahead, proceed to disconnect.
                                    //         if E::NODE_TYPE != NodeType::Beacon
                                    //             && E::status().is_syncing()
                                    //             && node_type == NodeType::Beacon
                                    //             && state.ledger().reader().latest_cumulative_weight() > block_header.cumulative_weight()
                                    //         {
                                    //             trace!("Disconnecting from {} (ahead of sync node)", peer_ip);
                                    //             break;
                                    //         }
                                    //
                                    //         // TODO (howardwu): TEMPORARY - Remove this after testnet2.
                                    //         // Sanity check for a V12 ledger.
                                    //         if N::ID == 3
                                    //             && block_header.height() > snarkvm::dpc::testnet2::V12_UPGRADE_BLOCK_HEIGHT
                                    //             && block_header.proof().is_hiding()
                                    //         {
                                    //             warn!("Peer {} is not V12-compliant, proceeding to disconnect", peer_ip);
                                    //             break;
                                    //         }
                                    //
                                    //         // Update peer's block height.
                                    //         peer.block_height = block_header.height();
                                    //     }
                                    //     Err(error) => warn!("[Ping] {}", error),
                                    // }

                                    // Update the version of the peer.
                                    peer.version = version;
                                    // Update the node type of the peer.
                                    peer.node_type = node_type;
                                    // Update the status of the peer.
                                    peer.status = status;

                                    // // Determine if the peer is on a fork (or unknown).
                                    // let is_fork = match state.ledger().reader().get_block_hash(peer.block_height) {
                                    //     Ok(expected_block_hash) => Some(expected_block_hash != block_hash),
                                    //     Err(_) => None,
                                    // };

                                    // Stop the clock on internal RTT.
                                    #[cfg(any(feature = "test", feature = "prometheus"))]
                                    metrics::histogram!(metrics::internal_rtt::PING, rtt_start.elapsed());

                                    // // Send a `Pong` message to the peer.
                                    // if let Err(error) = peer.send(Message::Pong(is_fork, Data::Object(state.ledger().reader().latest_block_locators()))).await {
                                    //     warn!("[Pong] {}", error);
                                    // }
                                },
                                Message::Pong(is_fork) => {
                                    #[cfg(any(feature = "test", feature = "prometheus"))]
                                    metrics::increment_counter!(metrics::message_counts::PONG);

                                    // Unfortunately can't be feature-flagged because of the enum
                                    // it's passed around in.
                                    let _rtt_start_instant: Option<Instant> = None;

                                    #[cfg(any(feature = "test", feature = "prometheus"))]
                                    let _rtt_start_instant = Some(rtt_start);

                                    // // Perform the deferred non-blocking deserialization of block locators.
                                    // let request = match block_locators.deserialize().await {
                                    //     // Route the `Pong` to the ledger.
                                    //     Ok(block_locators) => LedgerRequest::Pong(peer_ip, peer.node_type, peer.status, is_fork, block_locators, _rtt_start_instant),
                                    //     // Route the `Failure` to the ledger.
                                    //     Err(error) => LedgerRequest::Failure(peer_ip, format!("{}", error)),
                                    // };
                                    //
                                    // // Route the request to the ledger.
                                    // if let Err(error) = state.ledger().router().send(request).await {
                                    //     warn!("[Pong] {}", error);
                                    // }
                                    //
                                    // // Spawn an asynchronous task for the `Ping` request.
                                    // let peers_router = peers_router.clone();
                                    // let ledger_reader = state.ledger().reader().clone();
                                    // // Procure a resource id to register the task with, as it might be terminated at any point in time.
                                    // let ping_resource_id = E::resources().procure_id();
                                    // E::resources().register_task(Some(ping_resource_id), task::spawn(async move {
                                    //     // Sleep for the preset time before sending a `Ping` request.
                                    //     tokio::time::sleep(Duration::from_secs(E::PING_SLEEP_IN_SECS)).await;
                                    //
                                    //     // Retrieve the latest ledger state.
                                    //     let latest_block_hash = ledger_reader.latest_block_hash();
                                    //     let latest_block_header = ledger_reader.latest_block_header();
                                    //
                                    //     // Send a `Ping` request to the peer.
                                    //     let message = Message::Ping(E::MESSAGE_VERSION, N::ALEO_MAXIMUM_FORK_DEPTH, E::NODE_TYPE, E::status().get(), latest_block_hash, Data::Object(latest_block_header));
                                    //     if let Err(error) = peers_router.send(PeersRequest::MessageSend(peer_ip, message)).await {
                                    //         warn!("[Ping] {}", error);
                                    //     }
                                    //
                                    //     E::resources().deregister(ping_resource_id);
                                    // }));
                                }
                                Message::UnconfirmedBlock(block_height, block_hash, block) => {
                                    #[cfg(any(feature = "test", feature = "prometheus"))]
                                    metrics::increment_counter!(metrics::message_counts::UNCONFIRMED_BLOCK);

                                    // Drop the peer, if they have sent more than 5 unconfirmed blocks in the last 5 seconds.
                                    let frequency = peer.seen_inbound_blocks.values().filter(|t| t.elapsed().unwrap().as_secs() <= 5).count();
                                    if frequency >= 10 {
                                        warn!("Dropping {} for spamming unconfirmed blocks (frequency = {})", peer_ip, frequency);
                                        // Send a `PeerRestricted` message.
                                        if let Err(error) = peers_router.send(PeersRequest::PeerRestricted(peer_ip)).await {
                                            warn!("[PeerRestricted] {}", error);
                                        }
                                        break;
                                    }

                                    // Retrieve the last seen timestamp of the received block.
                                    let last_seen = peer.seen_inbound_blocks.entry(block_hash).or_insert(SystemTime::UNIX_EPOCH);
                                    let is_router_ready = last_seen.elapsed().unwrap().as_secs() > E::RADIO_SILENCE_IN_SECS;

                                    // Update the timestamp for the received block.
                                    peer.seen_inbound_blocks.insert(block_hash, SystemTime::now());

                                    // // Ensure the unconfirmed block is at least within 2 blocks of the latest block height,
                                    // // and no more that 2 blocks ahead of the latest block height.
                                    // // If it is stale, skip the routing of this unconfirmed block to the ledger.
                                    // let latest_block_height = state.ledger().reader().latest_block_height();
                                    // let lower_bound = latest_block_height.saturating_sub(2);
                                    // let upper_bound = latest_block_height.saturating_add(2);
                                    // let is_within_range = block_height >= lower_bound && block_height <= upper_bound;
                                    //
                                    // // Ensure the node is not peering.
                                    // let is_node_ready = !E::status().is_peering();
                                    //
                                    // // If this node is a beacon or sync node, skip this message, after updating the timestamp.
                                    // if E::NODE_TYPE == NodeType::Beacon || E::NODE_TYPE == NodeType::Beacon || !is_router_ready || !is_within_range || !is_node_ready {
                                    //     trace!("Skipping 'UnconfirmedBlock {}' from {}", block_height, peer_ip)
                                    // } else {
                                    //     // Perform the deferred non-blocking deserialization of the block.
                                    //     let request = match block.deserialize().await {
                                    //         // Ensure the claimed block height and block hash matches in the deserialized block.
                                    //         Ok(block) => match block_height == block.height() && block_hash == block.hash() {
                                    //             // Route the `UnconfirmedBlock` to the ledger.
                                    //             true => LedgerRequest::UnconfirmedBlock(peer_ip, block),
                                    //             // Route the `Failure` to the ledger.
                                    //             false => LedgerRequest::Failure(peer_ip, "Malformed UnconfirmedBlock message".to_string())
                                    //         },
                                    //         // Route the `Failure` to the ledger.
                                    //         Err(error) => LedgerRequest::Failure(peer_ip, format!("{}", error)),
                                    //     };
                                    //
                                    //     // Route the request to the ledger.
                                    //     if let Err(error) = state.ledger().router().send(request).await {
                                    //         warn!("[UnconfirmedBlock] {}", error);
                                    //     }
                                    // }
                                }
                                Message::UnconfirmedTransaction(transaction) => {
                                    #[cfg(any(feature = "test", feature = "prometheus"))]
                                    metrics::increment_counter!(metrics::message_counts::UNCONFIRMED_TRANSACTION);

                                    // Drop the peer, if they have sent more than 500 unconfirmed transactions in the last 5 seconds.
                                    let frequency = peer.seen_inbound_transactions.values().filter(|t| t.elapsed().unwrap().as_secs() <= 5).count();
                                    if frequency >= 500 {
                                        warn!("Dropping {} for spamming unconfirmed transactions (frequency = {})", peer_ip, frequency);
                                        // Send a `PeerRestricted` message.
                                        if let Err(error) = peers_router.send(PeersRequest::PeerRestricted(peer_ip)).await {
                                            warn!("[PeerRestricted] {}", error);
                                        }
                                        break;
                                    }

                                    // Perform the deferred non-blocking deserialisation of the
                                    // transaction.
                                    match transaction.deserialize().await {
                                        Ok(transaction) => {
                                            // // Retrieve the last seen timestamp of the received transaction.
                                            // let last_seen = peer.seen_inbound_transactions.entry(transaction.id()).or_insert(SystemTime::UNIX_EPOCH);
                                            // let is_router_ready = last_seen.elapsed().unwrap().as_secs() > E::RADIO_SILENCE_IN_SECS;
                                            //
                                            // // Update the timestamp for the received transaction.
                                            // peer.seen_inbound_transactions.insert(transaction.id(), SystemTime::now());
                                            //
                                            // // Ensure the node is not peering.
                                            // let is_node_ready = !E::status().is_peering();
                                            //
                                            // // If this node is a beacon or sync node, skip this message, after updating the timestamp.
                                            // if E::NODE_TYPE == NodeType::Beacon || E::NODE_TYPE == NodeType::Beacon || !is_router_ready || !is_node_ready {
                                            //     trace!("Skipping 'UnconfirmedTransaction {}' from {}", transaction.id(), peer_ip);
                                            // } else {
                                            //     // // Route the `UnconfirmedTransaction` to the prover.
                                            //     // if let Err(error) = state.prover().router().send(ProverRequest::UnconfirmedTransaction(peer_ip, transaction)).await {
                                            //     //     warn!("[UnconfirmedTransaction] {}", error);
                                            //     //
                                            //     // }
                                            // }
                                        }
                                        Err(error) => warn!("[UnconfirmedTransaction] {}", error)
                                    }
                                }
                            }
                        }
                        // An error occurred.
                        Some(Err(error)) => error!("Failed to read message from {}: {}", peer_ip, error),
                        // The stream has been disconnected.
                        None => break,
                    },
                }
            }

            // // When this is reached, it means the peer has disconnected.
            // // Route a `Disconnect` to the ledger.
            // if let Err(error) = state.ledger().router()
            //     .send(LedgerRequest::Disconnect(peer_ip, DisconnectReason::PeerHasDisconnected))
            //     .await
            // {
            //     warn!("[Peer::Disconnect] {}", error);
            // }

            E::resources().deregister(peer_resource_id);
        }));
    }
}

/// Shorthand for the parent half of the `Peers` message channel.
pub type PeersRouter<N> = mpsc::Sender<PeersRequest<N>>;
/// Shorthand for the child half of the `Peers` message channel.
pub type PeersHandler<N> = mpsc::Receiver<PeersRequest<N>>;

/// Shorthand for the parent half of the connection result channel.
pub(crate) type ConnectionResult = oneshot::Sender<Result<()>>;

///
/// An enum of requests that the `Peers` struct processes.
///
#[derive(Debug)]
pub enum PeersRequest<N: Network> {
    /// Connect := (peer_ip, connection_result)
    Connect(SocketAddr, ConnectionResult),
    /// Heartbeat
    Heartbeat,
    /// MessagePropagate := (peer_ip, message)
    MessagePropagate(SocketAddr, Message<N>),
    /// MessageSend := (peer_ip, message)
    MessageSend(SocketAddr, Message<N>),
    /// PeerConnecting := (stream, peer_ip)
    PeerConnecting(TcpStream, SocketAddr),
    /// PeerConnected := (peer_ip, outbound_router)
    PeerConnected(SocketAddr, OutboundRouter<N>),
    /// PeerDisconnected := (peer_ip)
    PeerDisconnected(SocketAddr),
    /// PeerRestricted := (peer_ip)
    PeerRestricted(SocketAddr),
    /// SendPeerResponse := (peer_ip, rtt_start)
    /// Note: rtt_start is for the request/response cycle for sharing peers.
    SendPeerResponse(SocketAddr, Option<Instant>),
    /// ReceivePeerResponse := (\[peer_ip\])
    ReceivePeerResponse(Vec<SocketAddr>),
}

///
/// A list of peers connected to the node.
///
pub struct Peers<N: Network, E: Environment> {
    /// The state of the node.
    state: State<N, E>,
    /// The peers router of the node.
    peers_router: PeersRouter<N>,
    /// The map connected peer IPs to their outbound message router.
    connected_peers: RwLock<HashMap<SocketAddr, OutboundRouter<N>>>,
    /// The set of candidate peer IPs.
    candidate_peers: RwLock<HashSet<SocketAddr>>,
    /// The set of restricted peer IPs.
    restricted_peers: RwLock<HashMap<SocketAddr, Instant>>,
    /// The map of peers to their first-seen port number, number of attempts, and timestamp of the last inbound connection request.
    seen_inbound_connections: RwLock<HashMap<SocketAddr, ((u16, u32), SystemTime)>>,
    /// The map of peers to the timestamp of their last outbound connection request.
    seen_outbound_connections: RwLock<HashMap<SocketAddr, SystemTime>>,
}

impl<N: Network, E: Environment> Peers<N, E> {
    ///
    /// Initializes a new instance of `Peers` and its corresponding handler.
    ///
    pub async fn new(state: State<N, E>) -> (Self, mpsc::Receiver<PeersRequest<N>>) {
        // Initialize an MPSC channel for sending requests to the `Peers` struct.
        let (peers_router, peers_handler) = mpsc::channel(1024);

        // Initialize the peers.
        let peers = Self {
            state,
            peers_router,
            connected_peers: Default::default(),
            candidate_peers: Default::default(),
            restricted_peers: Default::default(),
            seen_inbound_connections: Default::default(),
            seen_outbound_connections: Default::default(),
        };

        (peers, peers_handler)
    }

    ///
    /// Returns the peers router.
    ///
    pub fn router(&self) -> &PeersRouter<N> {
        &self.peers_router
    }

    ///
    /// Returns `true` if the node is connected to the given IP.
    ///
    pub async fn is_connected_to(&self, ip: SocketAddr) -> bool {
        self.connected_peers.read().await.contains_key(&ip)
    }

    ///
    /// Returns `true` if the given IP is restricted.
    ///
    pub async fn is_restricted(&self, ip: SocketAddr) -> bool {
        match self.restricted_peers.read().await.get(&ip) {
            Some(timestamp) => timestamp.elapsed().as_secs() < E::RADIO_SILENCE_IN_SECS,
            None => false,
        }
    }

    ///
    /// Returns the list of connected peers.
    ///
    pub async fn connected_peers(&self) -> Vec<SocketAddr> {
        self.connected_peers.read().await.keys().copied().collect()
    }

    ///
    /// Returns the list of candidate peers.
    ///
    pub async fn candidate_peers(&self) -> HashSet<SocketAddr> {
        self.candidate_peers.read().await.clone()
    }

    ///
    /// Returns the set of connected beacon nodes.
    ///
    pub async fn connected_beacon_nodes(&self) -> HashSet<SocketAddr> {
        let beacon_nodes = E::beacon_nodes();
        self.connected_peers
            .read()
            .await
            .keys()
            .filter(|addr| beacon_nodes.contains(addr))
            .copied()
            .collect()
    }

    ///
    /// Returns the number of connected beacon nodes.
    ///
    pub async fn number_of_connected_beacon_nodes(&self) -> usize {
        let beacon_nodes = E::beacon_nodes();
        self.connected_peers
            .read()
            .await
            .keys()
            .filter(|addr| beacon_nodes.contains(addr))
            .count()
    }

    ///
    /// Returns the number of connected peers.
    ///
    pub async fn number_of_connected_peers(&self) -> usize {
        self.connected_peers.read().await.len()
    }

    ///
    /// Returns the number of candidate peers.
    ///
    pub async fn number_of_candidate_peers(&self) -> usize {
        self.candidate_peers.read().await.len()
    }

    ///
    /// Returns the number of restricted peers.
    ///
    pub async fn number_of_restricted_peers(&self) -> usize {
        self.restricted_peers.read().await.len()
    }

    ///
    /// Performs the given `request` to the peers.
    /// All requests must go through this `update`, so that a unified view is preserved.
    ///
    pub(crate) async fn update(&self, request: PeersRequest<N>) {
        match request {
            PeersRequest::Connect(peer_ip, connection_result) => {
                // Ensure the peer IP is not this node.
                if self.state.is_local_ip(&peer_ip) {
                    debug!("Skipping connection request to {} (attempted to self-connect)", peer_ip);
                }
                // Ensure the node does not surpass the maximum number of peer connections.
                else if self.number_of_connected_peers().await >= E::MAXIMUM_NUMBER_OF_PEERS {
                    debug!("Skipping connection request to {} (maximum peers reached)", peer_ip);
                }
                // Ensure the peer is a new connection.
                else if self.is_connected_to(peer_ip).await {
                    debug!("Skipping connection request to {} (already connected)", peer_ip);
                }
                // Ensure the peer is not restricted.
                else if self.is_restricted(peer_ip).await {
                    debug!("Skipping connection request to {} (restricted)", peer_ip);
                }
                // Attempt to open a TCP stream.
                else {
                    // Lock seen_outbound_connections for further processing.
                    let mut seen_outbound_connections = self.seen_outbound_connections.write().await;

                    // Ensure the node respects the connection frequency limit.
                    let last_seen = seen_outbound_connections.entry(peer_ip).or_insert(SystemTime::UNIX_EPOCH);
                    let elapsed = last_seen.elapsed().unwrap_or(Duration::MAX).as_secs();
                    if elapsed < E::RADIO_SILENCE_IN_SECS {
                        trace!("Skipping connection request to {} (tried {} secs ago)", peer_ip, elapsed);
                    } else {
                        debug!("Connecting to {}...", peer_ip);
                        // Update the last seen timestamp for this peer.
                        seen_outbound_connections.insert(peer_ip, SystemTime::now());

                        // Release the lock over seen_outbound_connections.
                        drop(seen_outbound_connections);

                        // Initialize the peer handler.
                        match timeout(Duration::from_millis(E::CONNECTION_TIMEOUT_IN_MILLIS), TcpStream::connect(peer_ip)).await {
                            Ok(stream) => match stream {
                                Ok(stream) => Peer::handler(self.state.clone(), stream, Some(connection_result)).await,
                                Err(error) => {
                                    trace!("Failed to connect to '{}': '{:?}'", peer_ip, error);
                                    self.candidate_peers.write().await.remove(&peer_ip);
                                }
                            },
                            Err(error) => {
                                error!("Unable to reach '{}': '{:?}'", peer_ip, error);
                                self.candidate_peers.write().await.remove(&peer_ip);
                            }
                        };
                    }
                }
            }
            PeersRequest::Heartbeat => {
                // Obtain the number of connected peers.
                let number_of_connected_peers = self.number_of_connected_peers().await;
                // Ensure the number of connected peers is below the maximum threshold.
                if number_of_connected_peers > E::MAXIMUM_NUMBER_OF_PEERS {
                    debug!("Exceeded maximum number of connected peers");

                    // Determine the peers to disconnect from.
                    let num_excess_peers = number_of_connected_peers.saturating_sub(E::MAXIMUM_NUMBER_OF_PEERS);
                    let peer_ips_to_disconnect = self
                        .connected_peers
                        .read()
                        .await
                        .iter()
                        .filter(|(peer_ip, _)| !E::beacon_nodes().contains(peer_ip) && !E::trusted_nodes().contains(peer_ip))
                        .take(num_excess_peers)
                        .map(|(&peer_ip, _)| peer_ip)
                        .collect::<Vec<SocketAddr>>();

                    // Proceed to send disconnect requests to these peers.
                    for peer_ip in peer_ips_to_disconnect {
                        info!("Disconnecting from {} (exceeded maximum connections)", peer_ip);
                        self.send(peer_ip, Message::Disconnect(DisconnectReason::TooManyPeers)).await;
                        // Add an entry for this `Peer` in the restricted peers.
                        self.restricted_peers.write().await.insert(peer_ip, Instant::now());
                    }
                }

                // TODO (howardwu): This logic can be optimized and unified with the context around it.
                // Determine if the node is connected to more sync nodes than expected.
                let connected_beacon_nodes = self.connected_beacon_nodes().await;
                let number_of_connected_beacon_nodes = connected_beacon_nodes.len();
                let num_excess_beacon_nodes = number_of_connected_beacon_nodes.saturating_sub(1);
                if num_excess_beacon_nodes > 0 {
                    debug!("Exceeded maximum number of sync nodes");

                    // Proceed to send disconnect requests to these peers.
                    for peer_ip in connected_beacon_nodes
                        .iter()
                        .copied()
                        .choose_multiple(&mut OsRng::default(), num_excess_beacon_nodes)
                    {
                        info!("Disconnecting from {} (exceeded maximum connections)", peer_ip);
                        self.send(peer_ip, Message::Disconnect(DisconnectReason::TooManyPeers)).await;
                        // Add an entry for this `Peer` in the restricted peers.
                        self.restricted_peers.write().await.insert(peer_ip, Instant::now());
                    }
                }

                // Ensure that the trusted nodes are connected.
                if !E::trusted_nodes().is_empty() {
                    let connected_peers = self.connected_peers().await.into_iter().collect::<HashSet<_>>();
                    let trusted_nodes = E::trusted_nodes();
                    let disconnected_trusted_nodes = trusted_nodes.difference(&connected_peers).copied();
                    for peer_ip in disconnected_trusted_nodes {
                        // Initialize the connection process.
                        let (router, handler) = oneshot::channel();
                        let request = PeersRequest::Connect(peer_ip, router);
                        if let Err(error) = self.peers_router.send(request).await {
                            warn!("Failed to transmit the request: '{}'", error);
                        }

                        // Do not wait for the result of each connection.
                        // Procure a resource id to register the task with, as it might be terminated at any point in time.
                        let resource_id = E::resources().procure_id();
                        E::resources().register_task(
                            Some(resource_id),
                            task::spawn(async move {
                                let _ = handler.await;

                                E::resources().deregister(resource_id);
                            }),
                        );
                    }
                }

                // Skip if the number of connected peers is above the minimum threshold.
                match number_of_connected_peers < E::MINIMUM_NUMBER_OF_PEERS {
                    true => {
                        if number_of_connected_peers > 0 {
                            trace!("Sending requests for more peer connections");
                            // Request more peers if the number of connected peers is below the threshold.
                            for peer_ip in self.connected_peers().await.iter().choose_multiple(&mut OsRng::default(), 3) {
                                self.send(*peer_ip, Message::PeerRequest).await;
                            }
                        }
                    }
                    false => return,
                };

                // Add the sync nodes to the list of candidate peers.
                if number_of_connected_beacon_nodes == 0 {
                    self.add_candidate_peers(E::beacon_nodes().iter()).await;
                }

                // Attempt to connect to more peers if the number of connected peers is below the minimum threshold.
                // Select the peers randomly from the list of candidate peers.
                let midpoint_number_of_peers = E::MINIMUM_NUMBER_OF_PEERS.saturating_add(E::MAXIMUM_NUMBER_OF_PEERS) / 2;
                for peer_ip in self
                    .candidate_peers()
                    .await
                    .iter()
                    .copied()
                    .choose_multiple(&mut OsRng::default(), midpoint_number_of_peers)
                {
                    // Ensure this node is not connected to more than the permitted number of sync nodes.
                    if E::beacon_nodes().contains(&peer_ip) && number_of_connected_beacon_nodes >= 1 {
                        continue;
                    }

                    if !self.is_connected_to(peer_ip).await {
                        trace!("Attempting connection to {}...", peer_ip);

                        // Initialize the connection process.
                        let (router, handler) = oneshot::channel();
                        let request = PeersRequest::Connect(peer_ip, router);
                        if let Err(error) = self.peers_router.send(request).await {
                            warn!("Failed to transmit the request: '{}'", error);
                        }
                        // Do not wait for the result of each connection.
                        // Procure a resource id to register the task with, as it might be terminated at any point in time.
                        let resource_id = E::resources().procure_id();
                        E::resources().register_task(
                            Some(resource_id),
                            task::spawn(async move {
                                let _ = handler.await;

                                E::resources().deregister(resource_id);
                            }),
                        );
                    }
                }
            }
            PeersRequest::MessagePropagate(sender, message) => self.propagate(sender, message).await,
            PeersRequest::MessageSend(sender, message) => self.send(sender, message).await,
            PeersRequest::PeerConnecting(stream, peer_ip) => {
                // Ensure the peer IP is not this node.
                if self.state.is_local_ip(&peer_ip) {
                    debug!("Skipping connection request to {} (attempted to self-connect)", peer_ip);
                }
                // Ensure the node does not surpass the maximum number of peer connections.
                else if self.number_of_connected_peers().await >= E::MAXIMUM_NUMBER_OF_PEERS {
                    debug!("Dropping connection request from {} (maximum peers reached)", peer_ip);
                }
                // Ensure the node is not already connected to this peer.
                else if self.is_connected_to(peer_ip).await {
                    debug!("Dropping connection request from {} (already connected)", peer_ip);
                }
                // Ensure the peer is not restricted.
                else if self.is_restricted(peer_ip).await {
                    debug!("Dropping connection request from {} (restricted)", peer_ip);
                }
                // Spawn a handler to be run asynchronously.
                else {
                    // Sanitize the port from the peer, if it is a remote IP address.
                    let (peer_lookup, peer_port) = match peer_ip.ip().is_loopback() {
                        // Loopback case - Do not sanitize, merely pass through.
                        true => (peer_ip, peer_ip.port()),
                        // Remote case - Sanitize, storing u16::MAX for the peer IP address to dedup the peer next time.
                        false => (SocketAddr::new(peer_ip.ip(), u16::MAX), peer_ip.port()),
                    };

                    // Lock seen_inbound_connections for further processing.
                    let mut seen_inbound_connections = self.seen_inbound_connections.write().await;

                    // Fetch the inbound tracker entry for this peer.
                    let ((initial_port, num_attempts), last_seen) = seen_inbound_connections
                        .entry(peer_lookup)
                        .or_insert(((peer_port, 0), SystemTime::UNIX_EPOCH));
                    let elapsed = last_seen.elapsed().unwrap_or(Duration::MAX).as_secs();

                    // Reset the inbound tracker entry for this peer, if the predefined elapsed time has passed.
                    if elapsed > E::RADIO_SILENCE_IN_SECS {
                        // Reset the initial port for this peer.
                        *initial_port = peer_port;
                        // Reset the number of attempts for this peer.
                        *num_attempts = 0;
                        // Reset the last seen timestamp for this peer.
                        *last_seen = SystemTime::now();
                    }

                    // Ensure the connecting peer has not surpassed the connection attempt limit.
                    if *num_attempts > E::MAXIMUM_CONNECTION_FAILURES {
                        trace!("Dropping connection request from {} (tried {} secs ago)", peer_ip, elapsed);
                        // Add an entry for this `Peer` in the restricted peers.
                        self.restricted_peers.write().await.insert(peer_ip, Instant::now());
                    } else {
                        debug!("Received a connection request from {}", peer_ip);
                        // Update the number of attempts for this peer.
                        *num_attempts += 1;

                        // Release the lock over seen_inbound_connections.
                        drop(seen_inbound_connections);

                        // Initialize the peer handler.
                        Peer::handler(self.state.clone(), stream, None).await;
                    }
                }
            }
            PeersRequest::PeerConnected(peer_ip, outbound) => {
                // Add an entry for this `Peer` in the connected peers.
                self.connected_peers.write().await.insert(peer_ip, outbound);
                // Remove an entry for this `Peer` in the candidate peers, if it exists.
                self.candidate_peers.write().await.remove(&peer_ip);

                #[cfg(any(feature = "test", feature = "prometheus"))]
                {
                    let number_of_connected_peers = self.number_of_connected_peers().await;
                    let number_of_candidate_peers = self.number_of_candidate_peers().await;
                    metrics::gauge!(metrics::peers::CONNECTED, number_of_connected_peers as f64);
                    metrics::gauge!(metrics::peers::CANDIDATE, number_of_candidate_peers as f64);
                }
            }
            PeersRequest::PeerDisconnected(peer_ip) => {
                // Remove an entry for this `Peer` in the connected peers, if it exists.
                self.connected_peers.write().await.remove(&peer_ip);
                // Add an entry for this `Peer` in the candidate peers.
                self.candidate_peers.write().await.insert(peer_ip);

                #[cfg(any(feature = "test", feature = "prometheus"))]
                {
                    let number_of_connected_peers = self.number_of_connected_peers().await;
                    let number_of_candidate_peers = self.number_of_candidate_peers().await;
                    metrics::gauge!(metrics::peers::CONNECTED, number_of_connected_peers as f64);
                    metrics::gauge!(metrics::peers::CANDIDATE, number_of_candidate_peers as f64);
                }
            }
            PeersRequest::PeerRestricted(peer_ip) => {
                // Remove an entry for this `Peer` in the connected peers, if it exists.
                self.connected_peers.write().await.remove(&peer_ip);
                // Add an entry for this `Peer` in the restricted peers.
                self.restricted_peers.write().await.insert(peer_ip, Instant::now());

                #[cfg(any(feature = "test", feature = "prometheus"))]
                {
                    let number_of_connected_peers = self.number_of_connected_peers().await;
                    let number_of_restricted_peers = self.number_of_restricted_peers().await;
                    metrics::gauge!(metrics::peers::CONNECTED, number_of_connected_peers as f64);
                    metrics::gauge!(metrics::peers::RESTRICTED, number_of_restricted_peers as f64);
                }
            }
            PeersRequest::SendPeerResponse(recipient, rtt_start) => {
                // Send a `PeerResponse` message.
                let connected_peers = self.connected_peers().await;
                self.send(recipient, Message::PeerResponse(connected_peers, rtt_start)).await;
            }
            PeersRequest::ReceivePeerResponse(peer_ips) => {
                self.add_candidate_peers(peer_ips.iter()).await;

                #[cfg(any(feature = "test", feature = "prometheus"))]
                {
                    let number_of_candidate_peers = self.number_of_candidate_peers().await;
                    metrics::gauge!(metrics::peers::CANDIDATE, number_of_candidate_peers as f64);
                }
            }
        }
    }

    /// Adds the given peer IPs to the set of candidate peers.
    ///
    /// This method skips adding any given peers if the combined size exceeds the threshold,
    /// as the peer providing this list could be subverting the protocol.
    async fn add_candidate_peers<'a, T: ExactSizeIterator<Item = &'a SocketAddr> + IntoIterator>(&self, peers: T) {
        // Acquire the candidate peers write lock.
        let mut candidate_peers = self.candidate_peers.write().await;
        // Ensure the combined number of peers does not surpass the threshold.
        for peer_ip in peers.take(E::MAXIMUM_CANDIDATE_PEERS.saturating_sub(candidate_peers.len())) {
            // Ensure the peer is not itself and is a new candidate peer.
            if !self.state.is_local_ip(peer_ip) && !self.is_connected_to(*peer_ip).await {
                // Proceed to insert each new candidate peer IP.
                candidate_peers.insert(*peer_ip);
            }
        }
    }

    /// Sends the given message to specified peer.
    async fn send(&self, peer: SocketAddr, message: Message<N>) {
        let target_peer = self.connected_peers.read().await.get(&peer).cloned();
        match target_peer {
            Some(outbound) => {
                if let Err(error) = outbound.send(message).await {
                    trace!("Outbound channel failed: {}", error);
                    self.connected_peers.write().await.remove(&peer);

                    #[cfg(any(feature = "test", feature = "prometheus"))]
                    {
                        let number_of_connected_peers = self.number_of_connected_peers().await;
                        metrics::gauge!(metrics::peers::CONNECTED, number_of_connected_peers as f64);
                    }
                }
            }
            None => warn!("Attempted to send to a non-connected peer {}", peer),
        }
    }

    /// Sends the given message to every connected peer, excluding the sender.
    async fn propagate(&self, sender: SocketAddr, mut message: Message<N>) {
        // Perform ahead-of-time, non-blocking serialization just once for applicable objects.
        if let Message::UnconfirmedBlock(_, _, ref mut data) = message {
            let serialized_block = Data::serialize(data.clone()).await.expect("Block serialization is bugged");
            let _ = std::mem::replace(data, Data::Buffer(serialized_block));
        }

        // Iterate through all peers that are not the sender or a beacon node.
        for peer in self
            .connected_peers()
            .await
            .iter()
            .filter(|peer_ip| *peer_ip != &sender && !E::beacon_nodes().contains(peer_ip))
        {
            self.send(*peer, message.clone()).await;
        }
    }

    /// Removes the addresses of all known peers.
    #[cfg(feature = "test")]
    pub async fn reset_known_peers(&self) {
        self.candidate_peers.write().await.clear();
        self.restricted_peers.write().await.clear();
        self.seen_inbound_connections.write().await.clear();
        self.seen_outbound_connections.write().await.clear();
    }
}

#[derive(Clone)]
pub struct State<N: Network, E: Environment> {
    /// The local IP of the node.
    local_ip: Arc<SocketAddr>,
    /// The Aleo account of the node.
    account: Arc<Account<N>>,
    /// The list of peers for the node.
    peers: Arc<OnceBox<Peers<N, E>>>,
}

impl<N: Network, E: Environment> State<N, E> {
    /// Initializes a new `State` instance.
    pub async fn new(node_ip: SocketAddr, account: Account<N>) -> Result<Self> {
        // Initialize a new TCP listener at the given IP.
        let (local_ip, listener) = match TcpListener::bind(node_ip).await {
            Ok(listener) => (listener.local_addr().expect("Failed to fetch the local IP"), listener),
            Err(error) => panic!("Failed to bind listener: {:?}. Check if another Aleo node is running", error),
        };

        // Construct the state.
        let state = Self {
            local_ip: Arc::new(local_ip),
            account: Arc::new(account),
            peers: Default::default(),
        };

        // Initialize a new peers module.
        let (peers, peers_handler) = Peers::new(state.clone()).await;
        // Set the peers into state.
        state
            .peers
            .set(peers.into())
            .map_err(|_| anyhow!("Failed to set peers into state"))?;
        // Initialize the peers.
        state.initialize_peers(peers_handler).await;

        // Initialize the listener.
        state.initialize_listener(listener).await;
        // Initialize the heartbeat.
        state.initialize_heartbeat().await;

        Ok(state)
    }

    /// Returns the IP address of this node.
    pub fn local_ip(&self) -> &SocketAddr {
        &self.local_ip
    }

    /// Returns the Aleo address of this node.
    pub fn address(&self) -> &Address<N> {
        self.account.address()
    }

    /// Returns the peers module of this node.
    pub fn peers(&self) -> &Peers<N, E> {
        &self.peers.get().unwrap()
    }

    /// Returns `true` if the given IP is this node.
    pub fn is_local_ip(&self, ip: &SocketAddr) -> bool {
        *ip == *self.local_ip || (ip.ip().is_unspecified() || ip.ip().is_loopback()) && ip.port() == self.local_ip.port()
    }
}

impl<N: Network, E: Environment> State<N, E> {
    ///
    /// Initialize the connection listener for new peers.
    ///
    async fn initialize_peers(&self, mut peers_handler: PeersHandler<N>) {
        let state = self.clone();
        spawn_task!({
            // Asynchronously wait for a peers request.
            while let Some(request) = peers_handler.recv().await {
                let state = state.clone();
                // Procure a resource ID for the task, as it may terminate at any time.
                let resource_id = E::resources().procure_id();
                // Asynchronously process a peers request.
                E::resources().register_task(
                    Some(resource_id),
                    tokio::spawn(async move {
                        // Update the state of the peers.
                        state.peers().update(request).await;

                        E::resources().deregister(resource_id);
                    }),
                );
            }
        });
    }

    ///
    /// Initialize the connection listener for new peers.
    ///
    async fn initialize_listener(&self, listener: TcpListener) {
        let state = self.clone();
        spawn_task!({
            info!("Listening for peers at {}", state.local_ip);
            loop {
                // Don't accept connections if the node is breaching the configured peer limit.
                if state.peers().number_of_connected_peers().await < E::MAXIMUM_NUMBER_OF_PEERS {
                    // Asynchronously wait for an inbound TcpStream.
                    match listener.accept().await {
                        // Process the inbound connection request.
                        Ok((stream, peer_ip)) => {
                            let request = PeersRequest::PeerConnecting(stream, peer_ip);
                            if let Err(error) = state.peers().router().send(request).await {
                                error!("Failed to send request to peers: {}", error)
                            }
                        }
                        Err(error) => error!("Failed to accept a connection: {}", error),
                    }
                    // Add a small delay to prevent overloading the network from handshakes.
                    tokio::time::sleep(Duration::from_millis(150)).await;
                } else {
                    // Add a sleep delay as the node has reached peer capacity.
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        });
    }

    ///
    /// Initialize a new instance of the heartbeat.
    ///
    async fn initialize_heartbeat(&self) {
        let state = self.clone();
        spawn_task!({
            loop {
                // // Transmit a heartbeat request to the ledger.
                // if let Err(error) = state.ledger().router().send(LedgerRequest::Heartbeat).await {
                //     error!("Failed to send heartbeat to ledger: {}", error)
                // }
                // Transmit a heartbeat request to the peers.
                if let Err(error) = state.peers().router().send(PeersRequest::Heartbeat).await {
                    error!("Failed to send heartbeat to peers: {}", error)
                }
                // Sleep for `E::HEARTBEAT_IN_SECS` seconds.
                tokio::time::sleep(Duration::from_secs(E::HEARTBEAT_IN_SECS)).await;
            }
        });
    }
}

#[derive(Clone)]
pub struct Node<N: Network, E: Environment> {
    /// The current state of the node.
    state: State<N, E>,
}

impl<N: Network, E: Environment> Node<N, E> {
    /// Initializes a new instance of the node.
    pub async fn new(node_ip: SocketAddr, account: Account<N>) -> Result<Self> {
        // Initialize the state.
        let state = State::new(node_ip, account).await?;

        let node = Self { state };

        // /// Returns the storage path of the ledger.
        // pub(crate) fn ledger_storage_path(&self, _local_ip: SocketAddr) -> PathBuf {
        //     if cfg!(feature = "test") {
        //         // Tests may use any available ports, and removes the storage artifacts afterwards,
        //         // so that there is no need to adhere to a specific number assignment logic.
        //         PathBuf::from(format!("/tmp/snarkos-test-ledger-{}", _local_ip.port()))
        //     } else {
        //         aleo_std::aleo_ledger_dir(self.network, self.dev)
        //     }
        // }
        //
        // /// Returns the storage path of the validator.
        // pub(crate) fn validator_storage_path(&self, _local_ip: SocketAddr) -> PathBuf {
        //     if cfg!(feature = "test") {
        //         // Tests may use any available ports, and removes the storage artifacts afterwards,
        //         // so that there is no need to adhere to a specific number assignment logic.
        //         PathBuf::from(format!("/tmp/snarkos-test-validator-{}", _local_ip.port()))
        //     } else {
        //         // TODO (howardwu): Rename to validator.
        //         aleo_std::aleo_operator_dir(self.network, self.dev)
        //     }
        // }
        //
        // /// Returns the storage path of the prover.
        // pub(crate) fn prover_storage_path(&self, _local_ip: SocketAddr) -> PathBuf {
        //     if cfg!(feature = "test") {
        //         // Tests may use any available ports, and removes the storage artifacts afterwards,
        //         // so that there is no need to adhere to a specific number assignment logic.
        //         PathBuf::from(format!("/tmp/snarkos-test-prover-{}", _local_ip.port()))
        //     } else {
        //         aleo_std::aleo_prover_dir(self.network, self.dev)
        //     }
        // }
        //
        // // Initialize the ledger storage path.
        // let ledger_storage_path = node.ledger_storage_path(local_ip);
        // // Initialize the prover storage path.
        // let prover_storage_path = node.prover_storage_path(local_ip);
        // // Initialize the validator storage path.
        // let validator_storage_path = node.validator_storage_path(local_ip);

        // // Initialize a new instance for managing the ledger.
        // let (ledger, ledger_handler) = Ledger::<N, E>::open::<_>(&ledger_storage_path, state.clone()).await?;
        //
        // // Initialize a new instance for managing the prover.
        // let (prover, prover_handler) = Prover::open::<_>(&prover_storage_path, state.clone()).await?;
        //
        // // Initialize a new instance for managing the validator.
        // let (validator, validator_handler) = Operator::open::<_>(&validator_storage_path, state.clone()).await?;

        // // Initialise the metrics exporter.
        // #[cfg(any(feature = "test", feature = "prometheus"))]
        // Self::initialize_metrics(ledger.reader().clone());

        // node.state.initialize_ledger(ledger, ledger_handler).await;
        // node.state.initialize_prover(prover, prover_handler).await;
        // node.state.initialize_validator(validator, validator_handler).await;

        // node.state.validator().initialize().await;

        // node.initialize_notification(address).await;
        // node.initialize_rpc(node, address).await;

        Ok(node)
    }

    /// Returns the IP address of this node.
    pub fn local_ip(&self) -> &SocketAddr {
        self.state.local_ip()
    }

    /// Returns the Aleo address of this node.
    pub fn address(&self) -> &Address<N> {
        self.state.address()
    }

    /// Returns the peers module of this node.
    pub fn peers(&self) -> &Peers<N, E> {
        self.state.peers()
    }

    ///
    /// Sends a connection request to the given IP address.
    ///
    pub async fn connect_to(&self, peer_ip: SocketAddr) -> Result<()> {
        // Initialize the connection process.
        let (router, handler) = oneshot::channel();

        // Route a `Connect` request to the peer manager.
        self.peers().router().send(PeersRequest::Connect(peer_ip, router)).await?;

        // Wait until the connection task is initialized.
        handler.await.map(|_| ()).map_err(|e| e.into())
    }

    #[inline]
    pub async fn disconnect_from(&self, _peer_ip: SocketAddr, _reason: DisconnectReason) {
        // self.state.ledger().disconnect(peer_ip, reason).await
        // TODO (raychu86): Handle the disconnect case.
        unimplemented!()
    }

    ///
    /// Initialize a new instance of the RPC node.
    ///
    #[cfg(feature = "rpc")]
    async fn initialize_rpc(&self, cli: &CLI, address: Option<Address<N>>) {
        if !cli.norpc {
            // Initialize a new instance of the RPC node.
            let rpc_context = RpcContext::new(cli.rpc_username.clone(), cli.rpc_password.clone(), address, self.state.clone());
            let (rpc_node_addr, rpc_node_handle) = initialize_rpc_node::<N, E>(cli.rpc, rpc_context).await;

            debug!("JSON-RPC node listening on {}", rpc_node_addr);

            // Register the task; no need to provide an id, as it will run indefinitely.
            E::resources().register_task(None, rpc_node_handle);
        }
    }

    // #[cfg(any(feature = "test", feature = "prometheus"))]
    // fn initialize_metrics(ledger: LedgerReader<N>) {
    //     #[cfg(not(feature = "test"))]
    //     if let Some(handler) = snarkos_metrics::initialize() {
    //         // No need to provide an id, as the task will run indefinitely.
    //         E::resources().register_task(None, handler);
    //     }
    //
    //     // Set the block height as it could already be non-zero.
    //     metrics::gauge!(metrics::blocks::HEIGHT, ledger.latest_block_height() as f64);
    // }

    ///
    /// Disconnects from peers and proceeds to shut down the node.
    ///
    pub async fn shut_down(&self) {
        info!("Shutting down...");
        // Update the node status.
        E::status().update(Status::ShuttingDown);

        // Shut down the ledger.
        trace!("Proceeding to shut down the ledger...");
        // self.state.ledger().shut_down().await;

        // Flush the tasks.
        E::resources().shut_down();
        trace!("Node has shut down.");
    }
}
