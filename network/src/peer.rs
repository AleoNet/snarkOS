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

use crate::{OperatorRequest, ProverRequest};
use snarkos_environment::{
    helpers::{NodeType, State, Status},
    network::{Data, DisconnectReason, Message},
    Environment,
};
use snarkvm::dpc::prelude::*;
use std::sync::Arc;

use crate::state::NetworkState;
use anyhow::{anyhow, bail, Result};
use futures::SinkExt;
use std::{
    collections::HashMap,
    net::SocketAddr,
    time::{Duration, Instant, SystemTime},
};

use std::sync::atomic::{AtomicU32, Ordering};
use tokio::{
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf},
        TcpStream,
    },
    sync::{mpsc, RwLock},
    task,
    time::timeout,
};
use tokio_stream::StreamExt;
use tokio_util::codec::{FramedRead, FramedWrite};

///
/// The state for each connected client.
///
#[derive(Debug)]
pub(crate) struct Peer<N: Network, E: Environment> {
    network_state: NetworkState<N, E>,
    /// The IP address of the peer, with the port set to the listener port.
    listener_ip: SocketAddr,
    /// The message version of the peer.
    version: AtomicU32,
    /// The node type of the peer.
    /// TODO: make atomic (analogous to Status).
    node_type: RwLock<NodeType>,
    /// The node type of the peer.
    status: Status,
    /// The block header of the peer.
    block_header: RwLock<BlockHeader<N>>,
    /// The timestamp of the last message received from this peer.
    /// TODO: make atomic?
    last_seen: RwLock<Instant>,
    /// The TCP socket that handles sending and receiving data with this peer.
    pub outbound_sender: mpsc::Sender<Message<N, E>>,
    /// The map of block hashes to their last seen timestamp.
    seen_inbound_blocks: RwLock<HashMap<N::BlockHash, SystemTime>>,
    /// The map of transaction IDs to their last seen timestamp.
    seen_inbound_transactions: RwLock<HashMap<N::TransactionID, SystemTime>>,
    /// The map of peers to a map of block hashes to their last seen timestamp.
    seen_outbound_blocks: RwLock<HashMap<N::BlockHash, SystemTime>>,
    /// The map of peers to a map of transaction IDs to their last seen timestamp.
    seen_outbound_transactions: RwLock<HashMap<N::TransactionID, SystemTime>>,
}

