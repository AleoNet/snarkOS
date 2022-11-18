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
use snarkos_node_executor::{NodeType, RawStatus};
use snarkos_node_messages::*;
use snarkvm::prelude::{Block, Network, ProverSolution, Transaction};

use core::time::Duration;
use rand::Rng;
use snarkos_node_tcp::{protocols::Reading, ConnectionSide, P2P};
use std::{io, net::SocketAddr, sync::atomic::Ordering};
use std::time::Instant;

#[async_trait]
impl<N: Network, R: Routes<N>> Reading for Router<N, R> {
    type Codec = MessageCodec<N>;
    type Message = Message<N>;

    /// Creates a [`Decoder`] used to interpret messages from the network.
    /// The `side` param indicates the connection side **from the node's perspective**.
    fn codec(&self, _peer_addr: SocketAddr, _side: ConnectionSide) -> Self::Codec {
        Default::default()
    }

    /// Processes a message received from the network.
    async fn process_message(&self, peer_ip: SocketAddr, message: Self::Message) -> io::Result<()> {
        // Update the timestamp for the received message.
        self.connected_peers.read().get(&peer_ip).map(|peer| {
            peer.insert_seen_message(message.id(), rand::thread_rng().gen());
        });

        // Process the message.
        let success = self.routes.get().expect("Router must initialize routes").handle_message(self, peer_ip, message).await;

        // Disconnect if the peer violated the protocol.
        if !success {
            warn!("Disconnecting from '{peer_ip}' (violated protocol)");
            self.send(peer_ip, Message::Disconnect(DisconnectReason::ProtocolViolation.into()));
            // Disconnect from this peer.
            let _disconnected = self.tcp().disconnect(peer_ip).await;
            debug_assert!(_disconnected);
            // Restrict this peer to prevent reconnection.
            self.insert_restricted_peer(peer_ip);
        }

        Ok(())
    }
}

#[async_trait]
pub trait Routes<N: Network>: 'static + Clone + Send + Sync {
    /// The minimum number of peers required to maintain connections with.
    const MINIMUM_NUMBER_OF_PEERS: usize = 1;
    /// The maximum number of peers permitted to maintain connections with.
    const MAXIMUM_NUMBER_OF_PEERS: usize = 21;
    /// The node type.
    const NODE_TYPE: NodeType;

    /// Handles the message from the peer.
    async fn handle_message(&self, router: &Router<N, Self>, peer_ip: SocketAddr, message: Message<N>) -> bool {
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
                router.send(peer_ip, Message::PeerResponse(PeerResponse { peers: router.connected_peers() }));
                true
            }
            Message::PeerResponse(message) => {
                // Adds the given peer IPs to the list of candidate peers.
                router.insert_candidate_peers(&message.peers);
                true
            }
            Message::Ping(message) => Self::ping(router, peer_ip, message),
            Message::Pong(message) => Self::pong(router, peer_ip, message).await,
            Message::PuzzleRequest(..) => {
                // Retrieve the number of puzzle requests in this interval.
                let num_requests =
                    router.seen_inbound_puzzle_requests.write().entry(peer_ip).or_default().clone();
                // Check if the number of puzzle requests is within the limit.
                if num_requests.load(Ordering::SeqCst) < Router::<N, Self>::MAXIMUM_PUZZLE_REQUESTS_PER_INTERVAL {
                    // Increment the number of puzzle requests.
                    num_requests.fetch_add(1, Ordering::SeqCst);
                    // Process the puzzle request.
                    self.puzzle_request(peer_ip, router).await
                } else {
                    // Peer is not following the protocol.
                    warn!("Peer {peer_ip} is not following the protocol");
                    false
                }
            }
            Message::PuzzleResponse(message) => self.puzzle_response(message, peer_ip).await,
            Message::UnconfirmedBlock(message) => {
                // Clone the message.
                let message_clone = message.clone();

                // Update the timestamp for the unconfirmed block.
                let seen_before = router.cache.insert_inbound_block(message.block_hash).is_some();

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
                                self.unconfirmed_block(router, peer_ip, message_clone, block)
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
                let seen_before = router.cache.insert_inbound_solution(message.puzzle_commitment).is_some();

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
                                self.unconfirmed_solution(router, peer_ip, message_clone, solution)
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
                let seen_before = router.cache.insert_inbound_transaction(message.transaction_id).is_some();

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
                                self.unconfirmed_transaction(router, peer_ip, message_clone, transaction)
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

    fn ping(router: &Router<N, Self>, peer_ip: SocketAddr, message: Ping) -> bool {
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
        router.update_connected_peer(peer_ip, update_peer);

        // // Determine if the peer is on a fork (or unknown).
        // let is_fork = match state.ledger().reader().get_block_hash(peer.block_height) {
        //     Ok(expected_block_hash) => Some(expected_block_hash != block_hash),
        //     Err(_) => None,
        // };
        let is_fork = Some(false);

        // Send a `Pong` message to the peer.
        router.send(peer_ip, Message::Pong(Pong { is_fork }));
        true
    }

    async fn pong(router: &Router<N, Self>, peer_ip: SocketAddr, _message: Pong) -> bool {
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
        let router = router.clone();
        tokio::spawn(async move {
            // Sleep for the preset time before sending a `Ping` request.
            tokio::time::sleep(Duration::from_secs(Router::<N, Self>::PING_SLEEP_IN_SECS)).await;

            // Prepare the `Ping` message.
            let message = Message::Ping(Ping {
                version: Message::<N>::VERSION,
                fork_depth: ALEO_MAXIMUM_FORK_DEPTH,
                node_type: Self::NODE_TYPE,
                status: router.status.get(),
            });

            // Send a `Ping` message to the peer.
            router.send(peer_ip, message);
        });
        true
    }

    async fn puzzle_request(&self, peer_ip: SocketAddr, _router: &Router<N, Self>) -> bool {
        debug!("Disconnecting '{peer_ip}' for the following reason - {:?}", DisconnectReason::ProtocolViolation);
        false
    }

    async fn puzzle_response(&self, _message: PuzzleResponse<N>, peer_ip: SocketAddr) -> bool {
        debug!("Disconnecting '{peer_ip}' for the following reason - {:?}", DisconnectReason::ProtocolViolation);
        false
    }

    fn unconfirmed_block(
        &self,
        router: &Router<N, Self>,
        peer_ip: SocketAddr,
        message: UnconfirmedBlock<N>,
        _block: Block<N>,
    ) -> bool {
        // Propagate the `UnconfirmedBlock`.
        router.propagate(Message::UnconfirmedBlock(message), vec![peer_ip]);
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

    fn unconfirmed_solution(
        &self,
        router: &Router<N, Self>,
        peer_ip: SocketAddr,
        message: UnconfirmedSolution<N>,
        solution: ProverSolution<N>,
    ) -> bool;

    fn unconfirmed_transaction(
        &self,
        router: &Router<N, Self>,
        peer_ip: SocketAddr,
        message: UnconfirmedTransaction<N>,
        _transaction: Transaction<N>,
    ) -> bool {
        // Propagate the `UnconfirmedTransaction`.
        router.propagate(Message::UnconfirmedTransaction(message), vec![peer_ip]);
        true
    }
}
