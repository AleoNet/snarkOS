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
    ///
    /// Initialize the handler for the new peer.
    ///
    pub(super) async fn handler(self, mut outbound_socket: Framed<TcpStream, MessageCodec<N>>, mut peer_handler: PeerHandler<N>) {
        // Retrieve the peers router.
        let peers_router = self.state.peers().router().clone();
        let peer = self.clone();
        spawn_task!(E::resources().procure_id(), {
            // Retrieve the peer IP.
            let peer_ip = *peer.ip();
            info!("Connected to {}", peer_ip);

            // Process incoming messages until this stream is disconnected.
            loop {
                tokio::select! {
                    // Message channel is routing a message outbound to the peer.
                    Some(mut message) = peer_handler.recv() => {
                        // Disconnect if the peer has not communicated back within the predefined time.
                        let last_seen_elapsed = peer.last_seen.read().await.elapsed().as_secs();
                        if last_seen_elapsed > E::RADIO_SILENCE_IN_SECS {
                            warn!("Peer {peer_ip} has not communicated in {last_seen_elapsed} seconds");
                            break;
                        }

                        // Ensure sufficient time has passed before needing to send the message.
                        let is_ready_to_send = match message {
                            Message::UnconfirmedBlock(block_height, block_hash, ref mut data) => {
                                // Retrieve the last seen timestamp of this block for this peer.
                                let last_seen = peer.seen_outbound_blocks.write().await.entry(block_hash).or_insert(SystemTime::UNIX_EPOCH).elapsed().unwrap().as_secs();
                                let is_ready_to_send = last_seen > E::RADIO_SILENCE_IN_SECS;

                                // Update the timestamp for the peer and sent block.
                                peer.seen_outbound_blocks.write().await.insert(block_hash, SystemTime::now());
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
                                let last_seen = peer.seen_outbound_transactions.write().await.entry(transaction.id()).or_insert(SystemTime::UNIX_EPOCH).elapsed().unwrap().as_secs();
                                let is_ready_to_send = last_seen > E::RADIO_SILENCE_IN_SECS;

                                // Update the timestamp for the peer and sent transaction.
                                peer.seen_outbound_transactions.write().await.insert(transaction.id(), SystemTime::now());
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
                            trace!("Sending '{}' to {}", message.name(), self.ip());

                            // Route the message to the peer.
                            if let Err(error) = outbound_socket.send(message).await {
                                warn!("[OutboundRouter] {}", error);
                            }
                        }
                    }
                    result = outbound_socket.next() => match result {
                        // Received a message from the peer.
                        Some(Ok(message)) => {
                            // Disconnect if the peer has not communicated back within the predefined time.
                            let last_seen_elapsed = peer.last_seen.read().await.elapsed().as_secs();
                            match last_seen_elapsed > E::RADIO_SILENCE_IN_SECS {
                                true => {
                                    warn!("Failed to receive a message from {peer_ip} in {last_seen_elapsed} seconds");
                                    break;
                                },
                                false => {
                                    // Update the last seen timestamp.
                                    *peer.last_seen.write().await = Instant::now();
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
                                    *peer.version.write().await = version;
                                    // Update the node type of the peer.
                                    *peer.node_type.write().await = node_type;
                                    // Update the status of the peer.
                                    *peer.status.write().await = status;

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
                                    let frequency = peer.seen_inbound_blocks.read().await.values().filter(|t| t.elapsed().unwrap().as_secs() <= 5).count();
                                    if frequency >= 10 {
                                        warn!("Dropping {} for spamming unconfirmed blocks (frequency = {})", peer_ip, frequency);
                                        // Send a `PeerRestricted` message.
                                        if let Err(error) = peers_router.send(PeersRequest::PeerRestricted(peer_ip)).await {
                                            warn!("[PeerRestricted] {}", error);
                                        }
                                        break;
                                    }

                                    // Acquire the lock on the seen inbound blocks.
                                    let mut seen_inbound_blocks = peer.seen_inbound_blocks.write().await;

                                    // Retrieve the last seen timestamp of the received block.
                                    let last_seen = seen_inbound_blocks.entry(block_hash).or_insert(SystemTime::UNIX_EPOCH);
                                    let is_router_ready = last_seen.elapsed().unwrap().as_secs() > E::RADIO_SILENCE_IN_SECS;

                                    // Update the timestamp for the received block.
                                    seen_inbound_blocks.insert(block_hash, SystemTime::now());
                                    // Drop the lock on the seen inbound blocks.
                                    drop(seen_inbound_blocks);

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
                                    let frequency = peer.seen_inbound_transactions.read().await.values().filter(|t| t.elapsed().unwrap().as_secs() <= 5).count();
                                    if frequency >= 500 {
                                        warn!("Dropping {} for spamming unconfirmed transactions (frequency = {})", peer_ip, frequency);
                                        // Send a `PeerRestricted` message.
                                        if let Err(error) = peers_router.send(PeersRequest::PeerRestricted(peer_ip)).await {
                                            warn!("[PeerRestricted] {}", error);
                                        }
                                        break;
                                    }

                                    // Perform the deferred non-blocking deserialization of the
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
        });
    }
}
