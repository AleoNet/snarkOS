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

use crate::{Cache, Peer, Router, ALEO_MAXIMUM_FORK_DEPTH};
use snarkos_node_executor::{NodeType, RawStatus};
use snarkos_node_messages::*;
use snarkos_node_tcp::{
    protocols::{Disconnect, Handshake, Reading, Writing},
    P2P,
};
use snarkvm::prelude::{Block, Network, ProverSolution, Transaction};
use std::collections::HashMap;

use futures::SinkExt;
use rand::{prelude::IteratorRandom, rngs::OsRng};
use std::{
    net::SocketAddr,
    sync::atomic::Ordering,
    time::{Duration, Instant},
};

#[async_trait]
pub trait Routes<N: Network>: P2P + Disconnect + Handshake + Reading + Writing<Message = Message<N>> {
    /// The minimum number of peers required to maintain connections with.
    const MINIMUM_NUMBER_OF_PEERS: usize = 1;
    /// The maximum number of peers permitted to maintain connections with.
    const MAXIMUM_NUMBER_OF_PEERS: usize = 21;

    /// Returns a reference to the router.
    fn router(&self) -> &Router<N>;

    /// Initialize the routes.
    async fn initialize_routes(&self) {
        // Enable the TCP protocols.
        self.enable_handshake().await;
        self.enable_reading().await;
        self.enable_writing().await;
        self.enable_disconnect().await;
        // Initialize the heartbeat.
        self.initialize_heartbeat().await;
        // Initialize the puzzle request.
        self.initialize_puzzle_request().await;
        // Initialize the report.
        self.initialize_report().await;
    }

    /// Initialize a new instance of the heartbeat.
    async fn initialize_heartbeat(&self) {
        let self_clone = self.clone();
        tokio::spawn(async move {
            loop {
                // Process a heartbeat in the router.
                self_clone.heartbeat().await;
                // Sleep for `HEARTBEAT_IN_SECS` seconds.
                tokio::time::sleep(Duration::from_secs(Router::<N>::HEARTBEAT_IN_SECS)).await;
            }
        });
    }

    /// Initialize a new instance of the puzzle request.
    async fn initialize_puzzle_request(&self) {
        if !self.router().node_type.is_beacon() {
            let self_clone = self.clone();
            tokio::spawn(async move {
                loop {
                    // Send a "PuzzleRequest".
                    self_clone.send_puzzle_request(self_clone.router().node_type);
                    // Sleep for `PUZZLE_REQUEST_IN_SECS` seconds.
                    tokio::time::sleep(Duration::from_secs(Router::<N>::PUZZLE_REQUEST_IN_SECS)).await;
                }
            });
        }
    }

    /// Initialize a new instance of the report.
    async fn initialize_report(&self) {
        let self_clone = self.clone();
        tokio::spawn(async move {
            let url = "https://vm.aleo.org/testnet3/report";
            loop {
                // Prepare the report.
                let mut report = HashMap::new();
                report.insert("node_address".to_string(), self_clone.router().address.to_string());
                report.insert("node_type".to_string(), self_clone.router().node_type.to_string());
                // Transmit the report.
                if reqwest::Client::new().post(url).json(&report).send().await.is_err() {
                    warn!("Failed to send report");
                }
                // Sleep for a fixed duration in seconds.
                tokio::time::sleep(Duration::from_secs(3600 * 6)).await;
            }
        });
    }

    /// Sends a "PuzzleRequest" to a reliable peer.
    async fn send_puzzle_request(&self, node_type: NodeType) {
        // TODO (howardwu): Change this logic for Phase 3.
        // Retrieve a reliable peer.
        let reliable_peer = match node_type.is_validator() {
            true => self.router().connected_beacons().first().copied(),
            false => self.router().reliable_peers().first().copied(),
        };
        // If a reliable peer exists, send a "PuzzleRequest" to it.
        if let Some(reliable_peer) = reliable_peer {
            // Send the "PuzzleRequest" to the reliable peer.
            self.send(reliable_peer, Message::PuzzleRequest(PuzzleRequest));
        } else {
            warn!("[PuzzleRequest] There are no reliable peers available yet");
        }
    }

