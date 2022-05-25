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

use crate::{
    helpers::{NodeType, State, Status},
    network::{
        ConnectionResult,
        DisconnectReason,
        LedgerReader,
        LedgerRequest,
        LedgerRouter,
        Message,
        OperatorRequest,
        OperatorRouter,
        PeersRequest,
        PeersRouter,
        ProverRequest,
        ProverRouter,
    },
    Data,
    Environment,
};
use snarkvm::dpc::prelude::*;

use anyhow::{anyhow, bail, Result};
use futures::SinkExt;
use rayon::iter::ParallelIterator;
use std::{
    collections::HashMap,
    net::SocketAddr,
    time::{Duration, Instant, SystemTime},
};
use tokio::{net::TcpStream, sync::mpsc, task, time::timeout};
use tokio_stream::StreamExt;
use tokio_util::codec::Framed;

/// Shorthand for the parent half of the `Peer` outbound message channel.
pub(crate) type OutboundRouter<N, E> = mpsc::Sender<Message<N, E>>;
/// Shorthand for the child half of the `Peer` outbound message channel.
type OutboundHandler<N, E> = mpsc::Receiver<Message<N, E>>;

///
/// The state for each connected client.
///
pub(crate) struct Peer<N: Network, E: Environment> {
    /// The IP address of the peer, with the port set to the listener port.
    listener_ip: SocketAddr,
    /// The message version of the peer.
    version: u32,
    /// The node type of the peer.
    node_type: NodeType,
    /// The node type of the peer.
    status: Status,
    /// The block header of the peer.
    block_header: BlockHeader<N>,
    /// The timestamp of the last message received from this peer.
    last_seen: Instant,
    /// The TCP socket that handles sending and receiving data with this peer.
    outbound_socket: Framed<TcpStream, Message<N, E>>,
    /// The `outbound_handler` half of the MPSC message channel, used to receive messages from peers.
    /// When a message is received on this `OutboundHandler`, it will be written to the socket.
    outbound_handler: OutboundHandler<N, E>,
    /// The map of block hashes to their last seen timestamp.
    seen_inbound_blocks: HashMap<N::BlockHash, SystemTime>,
    /// The map of transaction IDs to their last seen timestamp.
    seen_inbound_transactions: HashMap<N::TransactionID, SystemTime>,
    /// The map of peers to a map of block hashes to their last seen timestamp.
    seen_outbound_blocks: HashMap<N::BlockHash, SystemTime>,
    /// The map of peers to a map of transaction IDs to their last seen timestamp.
    seen_outbound_transactions: HashMap<N::TransactionID, SystemTime>,
}