impl<N: Network, E: Environment> Peer<N, E> {
    /// Create a new instance of `Peer`.
    pub async fn new(
        network_state: NetworkState<N, E>,
        stream: TcpStream,
        local_ip: SocketAddr,
        local_nonce: u64,
        connected_nonces: &[u64],
    ) -> Result<Arc<Self>> {
        let (read_half, write_half) = stream.into_split();
        // Construct the socket.
        // TODO: rename.
        let mut inbound_socket = FramedRead::new(read_half, Message::<N, E>::PeerRequest);
        let mut outbound_socket = FramedWrite::new(write_half, Message::<N, E>::PeerRequest);

        let ledger_reader = network_state.ledger.reader();

        // Perform the handshake before proceeding.
        let (peer_ip, peer_nonce, node_type, status) = Peer::handshake(
            &mut inbound_socket,
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

        // TODO: check this is large enough, perhaps use crossbeam impl?
        let (outbound_sender, outbound_receiver) = mpsc::channel(1024);

        let peer = Arc::new(Peer {
            network_state,
            listener_ip: peer_ip,
            version: AtomicU32::new(0),
            node_type: RwLock::new(node_type),
            status,
            block_header: RwLock::new(N::genesis_block().header().clone()),
            last_seen: RwLock::new(Instant::now()),
            outbound_sender,
            seen_inbound_blocks: Default::default(),
            seen_inbound_transactions: Default::default(),
            seen_outbound_blocks: Default::default(),
            seen_outbound_transactions: Default::default(),
        });

        // Add an entry for this `Peer` in the connected peers.
        peer.network_state.peers.peer_connected(peer_ip, peer_nonce).await;

        // TODO: rename or split.
        peer.clone()
            .start_io_tasks(inbound_socket, outbound_socket, outbound_receiver)
            .await;

        Ok(peer)
    }

    /// Returns the IP address of the peer, with the port set to the listener port.
    /// TODO: change confusing naming
    fn peer_ip(&self) -> SocketAddr {
        self.listener_ip
    }

    /// Sends the given message to this peer.
    async fn send(self: Arc<Self>, mut message: Message<N, E>) -> Result<()> {
        // Disconnect if the peer has not communicated back within the predefined time.
        let elapsed = self.last_seen.read().await.elapsed();
        if elapsed > Duration::from_secs(E::RADIO_SILENCE_IN_SECS) {
            warn!("Peer {} has not communicated in {} seconds", self.peer_ip(), elapsed.as_secs());
            return Ok(());
        }

        // Ensure sufficient time has passed before needing to send the message.
        let is_ready_to_send = match &mut message {
            Message::Ping(_, _, _, _, _, ref mut data) => {
                // Perform non-blocking serialisation of the block header.
                let serialized_header = Data::serialize(data.clone()).await.expect("Block header serialization is bugged");
                let _ = std::mem::replace(data, Data::Buffer(serialized_header));

                true
            }
            Message::UnconfirmedBlock(block_height, block_hash, ref mut data) => {
                // Retrieve the last seen timestamp of this block for this self.
                let mut locked_seen_blocks = self.seen_outbound_blocks.write().await;
                let last_seen = locked_seen_blocks.entry(*block_hash).or_insert(SystemTime::UNIX_EPOCH);
                let is_ready_to_send = last_seen.elapsed().unwrap().as_secs() > E::RADIO_SILENCE_IN_SECS;

                // Update the timestamp for the self and sent block.
                locked_seen_blocks.insert(*block_hash, SystemTime::now());
                // Drop the write lock.
                drop(locked_seen_blocks);

                // Report the unconfirmed block height.
                //
                if is_ready_to_send {
                    trace!("Preparing to send 'UnconfirmedBlock {}' to {}", block_height, self.peer_ip());
                }

                // Perform non-blocking serialization of the block (if it hasn't been serialized yet).
                let serialized_block = Data::serialize(data.clone()).await.expect("Block serialization is bugged");
                let _ = std::mem::replace(data, Data::Buffer(serialized_block));

                is_ready_to_send
            }
            Message::UnconfirmedTransaction(ref mut data) => {
                let tx = if let Data::Object(tx) = data {
                    tx
                } else {
                    panic!("Logic error: the transaction shouldn't have been serialized yet.");
                };

                // Retrieve the last seen timestamp of this transaction for this peer.
                let mut locked_seen_txs = self.seen_outbound_transactions.write().await;
                let last_seen = locked_seen_txs.entry(tx.transaction_id()).or_insert(SystemTime::UNIX_EPOCH);
                let is_ready_to_send = last_seen.elapsed().unwrap().as_secs() > E::RADIO_SILENCE_IN_SECS;

                // Update the timestamp for the peer and sent transaction.
                locked_seen_txs.insert(tx.transaction_id(), SystemTime::now());
                // Drop the write lock.
                drop(locked_seen_txs);

                // Report the unconfirmed block height.
                if is_ready_to_send {
                    trace!(
                        "Preparing to send 'UnconfirmedTransaction {}' to {}",
                        tx.transaction_id(),
                        self.peer_ip()
                    );
                }

                // Perform non-blocking serialization of the transaction.
                let serialized_transaction = Data::serialize(data.clone()).await.expect("Transaction serialization is bugged");
                let _ = std::mem::replace(data, Data::Buffer(serialized_transaction));

                is_ready_to_send
            }

            _ => true,
        };

        // TODO: remove the need for this bool and short-circuit the function instead.
        if is_ready_to_send {
            trace!("Sending '{}' to {}", message.name(), self.peer_ip());
            self.outbound_sender.send(message).await?;
        }

        Ok(())
    }

    /// Performs the handshake protocol, returning the listener IP and nonce of the peer upon success.
    async fn handshake(
        inbound_socket: &mut FramedRead<OwnedReadHalf, Message<N, E>>,
        outbound_socket: &mut FramedWrite<OwnedWriteHalf, Message<N, E>>,
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
        let (peer_nonce, node_type, status) = match inbound_socket.next().await {
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
        match inbound_socket.next().await {
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
    pub(super) async fn start_io_tasks(
        self: Arc<Self>,
        mut inbound_socket: FramedRead<OwnedReadHalf, Message<N, E>>,
        mut outbound_socket: FramedWrite<OwnedWriteHalf, Message<N, E>>,
        mut outbound_receiver: mpsc::Receiver<Message<N, E>>,
    ) {
        let peer = self;

        // Start outbound task: procure a resource id to register the task with, as it might be
        // terminated at any point in time.
        let inbound_resource_id = E::resources().procure_id();
        E::resources().register_task(
            Some(inbound_resource_id),
            tokio::spawn(async move {
                while let Some(message) = outbound_receiver.recv().await {
                    if let Err(_error) = outbound_socket.send(message).await {
                        // TODO: handle error.
                    }
                }

                // Returning None from the outbound receiver indicates all the senders have been
                // dropped, time to clean up the task.
                E::resources().deregister(inbound_resource_id);
            }),
        );

        // Start inbound task: procure a resource id to register the task with, as it might be
        // terminated at any point in time.
        let outbound_resource_id = E::resources().procure_id();
        E::resources().register_task(
            Some(outbound_resource_id),
            task::spawn(async move {
                // Register our peer with state which internally sets up some channels.

                // Retrieve the peer IP.
                let peer_ip = peer.peer_ip();
                let ledger_reader = peer.network_state.ledger.reader();
                info!("Connected to {}", peer_ip);

                // Process incoming messages until this stream is disconnected.
                loop {
                    let peer = peer.clone();

                    match inbound_socket.next().await {
                        // Received a message from the peer.
                        Some(Ok(message)) => {
                            // Disconnect if the peer has not communicated back within the predefined time.
                            let elapsed = peer.last_seen.read().await.elapsed();
                            match elapsed > Duration::from_secs(E::RADIO_SILENCE_IN_SECS) {
                                true => {
                                    warn!("Failed to receive a message from {} in {} seconds", peer_ip, elapsed.as_secs());
                                    break;
                                }
                                false => {
                                    // Update the last seen timestamp.
                                    *peer.last_seen.write().await = Instant::now();
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

                                        peer.network_state.ledger.add_failure(peer_ip, failure).await;

                                        continue;
                                    }
                                    // Retrieve the requested blocks.
                                    let blocks = match ledger_reader.get_blocks(start_block_height, end_block_height) {
                                        Ok(blocks) => blocks,
                                        Err(error) => {
                                            // Route a `Failure` to the ledger.
                                            peer.network_state.ledger.add_failure(peer_ip, format!("{}", error)).await;

                                            continue;
                                        }
                                    };
                                    // Send a `BlockResponse` message for each block to the peer.
                                    for block in blocks {
                                        debug!("Sending 'BlockResponse {}' to {}", block.height(), peer_ip);
                                        if let Err(error) = peer.outbound_sender.send(Message::BlockResponse(Data::Object(block))).await {
                                            warn!("[BlockResponse] {}", error);
                                            break;
                                        }
                                    }
                                }
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
                                            peer.network_state.ledger.block_response(peer_ip, block).await;
                                        }
                                        // Route the `Failure` to the ledger.
                                        Err(error) => {
                                            peer.network_state.ledger.add_failure(peer_ip, format!("{}", error)).await;
                                        }
                                    }
                                }
                                Message::ChallengeRequest(..) | Message::ChallengeResponse(..) => {
                                    // Peer is not following the protocol.
                                    warn!("Peer {} is not following the protocol", peer_ip);
                                    break;
                                }
                                Message::Disconnect(reason) => {
                                    debug!("Peer {} disconnected for the following reason: {:?}", peer_ip, reason);
                                    break;
                                }
                                Message::PeerRequest => {
                                    // Send a `PeerResponse` message.
                                    peer.network_state.peers.send_peer_response(peer_ip).await;
                                }
                                Message::PeerResponse(peer_ips) => {
                                    // Adds the given peer IPs to the list of candidate peers.
                                    peer.network_state.peers.receive_peer_response(peer_ips).await
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
                                                && peer.network_state.ledger.reader().latest_cumulative_weight()
                                                    > block_header.cumulative_weight()
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
                                            *peer.block_header.write().await = block_header;
                                        }
                                        Err(error) => warn!("[Ping] {}", error),
                                    }

                                    // Update the version of the peer.
                                    peer.version.store(version, Ordering::SeqCst);
                                    // Update the node type of the peer.
                                    *peer.node_type.write().await = node_type;
                                    // Update the status of the peer.
                                    peer.status.update(status);

                                    // Determine if the peer is on a fork (or unknown).
                                    let is_fork = match peer
                                        .network_state
                                        .ledger
                                        .reader()
                                        .get_block_hash(peer.block_header.read().await.height())
                                    {
                                        Ok(expected_block_hash) => Some(expected_block_hash != block_hash),
                                        Err(_) => None,
                                    };
                                    // Send a `Pong` message to the peer.
                                    if let Err(error) = peer
                                        .clone()
                                        .send(Message::Pong(
                                            is_fork,
                                            Data::Object(peer.network_state.ledger.reader().latest_block_locators()),
                                        ))
                                        .await
                                    {
                                        warn!("[Pong] {}", error);
                                    }
                                }
                                Message::Pong(is_fork, block_locators) => {
                                    // Perform the deferred non-blocking deserialization of block locators.
                                    match block_locators.deserialize().await {
                                        // Route the `Pong` to the ledger.
                                        Ok(block_locators) => {
                                            peer.network_state
                                                .ledger
                                                .pong(peer_ip, *peer.node_type.read().await, peer.status.get(), is_fork, block_locators)
                                                .await
                                        }
                                        // Route the `Failure` to the ledger.
                                        Err(error) => peer.network_state.ledger.add_failure(peer_ip, format!("{}", error)).await,
                                    }

                                    // Spawn an asynchronous task for the `Ping` request.
                                    let ledger_reader = peer.network_state.ledger.reader();
                                    // Procure a resource id to register the task with, as it might be terminated at any point in time.
                                    let ping_resource_id = E::resources().procure_id();
                                    E::resources().register_task(
                                        Some(ping_resource_id),
                                        task::spawn(async move {
                                            // Sleep for the preset time before sending a `Ping` request.
                                            tokio::time::sleep(Duration::from_secs(E::PING_SLEEP_IN_SECS)).await;

                                            // Retrieve the latest ledger state.
                                            let latest_block_hash = ledger_reader.latest_block_hash();
                                            let latest_block_header = ledger_reader.latest_block_header();

                                            // Send a `Ping` request to the peer.
                                            let message = Message::Ping(
                                                E::MESSAGE_VERSION,
                                                N::ALEO_MAXIMUM_FORK_DEPTH,
                                                E::NODE_TYPE,
                                                E::status().get(),
                                                latest_block_hash,
                                                Data::Object(latest_block_header),
                                            );
                                            peer.network_state.peers.send(peer_ip, message).await;

                                            E::resources().deregister(ping_resource_id);
                                        }),
                                    );
                                }
                                Message::UnconfirmedBlock(block_height, block_hash, block) => {
                                    // Drop the peer, if they have sent more than 5 unconfirmed blocks in the last 5 seconds.
                                    let frequency = peer
                                        .seen_inbound_blocks
                                        .read()
                                        .await
                                        .values()
                                        .filter(|t| t.elapsed().unwrap().as_secs() <= 5)
                                        .count();
                                    if frequency >= 10 {
                                        warn!("Dropping {} for spamming unconfirmed blocks (frequency = {})", peer_ip, frequency);
                                        // Send a `PeerRestricted` message.
                                        peer.network_state.peers.peer_restricted(peer_ip).await;

                                        break;
                                    }

                                    // Retrieve the last seen timestamp of the received block.
                                    let mut locked_seen_blocks = peer.seen_inbound_blocks.write().await;
                                    let last_seen = locked_seen_blocks.entry(block_hash).or_insert(SystemTime::UNIX_EPOCH);
                                    let is_router_ready = last_seen.elapsed().unwrap().as_secs() > E::RADIO_SILENCE_IN_SECS;

                                    // Update the timestamp for the received block.
                                    locked_seen_blocks.insert(block_hash, SystemTime::now());
                                    // Drop the write lock.
                                    drop(locked_seen_blocks);

                                    // Ensure the unconfirmed block is at least within 2 blocks of the latest block height,
                                    // and no more that 2 blocks ahead of the latest block height.
                                    // If it is stale, skip the routing of this unconfirmed block to the ledger.
                                    let latest_block_height = peer.network_state.ledger.reader().latest_block_height();
                                    let lower_bound = latest_block_height.saturating_sub(2);
                                    let upper_bound = latest_block_height.saturating_add(2);
                                    let is_within_range = block_height >= lower_bound && block_height <= upper_bound;

                                    // Ensure the node is not peering.
                                    let is_node_ready = !E::status().is_peering();

                                    // If this node is a beacon or sync node, skip this message, after updating the timestamp.
                                    if E::NODE_TYPE == NodeType::Beacon
                                        || E::NODE_TYPE == NodeType::Sync
                                        || !is_router_ready
                                        || !is_within_range
                                        || !is_node_ready
                                    {
                                        trace!("Skipping 'UnconfirmedBlock {}' from {}", block_height, peer_ip)
                                    } else {
                                        // Perform the deferred non-blocking deserialization of the block.
                                        match block.deserialize().await {
                                            // Ensure the claimed block height and block hash matches in the deserialized block.
                                            Ok(block) => match block_height == block.height() && block_hash == block.hash() {
                                                // Route the `UnconfirmedBlock` to the ledger.
                                                true => peer.network_state.ledger.unconfirmed_block(peer_ip, block).await,
                                                // Route the `Failure` to the ledger.
                                                false => {
                                                    peer.network_state
                                                        .ledger
                                                        .add_failure(peer_ip, "Malformed UnconfirmedBlock message".to_string())
                                                        .await
                                                }
                                            },
                                            // Route the `Failure` to the ledger.
                                            Err(error) => peer.network_state.ledger.add_failure(peer_ip, format!("{}", error)).await,
                                        }
                                    }
                                }
                                Message::UnconfirmedTransaction(tx) => {
                                    // Drop the peer, if they have sent more than 500 unconfirmed transactions in the last 5 seconds.
                                    let frequency = peer
                                        .seen_inbound_transactions
                                        .read()
                                        .await
                                        .values()
                                        .filter(|t| t.elapsed().unwrap().as_secs() <= 5)
                                        .count();
                                    if frequency >= 500 {
                                        warn!(
                                            "Dropping {} for spamming unconfirmed transactions (frequency = {})",
                                            peer_ip, frequency
                                        );
                                        // Send a `PeerRestricted` message.
                                        peer.network_state.peers.peer_restricted(peer_ip).await;

                                        break;
                                    }

                                    // Perform the deferred non-blocking deserialisation of the
                                    // transaction.
                                    match tx.deserialize().await {
                                        Ok(tx) => {
                                            // Retrieve the last seen timestamp of the received transaction.
                                            let mut locked_seen_txs = peer.seen_inbound_transactions.write().await;
                                            let last_seen = locked_seen_txs.entry(tx.transaction_id()).or_insert(SystemTime::UNIX_EPOCH);
                                            let is_router_ready = last_seen.elapsed().unwrap().as_secs() > E::RADIO_SILENCE_IN_SECS;

                                            // Update the timestamp for the received transaction.
                                            locked_seen_txs.insert(tx.transaction_id(), SystemTime::now());

                                            // Ensure the node is not peering.
                                            let is_node_ready = !E::status().is_peering();

                                            // If this node is a beacon or sync node, skip this message, after updating the timestamp.
                                            if E::NODE_TYPE == NodeType::Beacon
                                                || E::NODE_TYPE == NodeType::Sync
                                                || !is_router_ready
                                                || !is_node_ready
                                            {
                                                trace!("Skipping 'UnconfirmedTransaction {}' from {}", tx.transaction_id(), peer_ip);
                                            } else {
                                                // Route the `UnconfirmedTransaction` to the prover.
                                                peer.network_state
                                                    .prover
                                                    .update(ProverRequest::UnconfirmedTransaction(peer_ip, tx))
                                                    .await;
                                            }
                                        }
                                        Err(error) => warn!("[UnconfirmedTransaction] {}", error),
                                    }
                                }
                                Message::PoolRegister(address) => {
                                    if E::NODE_TYPE != NodeType::Operator {
                                        trace!("Skipping 'PoolRegister' from {}", peer_ip);
                                    }

                                    peer.network_state
                                        .operator
                                        .update(OperatorRequest::PoolRegister(peer_ip, address))
                                        .await;
                                }
                                Message::PoolRequest(share_difficulty, block_template) => {
                                    if E::NODE_TYPE != NodeType::Prover {
                                        trace!("Skipping 'PoolRequest' from {}", peer_ip);
                                    } else if let Ok(block_template) = block_template.deserialize().await {
                                        peer.network_state
                                            .prover
                                            .update(ProverRequest::PoolRequest(peer_ip, share_difficulty, block_template))
                                            .await;
                                    } else {
                                        warn!("[PoolRequest] could not deserialize block template");
                                    }
                                }
                                Message::PoolResponse(address, nonce, proof) => {
                                    if E::NODE_TYPE != NodeType::Operator {
                                        trace!("Skipping 'PoolResponse' from {}", peer_ip);
                                    } else if let Ok(proof) = proof.deserialize().await {
                                        peer.network_state
                                            .operator
                                            .update(OperatorRequest::PoolResponse(peer_ip, address, nonce, proof))
                                            .await;
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
                    }
                }

                // When this is reached, it means the peer has disconnected.
                // Route a `Disconnect` to the ledger.
                peer.network_state
                    .ledger
                    .disconnect(peer_ip, DisconnectReason::PeerHasDisconnected)
                    .await;

                E::resources().deregister(inbound_resource_id);
            }),
        );
    }
}