    /// Sends the given message to specified peer.
    fn send(&self, peer_ip: SocketAddr, message: Message<N>) {
        // Determine whether to send the message.
        if !self.should_send(&message) {
            return;
        }
        // Ensure the peer is connected before sending.
        match self.router().is_connected(&peer_ip) {
            true => {
                trace!("Sending '{}' to '{peer_ip}'", message.name());
                if let Err(error) = self.unicast(peer_ip, message) {
                    trace!("Failed to send message to '{peer_ip}': {error}");
                }
            }
            false => warn!("Attempted to send to a non-connected peer {peer_ip}"),
        }
    }

    /// Sends the given message to every connected peer, excluding the sender and any specified peer IPs.
    fn propagate(&self, mut message: Message<N>, excluded_peers: Vec<SocketAddr>) {
        // // Perform ahead-of-time, non-blocking serialization just once for applicable objects.
        // if let Message::UnconfirmedBlock(ref mut message) = message {
        //     if let Ok(serialized_block) = Data::serialize(message.block.clone()).await {
        //         let _ = std::mem::replace(&mut message.block, Data::Buffer(serialized_block));
        //     } else {
        //         error!("Block serialization is bugged");
        //     }
        // } else if let Message::UnconfirmedSolution(ref mut message) = message {
        //     if let Ok(serialized_solution) = Data::serialize(message.solution.clone()).await {
        //         let _ = std::mem::replace(&mut message.solution, Data::Buffer(serialized_solution));
        //     } else {
        //         error!("Solution serialization is bugged");
        //     }
        // } else if let Message::UnconfirmedTransaction(ref mut message) = message {
        //     if let Ok(serialized_transaction) = Data::serialize(message.transaction.clone()).await {
        //         let _ = std::mem::replace(&mut message.transaction, Data::Buffer(serialized_transaction));
        //     } else {
        //         error!("Transaction serialization is bugged");
        //     }
        // }

        // Determine whether to send the message.
        if !self.should_send(&message) {
            return;
        }
        // Iterate through all peers that are not the sender and excluded peers.
        for peer_ip in self
            .router()
            .connected_peers()
            .iter()
            .filter(|peer_ip| !self.router().is_local_ip(peer_ip) && !excluded_peers.contains(peer_ip))
        {
            trace!("Sending '{}' to '{peer_ip}'", message.name());
            if let Err(error) = self.unicast(*peer_ip, message.clone()) {
                warn!("Failed to send '{}' to '{peer_ip}': {error}", message.name());
            }
        }
    }

    /// Sends the given message to every connected beacon, excluding the sender and any specified beacon IPs.
    fn propagate_to_beacons(&self, mut message: Message<N>, excluded_beacons: Vec<SocketAddr>) {
        // // Perform ahead-of-time, non-blocking serialization just once for applicable objects.
        // if let Message::UnconfirmedBlock(ref mut message) = message {
        //     if let Ok(serialized_block) = Data::serialize(message.block.clone()).await {
        //         let _ = std::mem::replace(&mut message.block, Data::Buffer(serialized_block));
        //     } else {
        //         error!("Block serialization is bugged");
        //     }
        // } else if let Message::UnconfirmedSolution(ref mut message) = message {
        //     if let Ok(serialized_solution) = Data::serialize(message.solution.clone()).await {
        //         let _ = std::mem::replace(&mut message.solution, Data::Buffer(serialized_solution));
        //     } else {
        //         error!("Solution serialization is bugged");
        //     }
        // } else if let Message::UnconfirmedTransaction(ref mut message) = message {
        //     if let Ok(serialized_transaction) = Data::serialize(message.transaction.clone()).await {
        //         let _ = std::mem::replace(&mut message.transaction, Data::Buffer(serialized_transaction));
        //     } else {
        //         error!("Transaction serialization is bugged");
        //     }
        // }

        // Determine whether to send the message.
        if !self.should_send(&message) {
            return;
        }
        // Iterate through all beacons that are not the sender and excluded beacons.
        for peer_ip in self
            .router()
            .connected_beacons()
            .iter()
            .filter(|peer_ip| !self.router().is_local_ip(peer_ip) && !excluded_beacons.contains(peer_ip))
        {
            trace!("Sending '{}' to '{peer_ip}'", message.name());
            if let Err(error) = self.unicast(*peer_ip, message.clone()) {
                warn!("Failed to send '{}' to '{peer_ip}': {error}", message.name());
            }
        }
    }