impl<N: Network, E: Environment> Peer<N, E> {
    /// Create a new instance of `Peer`.
    async fn new(
        stream: TcpStream,
        local_ip: SocketAddr,
        local_nonce: u64,
        peers_router: &PeersRouter<N, E>,
        ledger_reader: &LedgerReader<N>,
        connected_nonces: &[u64],
    ) -> Result<Self> {
        // Construct the socket.
        let mut outbound_socket = Framed::new(stream, Message::<N, E>::PeerRequest);

        // Perform the handshake before proceeding.
        let (peer_ip, peer_nonce, node_type, status) = Peer::handshake(
            &mut outbound_socket,
            local_ip,
            local_nonce,
            ledger_reader.latest_cumulative_weight(),
            connected_nonces,
        )
        .await?;

        // Send the first `Ping` message to the peer.
        let message = Message::Ping(
            E::MESSAGE_VERSION,
            N::ALEO_MAXIMUM_FORK_DEPTH,
            E::NODE_TYPE,
            E::status().get(),
            ledger_reader.latest_block_hash(),
            Data::Object(ledger_reader.latest_block_header()),
        );
        trace!("Sending '{}' to {}", message.name(), peer_ip);
        outbound_socket.send(message).await?;

        // Create a channel for this peer.
        let (outbound_router, outbound_handler) = mpsc::channel(1024);

        // Add an entry for this `Peer` in the connected peers.
        peers_router
            .send(PeersRequest::PeerConnected(peer_ip, peer_nonce, outbound_router))
            .await?;

        Ok(Peer {
            listener_ip: peer_ip,
            version: 0,
            node_type,
            status,
            block_header: N::genesis_block().header().clone(),
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
    async fn send(&mut self, message: Message<N, E>) -> Result<()> {
        trace!("Sending '{}' to {}", message.name(), self.peer_ip());
        self.outbound_socket.send(message).await?;
        Ok(())
    }

    /// Performs the handshake protocol, returning the listener IP and nonce of the peer upon success.
    async fn handshake(
        outbound_socket: &mut Framed<TcpStream, Message<N, E>>,
        local_ip: SocketAddr,
        local_nonce: u64,
        local_cumulative_weight: u128,
        connected_nonces: &[u64],
    ) -> Result<(SocketAddr, u64, NodeType, Status)> {
        // Get the IP address of the peer.
        let mut peer_ip = outbound_socket.get_ref().peer_addr()?;

        // Retrieve the genesis block header.
        let genesis_header = N::genesis_block().header();

        // Send a challenge request to the peer.
        let message = Message::<N, E>::ChallengeRequest(
            E::MESSAGE_VERSION,
            N::ALEO_MAXIMUM_FORK_DEPTH,
            E::NODE_TYPE,
            E::status().get(),
            local_ip.port(),
            local_nonce,
            local_cumulative_weight,
        );
        trace!("Sending '{}-A' to {}", message.name(), peer_ip);
        outbound_socket.send(message).await?;

        // Wait for the counterparty challenge request to come in.
        let (peer_nonce, node_type, status) = match outbound_socket.next().await {
            Some(Ok(message)) => {
                // Process the message.
                trace!("Received '{}-B' from {}", message.name(), peer_ip);
                match message {
                    Message::ChallengeRequest(
                        version,
                        fork_depth,
                        node_type,
                        peer_status,
                        listener_port,
                        peer_nonce,
                        peer_cumulative_weight,
                    ) => {
                        // Ensure the message protocol version is not outdated.
                        if version < E::MESSAGE_VERSION {
                            warn!("Dropping {} on version {} (outdated)", peer_ip, version);

                            // Send the disconnect message.
                            let message = Message::Disconnect(DisconnectReason::OutdatedClientVersion);
                            outbound_socket.send(message).await?;

                            bail!("Dropping {} on version {} (outdated)", peer_ip, version);
                        }
                        // Ensure the maximum fork depth is correct.
                        if fork_depth != N::ALEO_MAXIMUM_FORK_DEPTH {
                            // Send the disconnect message.
                            let message = Message::Disconnect(DisconnectReason::InvalidForkDepth);
                            outbound_socket.send(message).await?;

                            bail!("Dropping {} for an incorrect maximum fork depth of {}", peer_ip, fork_depth);
                        }
                        // If this node is not a sync node and is syncing, the peer is a sync node, and this node is ahead, proceed to disconnect.
                        if E::NODE_TYPE != NodeType::Sync
                            && E::status().is_syncing()
                            && node_type == NodeType::Sync
                            && local_cumulative_weight > peer_cumulative_weight
                        {
                            // Send the disconnect message.
                            let message = Message::Disconnect(DisconnectReason::YouNeedToSyncFirst);
                            outbound_socket.send(message).await?;

                            bail!("Dropping {} as this node is ahead", peer_ip);
                        }
                        // If this node is a sync node, the peer is not a sync node and is syncing, and the peer is ahead, proceed to disconnect.
                        if E::NODE_TYPE == NodeType::Sync
                            && node_type != NodeType::Sync
                            && peer_status == State::Syncing
                            && peer_cumulative_weight > local_cumulative_weight
                        {
                            // Send the disconnect message.
                            let message = Message::Disconnect(DisconnectReason::INeedToSyncFirst);
                            outbound_socket.send(message).await?;

                            bail!("Dropping {} as this node is ahead", peer_ip);
                        }
                        // Ensure the peer is not this node.
                        if local_nonce == peer_nonce {
                            bail!("Attempted to connect to self (nonce = {})", peer_nonce);
                        }
                        // Ensure the peer is not already connected to this node.
                        if connected_nonces.contains(&peer_nonce) {
                            bail!("Already connected to a peer with nonce {}", peer_nonce);
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

                        // Initialize a status variable.
                        let status = Status::new();
                        status.update(peer_status);

                        (peer_nonce, node_type, status)
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
                        match &block_header == genesis_header {
                            true => Ok((peer_ip, peer_nonce, node_type, status)),
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
    #[allow(clippy::too_many_arguments)]
    pub(super) async fn handler(
        stream: TcpStream,
        local_ip: SocketAddr,
        local_nonce: u64,
        peers_router: &PeersRouter<N, E>,
        ledger_reader: LedgerReader<N>,
        ledger_router: LedgerRouter<N>,
        prover_router: ProverRouter<N>,
        operator_router: OperatorRouter<N>,
        connected_nonces: Vec<u64>,
        connection_result: Option<ConnectionResult>,
    ) {
        let peers_router = peers_router.clone();

        E::tasks().append(task::spawn(async move {
            // Register our peer with state which internally sets up some channels.
            let mut peer = match Peer::new(
                stream,
                local_ip,
                local_nonce,
                &peers_router,
                &ledger_reader,
                &connected_nonces,
            )
                .await
            {
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
                                Message::Ping(_, _, _, _, _, ref mut data) => {
                                    // Perform non-blocking serialisation of the block header.
                                    let serialized_header = Data::serialize(data.clone()).await.expect("Block header serialization is bugged");
                                    let _ = std::mem::replace(data, Data::Buffer(serialized_header));

                                    true
                                }
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
                                        .entry(transaction.transaction_id())
                                        .or_insert(SystemTime::UNIX_EPOCH);
                                    let is_ready_to_send = last_seen.elapsed().unwrap().as_secs() > E::RADIO_SILENCE_IN_SECS;

                                    // Update the timestamp for the peer and sent transaction.
                                    peer.seen_outbound_transactions.insert(transaction.transaction_id(), SystemTime::now());
                                    // Report the unconfirmed block height.
                                    if is_ready_to_send {
                                        trace!(
                                            "Preparing to send 'UnconfirmedTransaction {}' to {}",
                                            transaction.transaction_id(),
                                            peer_ip
                                        );
                                    }

                                    // Perform non-blocking serialization of the transaction.
                                    let serialized_transaction = Data::serialize(data.clone()).await.expect("Transaction serialization is bugged");
                                    let _ = std::mem::replace(data, Data::Buffer(serialized_transaction));

                                    is_ready_to_send
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
                            // Process the message.
                            trace!("Received '{}' from {}", message.name(), peer_ip);
                            match message {
                                Message::BlockRequest(start_block_height, end_block_height) => {
                                    // Ensure the request is within the accepted limits.
                                    let number_of_blocks = end_block_height.saturating_sub(start_block_height);
                                    if number_of_blocks > E::MAXIMUM_BLOCK_REQUEST {
                                        // Route a `Failure` to the ledger.
                                        let failure = format!("Attempted to request {} blocks", number_of_blocks);
                                        if let Err(error) = ledger_router.send(LedgerRequest::Failure(peer_ip, failure)).await {
                                            warn!("[Failure] {}", error);
                                        }
                                        continue;
                                    }
                                    // Retrieve the requested blocks.
                                    let blocks: Vec<Block<N>> = match ledger_reader.get_blocks(start_block_height, end_block_height).and_then(|blocks| blocks.collect()) {
                                        Ok(blocks) => blocks,
                                        Err(error) => {
                                            // Route a `Failure` to the ledger.
                                            if let Err(error) = ledger_router.send(LedgerRequest::Failure(peer_ip, format!("{}", error))).await {
                                                warn!("[Failure] {}", error);
                                            }
                                            continue;
                                        }
                                    };
                                    // Send a `BlockResponse` message for each block to the peer.
                                    for block in blocks {
                                        debug!("Sending 'BlockResponse {}' to {}", block.height(), peer_ip);
                                        if let Err(error) = peer.outbound_socket.send(Message::BlockResponse(Data::Object(block))).await {
                                            warn!("[BlockResponse] {}", error);
                                            break;
                                        }
                                    }
                                },
                                Message::BlockResponse(block) => {
                                    // Perform the deferred non-blocking deserialization of the block.
                                    match block.deserialize().await {
                                        Ok(block) => {
                                            // TODO (howardwu): TEMPORARY - Remove this after testnet2.
                                            // Sanity check for a V12 ledger.
                                            if N::NETWORK_ID == 2
                                                && block.height() > snarkvm::dpc::testnet2::V12_UPGRADE_BLOCK_HEIGHT
                                                && block.header().proof().is_hiding()
                                            {
                                                warn!("Peer {} is not V12-compliant, proceeding to disconnect", peer_ip);
                                                break;
                                            }

                                            // Route the `BlockResponse` to the ledger.
                                            if let Err(error) = ledger_router.send(LedgerRequest::BlockResponse(peer_ip, block, prover_router.clone())).await {
                                                warn!("[BlockResponse] {}", error);
                                            }
                                        },
                                        // Route the `Failure` to the ledger.
                                        Err(error) => if let Err(error) = ledger_router.send(LedgerRequest::Failure(peer_ip, format!("{}", error))).await {
                                            warn!("[Failure] {}", error);
                                        }
                                    }
                                }
                                Message::ChallengeRequest(..) | Message::ChallengeResponse(..) => {
                                    // Peer is not following the protocol.
                                    warn!("Peer {} is not following the protocol", peer_ip);
                                    break;
                                },
                                Message::Disconnect(reason) => {
                                    debug!("Peer {} disconnected for the following reason: {:?}", peer_ip, reason);
                                    break;
                                },
                                Message::PeerRequest => {
                                    // Send a `PeerResponse` message.
                                    if let Err(error) = peers_router.send(PeersRequest::SendPeerResponse(peer_ip)).await {
                                        warn!("[PeerRequest] {}", error);
                                    }
                                }
                                Message::PeerResponse(peer_ips) => {
                                    // Adds the given peer IPs to the list of candidate peers.
                                    if let Err(error) = peers_router.send(PeersRequest::ReceivePeerResponse(peer_ips)).await {
                                        warn!("[PeerResponse] {}", error);
                                    }
                                }
                                Message::Ping(version, fork_depth, node_type, status, block_hash, block_header) => {
                                    // Ensure the message protocol version is not outdated.
                                    if version < E::MESSAGE_VERSION {
                                        warn!("Dropping {} on version {} (outdated)", peer_ip, version);
                                        break;
                                    }
                                    // Ensure the maximum fork depth is correct.
                                    if fork_depth != N::ALEO_MAXIMUM_FORK_DEPTH {
                                        warn!("Dropping {} for an incorrect maximum fork depth of {}", peer_ip, fork_depth);
                                        break;
                                    }
                                    // Perform the deferred non-blocking deserialization of the block header.
                                    match block_header.deserialize().await {
                                        Ok(block_header) => {
                                            // If this node is not a sync node and is syncing, the peer is a sync node, and this node is ahead, proceed to disconnect.
                                            if E::NODE_TYPE != NodeType::Sync
                                                && E::status().is_syncing()
                                                && node_type == NodeType::Sync
                                                && ledger_reader.latest_cumulative_weight() > block_header.cumulative_weight()
                                            {
                                                trace!("Disconnecting from {} (ahead of sync node)", peer_ip);
                                                break;
                                            }

                                            // TODO (howardwu): TEMPORARY - Remove this after testnet2.
                                            // Sanity check for a V12 ledger.
                                            if N::NETWORK_ID == 2
                                                && block_header.height() > snarkvm::dpc::testnet2::V12_UPGRADE_BLOCK_HEIGHT
                                                && block_header.proof().is_hiding()
                                            {
                                                warn!("Peer {} is not V12-compliant, proceeding to disconnect", peer_ip);
                                                break;
                                            }

                                            // Update the block header of the peer.
                                            peer.block_header = block_header;
                                        }
                                        Err(error) => warn!("[Ping] {}", error),
                                    }

                                    // Update the version of the peer.
                                    peer.version = version;
                                    // Update the node type of the peer.
                                    peer.node_type = node_type;
                                    // Update the status of the peer.
                                    peer.status.update(status);

                                    // Determine if the peer is on a fork (or unknown).
                                    let is_fork = match ledger_reader.get_block_hash(peer.block_header.height()) {
                                        Ok(expected_block_hash) => Some(expected_block_hash != block_hash),
                                        Err(_) => None,
                                    };
                                    // Send a `Pong` message to the peer.
                                    if let Err(error) = peer.send(Message::Pong(is_fork, Data::Object(ledger_reader.latest_block_locators()))).await {
                                        warn!("[Pong] {}", error);
                                    }
                                },
                                Message::Pong(is_fork, block_locators) => {
                                    // Perform the deferred non-blocking deserialization of block locators.
                                    let request = match block_locators.deserialize().await {
                                        // Route the `Pong` to the ledger.
                                        Ok(block_locators) => LedgerRequest::Pong(peer_ip, peer.node_type, peer.status.get(), is_fork, block_locators),
                                        // Route the `Failure` to the ledger.
                                        Err(error) => LedgerRequest::Failure(peer_ip, format!("{}", error)),
                                    };

                                    // Route the request to the ledger.
                                    if let Err(error) = ledger_router.send(request).await {
                                        warn!("[Pong] {}", error);
                                    }

                                    // Spawn an asynchronous task for the `Ping` request.
                                    let peers_router = peers_router.clone();
                                    let ledger_reader = ledger_reader.clone();
                                    E::tasks().append(task::spawn(async move {
                                        // Sleep for the preset time before sending a `Ping` request.
                                        tokio::time::sleep(Duration::from_secs(E::PING_SLEEP_IN_SECS)).await;

                                        // Retrieve the latest ledger state.
                                        let latest_block_hash = ledger_reader.latest_block_hash();
                                        let latest_block_header = ledger_reader.latest_block_header();

                                        // Send a `Ping` request to the peer.
                                        let message = Message::Ping(E::MESSAGE_VERSION, N::ALEO_MAXIMUM_FORK_DEPTH, E::NODE_TYPE, E::status().get(), latest_block_hash, Data::Object(latest_block_header));
                                        if let Err(error) = peers_router.send(PeersRequest::MessageSend(peer_ip, message)).await {
                                            warn!("[Ping] {}", error);
                                        }
                                    }));
                                }
                                Message::UnconfirmedBlock(block_height, block_hash, block) => {
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

                                    // Ensure the unconfirmed block is at least within 2 blocks of the latest block height,
                                    // and no more that 2 blocks ahead of the latest block height.
                                    // If it is stale, skip the routing of this unconfirmed block to the ledger.
                                    let latest_block_height = ledger_reader.latest_block_height();
                                    let lower_bound = latest_block_height.saturating_sub(2);
                                    let upper_bound = latest_block_height.saturating_add(2);
                                    let is_within_range = block_height >= lower_bound && block_height <= upper_bound;

                                    // Ensure the node is not peering.
                                    let is_node_ready = !E::status().is_peering();

                                    // If this node is a beacon or sync node, skip this message, after updating the timestamp.
                                    if E::NODE_TYPE == NodeType::Beacon || E::NODE_TYPE == NodeType::Sync || !is_router_ready || !is_within_range || !is_node_ready {
                                        trace!("Skipping 'UnconfirmedBlock {}' from {}", block_height, peer_ip)
                                    } else {
                                        // Perform the deferred non-blocking deserialization of the block.
                                        let request = match block.deserialize().await {
                                            // Ensure the claimed block height and block hash matches in the deserialized block.
                                            Ok(block) => match block_height == block.height() && block_hash == block.hash() {
                                                // Route the `UnconfirmedBlock` to the ledger.
                                                true => LedgerRequest::UnconfirmedBlock(peer_ip, block, prover_router.clone()),
                                                // Route the `Failure` to the ledger.
                                                false => LedgerRequest::Failure(peer_ip, "Malformed UnconfirmedBlock message".to_string())
                                            },
                                            // Route the `Failure` to the ledger.
                                            Err(error) => LedgerRequest::Failure(peer_ip, format!("{}", error)),
                                        };

                                        // Route the request to the ledger.
                                        if let Err(error) = ledger_router.send(request).await {
                                            warn!("[UnconfirmedBlock] {}", error);
                                        }
                                    }
                                }
                                Message::UnconfirmedTransaction(transaction) => {
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
                                            // Retrieve the last seen timestamp of the received transaction.
                                            let last_seen = peer.seen_inbound_transactions.entry(transaction.transaction_id()).or_insert(SystemTime::UNIX_EPOCH);
                                            let is_router_ready = last_seen.elapsed().unwrap().as_secs() > E::RADIO_SILENCE_IN_SECS;

                                            // Update the timestamp for the received transaction.
                                            peer.seen_inbound_transactions.insert(transaction.transaction_id(), SystemTime::now());

                                            // Ensure the node is not peering.
                                            let is_node_ready = !E::status().is_peering();

                                            // If this node is a beacon or sync node, skip this message, after updating the timestamp.
                                            if E::NODE_TYPE == NodeType::Beacon || E::NODE_TYPE == NodeType::Sync || !is_router_ready || !is_node_ready {
                                                trace!("Skipping 'UnconfirmedTransaction {}' from {}", transaction.transaction_id(), peer_ip);
                                            } else {
                                                // Route the `UnconfirmedTransaction` to the prover.
                                                if let Err(error) = prover_router.send(ProverRequest::UnconfirmedTransaction(peer_ip, transaction)).await {
                                                    warn!("[UnconfirmedTransaction] {}", error);

                                                }
                                            }

                                        }
                                        Err(error) => warn!("[UnconfirmedTransaction] {}", error)
                                    }
                                }
                                Message::PoolRegister(address) => {
                                    if E::NODE_TYPE != NodeType::Operator {
                                        trace!("Skipping 'PoolRegister' from {}", peer_ip);
                                    } else if let Err(error) = operator_router.send(OperatorRequest::PoolRegister(peer_ip, address)).await {
                                        warn!("[PoolRegister] {}", error);
                                    }
                                }
                                Message::PoolRequest(share_difficulty, block_template) => {
                                    if E::NODE_TYPE != NodeType::Prover {
                                        trace!("Skipping 'PoolRequest' from {}", peer_ip);
                                    } else if let Ok(block_template) = block_template.deserialize().await {
                                        if let Err(error) = prover_router.send(ProverRequest::PoolRequest(peer_ip, share_difficulty, block_template)).await {
                                            warn!("[PoolRequest] {}", error);
                                        }
                                    } else {
                                        warn!("[PoolRequest] could not deserialize block template");
                                    }
                                }
                                Message::PoolResponse(address, nonce, proof) => {
                                    if E::NODE_TYPE != NodeType::Operator {
                                        trace!("Skipping 'PoolResponse' from {}", peer_ip);
                                    } else if let Ok(proof) = proof.deserialize().await {
                                        if let Err(error) = operator_router.send(OperatorRequest::PoolResponse(peer_ip, address, nonce, proof)).await {
                                            warn!("[PoolResponse] {}", error);
                                        }
                                    } else {
                                        warn!("[PoolResponse] could not deserialize proof");
                                    }
                                }
                                Message::Unused(_) => break, // Peer is not following the protocol.
                            }
                        }
                        // An error occurred.
                        Some(Err(error)) => error!("Failed to read message from {}: {}", peer_ip, error),
                        // The stream has been disconnected.
                        None => break,
                    },
                }
            }

            // When this is reached, it means the peer has disconnected.
            // Route a `Disconnect` to the ledger.
            if let Err(error) = ledger_router
                .send(LedgerRequest::Disconnect(peer_ip, DisconnectReason::PeerHasDisconnected))
                .await
            {
                warn!("[Peer::Disconnect] {}", error);
            }
        }));
    }
}