    /// Returns `true` if the message should be sent.
    fn should_send(&self, message: &Message<N>) -> bool {
        // Determine whether to send the message.
        match message {
            Message::UnconfirmedBlock(message) => {
                // Update the timestamp for the unconfirmed block.
                let seen_before = self.router().cache.insert_outbound_block(message.block_hash).is_some();
                // Determine whether to send the block.
                !seen_before
            }
            Message::UnconfirmedSolution(message) => {
                // Update the timestamp for the unconfirmed solution.
                let seen_before = self.router().cache.insert_outbound_solution(message.puzzle_commitment).is_some();
                // Determine whether to send the solution.
                !seen_before
            }
            Message::UnconfirmedTransaction(message) => {
                // Update the timestamp for the unconfirmed transaction.
                let seen_before = self.router().cache.insert_outbound_transaction(message.transaction_id).is_some();
                // Determine whether to send the transaction.
                !seen_before
            }
            // For all other message types, return `true`.
            _ => true,
        }
    }

    /// Handles the message from the peer.
    async fn handle_message(&self, peer_ip: SocketAddr, message: Message<N>) -> bool {
        // Process the message.
        trace!("Received '{}' from '{peer_ip}'", message.name());
        match message {
            Message::BlockRequest(message) => Self::block_request(message, peer_ip),
            Message::BlockResponse(message) => Self::block_response(message, peer_ip),
            Message::ChallengeRequest(..) | Message::ChallengeResponse(..) => {
                // Peer is not following the protocol.
                warn!("Peer {peer_ip} is not following the protocol");
                false
            }
            Message::Disconnect(message) => {
                debug!("Disconnecting peer {peer_ip} for the following reason: {:?}", message.reason);
                false
            }
            Message::PeerRequest(..) => {
                // Send a `PeerResponse` message.
                self.send(peer_ip, Message::PeerResponse(PeerResponse { peers: self.router().connected_peers() }));
                true
            }
            Message::PeerResponse(message) => {
                // Adds the given peer IPs to the list of candidate peers.
                self.router().insert_candidate_peers(&message.peers);
                true
            }
            Message::Ping(message) => self.ping(peer_ip, message),
            Message::Pong(message) => self.pong(peer_ip, message).await,
            Message::PuzzleRequest(..) => {
                // Insert the puzzle request for the peer, and fetch the recent frequency.
                let frequency = self.router().cache.insert_inbound_puzzle_request(peer_ip);
                // Check if the number of puzzle requests is within the limit.
                if frequency < Router::<N>::MAXIMUM_PUZZLE_REQUESTS_PER_INTERVAL {
                    // Process the puzzle request.
                    self.puzzle_request(peer_ip).await
                } else {
                    // Peer is not following the protocol.
                    warn!("Peer {peer_ip} is not following the protocol");
                    false
                }
            }
            Message::PuzzleResponse(message) => {
                // Check that this node previously sent a puzzle request to this peer.
                if self.router().cache.contains_outbound_puzzle_request(&peer_ip) {
                    // Decrement the number of puzzle requests.
                    self.router().cache.decrement_outbound_puzzle_requests(peer_ip);
                    // Process the puzzle response.
                    self.puzzle_response(peer_ip, message).await
                } else {
                    // Peer is not following the protocol.
                    warn!("Peer {peer_ip} is not following the protocol");
                    false
                }
            }
            Message::UnconfirmedBlock(message) => {
                // Clone the message.
                let message_clone = message.clone();

                // Update the timestamp for the unconfirmed block.
                let seen_before = self.router().cache.insert_inbound_block(message.block_hash).is_some();
                // Determine whether to propagate the block.
                if seen_before {
                    trace!("Skipping 'UnconfirmedBlock {}' from '{peer_ip}'", message.block_hash);
                    true
                } else {
                    // Perform the deferred non-blocking deserialization of the block.
                    match message.block.deserialize().await {
                        Ok(block) => {
                            // Check that the block parameters match.
                            if message.block_height != block.height() || message.block_hash != block.hash() {
                                // Peer is not following the protocol.
                                warn!("Peer {peer_ip} is not following the 'UnconfirmedBlock' protocol");
                                false
                            } else {
                                // Handle the unconfirmed block.
                                self.unconfirmed_block(peer_ip, message_clone, block)
                            }
                        }
                        Err(error) => {
                            warn!("[UnconfirmedBlock] {error}");
                            true
                        }
                    }
                }
            }
            Message::UnconfirmedSolution(message) => {
                // Clone the message.
                let message_clone = message.clone();

                // Update the timestamp for the unconfirmed solution.
                let seen_before = self.router().cache.insert_inbound_solution(message.puzzle_commitment).is_some();
                // Determine whether to propagate the solution.
                if seen_before {
                    trace!("Skipping 'UnconfirmedSolution' from '{peer_ip}'");
                    true
                } else {
                    // Perform the deferred non-blocking deserialization of the solution.
                    match message.solution.deserialize().await {
                        Ok(solution) => {
                            // Check that the solution parameters match.
                            if message.puzzle_commitment != solution.commitment() {
                                // Peer is not following the protocol.
                                warn!("Peer {peer_ip} is not following the 'UnconfirmedSolution' protocol");
                                false
                            } else {
                                // Handle the unconfirmed solution.
                                self.unconfirmed_solution(peer_ip, message_clone, solution).await
                            }
                        }
                        Err(error) => {
                            warn!("[UnconfirmedSolution] {error}");
                            true
                        }
                    }
                }
            }
            Message::UnconfirmedTransaction(message) => {
                // Clone the message.
                let message_clone = message.clone();

                // Update the timestamp for the unconfirmed transaction.
                let seen_before = self.router().cache.insert_inbound_transaction(message.transaction_id).is_some();
                // Determine whether to propagate the transaction.
                if seen_before {
                    trace!("Skipping 'UnconfirmedTransaction {}' from '{peer_ip}'", message.transaction_id);
                    true
                } else {
                    // Perform the deferred non-blocking deserialization of the transaction.
                    match message.transaction.deserialize().await {
                        Ok(transaction) => {
                            // Check that the transaction parameters match.
                            if message.transaction_id != transaction.id() {
                                // Peer is not following the protocol.
                                warn!("Peer {peer_ip} is not following the 'UnconfirmedTransaction' protocol");
                                false
                            } else {
                                // Handle the unconfirmed transaction.
                                self.unconfirmed_transaction(peer_ip, message_clone, transaction)
                            }
                        }
                        Err(error) => {
                            warn!("[UnconfirmedTransaction] {error}");
                            true
                        }
                    }
                }
            }
        }
    }

    fn block_request(_message: BlockRequest, peer_ip: SocketAddr) -> bool {
        // // Ensure the request is within the accepted limits.
        // let number_of_blocks = end_block_height.saturating_sub(start_block_height);
        // if number_of_blocks > Router::<N>::MAXIMUM_BLOCK_REQUEST {
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
        debug!("Disconnecting '{peer_ip}' for the following reason - {:?}", DisconnectReason::ProtocolViolation);
        false
    }

    fn block_response(_message: BlockResponse<N>, peer_ip: SocketAddr) -> bool {
        // // Perform the deferred non-blocking deserialization of the block.
        // match block.deserialize().await {
        //     Ok(block) => {
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
        debug!("Disconnecting '{peer_ip}' for the following reason - {:?}", DisconnectReason::ProtocolViolation);
        false
    }

    fn ping(&self, peer_ip: SocketAddr, message: Ping) -> bool {
        // Ensure the message protocol version is not outdated.
        if message.version < Message::<N>::VERSION {
            warn!("Dropping {peer_ip} on version {} (outdated)", message.version);
            return false;
        }
        // Ensure the maximum fork depth is correct.
        if message.fork_depth != ALEO_MAXIMUM_FORK_DEPTH {
            warn!("Dropping {peer_ip} for an incorrect maximum fork depth of {}", message.fork_depth);
            return false;
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
        //         // Update peer's block height.
        //         peer.block_height = block_header.height();
        //     }
        //     Err(error) => warn!("[Ping] {}", error),
        // }

        // Update the connected peer.
        let update_peer = |peer: &mut Peer| {
            // Update the last seen timestamp of the peer.
            peer.set_last_seen(Instant::now());
            // Update the version of the peer.
            peer.set_version(message.version);
            // Update the node type of the peer.
            peer.set_node_type(message.node_type);
            // Update the status of the peer.
            peer.set_status(RawStatus::from_status(message.status));
        };
        self.router().update_connected_peer(peer_ip, update_peer);

        // // Determine if the peer is on a fork (or unknown).
        // let is_fork = match state.ledger().reader().get_block_hash(peer.block_height) {
        //     Ok(expected_block_hash) => Some(expected_block_hash != block_hash),
        //     Err(_) => None,
        // };
        let is_fork = Some(false);

        // Send a `Pong` message to the peer.
        self.send(peer_ip, Message::Pong(Pong { is_fork }));
        true
    }

    async fn pong(&self, peer_ip: SocketAddr, _message: Pong) -> bool {
        // // Perform the deferred non-blocking deserialization of block locators.
        // let request = match block_locators.deserialize().await {
        //     // Route the `Pong` to the ledger.
        //     Ok(block_locators) => LedgerRequest::Pong(peer_ip, peer.node_type, peer.status, is_fork, block_locators),
        //     // Route the `Failure` to the ledger.
        //     Err(error) => LedgerRequest::Failure(peer_ip, format!("{}", error)),
        // };
        //
        // // Route the request to the ledger.
        // if let Err(error) = state.ledger().router().send(request).await {
        //     warn!("[Pong] {}", error);
        // }

        // Spawn an asynchronous task for the `Ping` request.
        let self_clone = self.clone();
        tokio::spawn(async move {
            // Sleep for the preset time before sending a `Ping` request.
            tokio::time::sleep(Duration::from_secs(Router::<N>::PING_SLEEP_IN_SECS)).await;

            // Prepare the `Ping` message.
            let message = Message::Ping(Ping {
                version: Message::<N>::VERSION,
                fork_depth: ALEO_MAXIMUM_FORK_DEPTH,
                node_type: self_clone.router().node_type,
                status: self_clone.router().status.get(),
            });

            // Send a `Ping` message to the peer.
            self_clone.send(peer_ip, message);
        });
        true
    }

    async fn puzzle_request(&self, peer_ip: SocketAddr) -> bool {
        debug!("Disconnecting '{peer_ip}' for the following reason - {:?}", DisconnectReason::ProtocolViolation);
        false
    }

    async fn puzzle_response(&self, peer_ip: SocketAddr, _message: PuzzleResponse<N>) -> bool {
        debug!("Disconnecting '{peer_ip}' for the following reason - {:?}", DisconnectReason::ProtocolViolation);
        false
    }

    fn unconfirmed_block(&self, peer_ip: SocketAddr, message: UnconfirmedBlock<N>, _block: Block<N>) -> bool {
        // Propagate the `UnconfirmedBlock`.
        self.propagate(Message::UnconfirmedBlock(message), vec![peer_ip]);
        true

        // // Ensure the unconfirmed block is at least within 2 blocks of the latest block height,
        // // and no more that 2 blocks ahead of the latest block height.
        // // If it is stale, skip the routing of this unconfirmed block to the ledger.
        // let latest_block_height = state.ledger().reader().latest_block_height();
        // let lower_bound = latest_block_height.saturating_sub(2);
        // let upper_bound = latest_block_height.saturating_add(2);
        // let is_within_range = block_height >= lower_bound && block_height <= upper_bound;
        //
        // // Ensure the node is not peering.
        // let is_node_ready = !Self::status().is_peering();
        //
        // if !is_router_ready || !is_within_range || !is_node_ready {
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

    async fn unconfirmed_solution(
        &self,
        peer_ip: SocketAddr,
        message: UnconfirmedSolution<N>,
        solution: ProverSolution<N>,
    ) -> bool;

    fn unconfirmed_transaction(
        &self,
        peer_ip: SocketAddr,
        message: UnconfirmedTransaction<N>,
        _transaction: Transaction<N>,
    ) -> bool {
        // Propagate the `UnconfirmedTransaction`.
        self.propagate(Message::UnconfirmedTransaction(message), vec![peer_ip]);
        true
    }

    /// Handles the heartbeat request.
    async fn heartbeat(&self) {
        debug!("Peers: {:?}", self.router().connected_peers());

        // TODO (howardwu): Remove this in Phase 3.
        if self.router().node_type.is_beacon() {
            // Proceed to send disconnect requests to these peers.
            for peer_ip in self.router().connected_peers() {
                if !self.router().trusted_peers().contains(&peer_ip) {
                    info!("Disconnecting from '{peer_ip}' (exceeded maximum connections)");
                    self.send(peer_ip, Message::Disconnect(DisconnectReason::TooManyPeers.into()));
                    // Disconnect from this peer.
                    let _disconnected = self.router().tcp.disconnect(peer_ip).await;
                    debug_assert!(_disconnected);
                    // Restrict this peer to prevent reconnection.
                    self.router().insert_restricted_peer(peer_ip);
                }
            }
        }

        // Check if any connected peer is stale.
        let connected_peers = self.router().connected_peers.read().clone();
        for peer in connected_peers.into_values() {
            let peer_ip = *peer.ip();
            // Disconnect if the peer has not communicated back within the predefined time.
            let last_seen_elapsed = peer.last_seen().elapsed().as_secs();
            if last_seen_elapsed > Router::<N>::RADIO_SILENCE_IN_SECS {
                warn!("Peer {peer_ip} has not communicated in {last_seen_elapsed} seconds");
                // Disconnect from this peer.
                let _disconnected = self.router().tcp.disconnect(peer_ip).await;
                debug_assert!(_disconnected);
                // Restrict this peer to prevent reconnection.
                self.router().insert_restricted_peer(peer_ip);
            }

            // Drop the peer, if they have sent more than 50 messages in the last 5 seconds.
            let frequency = peer.message_frequency();
            if frequency >= 50 {
                warn!("Dropping {peer_ip} for spamming messages (frequency = {frequency})");
                // Disconnect from this peer.
                let _disconnected = self.router().tcp.disconnect(peer_ip).await;
                debug_assert!(_disconnected);
                // Restrict this peer to prevent reconnection.
                self.router().insert_restricted_peer(peer_ip);
            }
        }

        // Compute the number of excess peers.
        let num_excess_peers = self.router().number_of_connected_peers().saturating_sub(Self::MAXIMUM_NUMBER_OF_PEERS);
        // Ensure the number of connected peers is below the maximum threshold.
        if num_excess_peers > 0 {
            debug!("Exceeded maximum number of connected peers, disconnecting from {num_excess_peers} peers");
            // Determine the peers to disconnect from.
            let peer_ips_to_disconnect = self
                .router()
                .connected_peers()
                .into_iter()
                .filter(
                    |peer_ip| /* !E::beacon_nodes().contains(&peer_ip) && */ !self.router().trusted_peers().contains(peer_ip),
                )
                .take(num_excess_peers);

            // Proceed to send disconnect requests to these peers.
            for peer_ip in peer_ips_to_disconnect {
                info!("Disconnecting from '{peer_ip}' (exceeded maximum connections)");
                self.send(peer_ip, Message::Disconnect(DisconnectReason::TooManyPeers.into()));
                // Disconnect from this peer.
                let _disconnected = self.router().tcp.disconnect(peer_ip).await;
                debug_assert!(_disconnected);
                // Restrict this peer to prevent reconnection.
                self.router().insert_restricted_peer(peer_ip);
            }
        }

        // TODO (howardwu): This logic can be optimized and unified with the context around it.
        // Determine if the node is connected to more sync nodes than allowed.
        let connected_beacons = self.router().connected_beacons();
        let num_excess_beacons = connected_beacons.len().saturating_sub(1);
        if num_excess_beacons > 0 {
            debug!("Exceeded maximum number of beacons");

            // Proceed to send disconnect requests to these peers.
            for peer_ip in connected_beacons.iter().copied().choose_multiple(&mut OsRng::default(), num_excess_beacons)
            {
                info!("Disconnecting from 'beacon' {peer_ip} (exceeded maximum connections)");
                self.send(peer_ip, Message::Disconnect(DisconnectReason::TooManyPeers.into()));
                // Disconnect from this peer.
                let _disconnected = self.router().tcp.disconnect(peer_ip).await;
                debug_assert!(_disconnected);
                // Restrict this peer to prevent reconnection.
                self.router().insert_restricted_peer(peer_ip);
            }
        }

        // Ensure that the trusted nodes are connected.
        for peer_ip in self.router().trusted_peers() {
            // If the peer is not connected, attempt to connect to it.
            if !self.router().is_connected(peer_ip) {
                // Attempt to connect to the trusted peer.
                if let Err(error) = self.router().tcp.connect(*peer_ip).await {
                    warn!("Failed to connect to trusted peer '{peer_ip}': {error}");
                }
            }
        }

        // Obtain the number of connected peers.
        let num_connected = self.router().number_of_connected_peers();
        let num_to_connect_with = Self::MINIMUM_NUMBER_OF_PEERS.saturating_sub(num_connected);
        // Request more peers if the number of connected peers is below the threshold.
        if num_to_connect_with > 0 {
            trace!("Sending requests for more peer connections");

            // Request more peers from the connected peers.
            for candidate_addr in self.router().candidate_peers().into_iter().take(num_to_connect_with) {
                // Attempt to connect to the candidate peer.
                let connection_succesful = self.router().tcp.connect(candidate_addr).await.is_ok();
                // Remove the peer from the candidate peers.
                self.router().remove_candidate_peer(candidate_addr);
                // Restrict the peer if the connection was not successful.
                if !connection_succesful {
                    self.router().insert_restricted_peer(candidate_addr);
                }
            }

            // If we have connected peers, request more addresses from them.
            if num_connected > 0 {
                for peer_ip in self.router().connected_peers().iter().choose_multiple(&mut OsRng::default(), 3) {
                    self.send(*peer_ip, Message::PeerRequest(PeerRequest));
                }
            }
        }
    }
}
