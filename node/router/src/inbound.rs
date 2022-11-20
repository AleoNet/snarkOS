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

use crate::{Outbound, Peer, ALEO_MAXIMUM_FORK_DEPTH};
use snarkos_node_messages::{
    BlockRequest,
    BlockResponse,
    DisconnectReason,
    Message,
    PeerResponse,
    Ping,
    Pong,
    PuzzleResponse,
    RawStatus,
    UnconfirmedBlock,
    UnconfirmedSolution,
    UnconfirmedTransaction,
};
use snarkos_node_tcp::protocols::Reading;
use snarkvm::prelude::{Block, Network, ProverSolution, Transaction};

use anyhow::{bail, Result};
use std::{
    net::SocketAddr,
    time::{Duration, Instant},
};

#[async_trait]
pub trait Inbound<N: Network>: Reading + Outbound<N> {
    /// The maximum number of puzzle requests per interval.
    const MAXIMUM_PUZZLE_REQUESTS_PER_INTERVAL: usize = 5;
    /// The duration in seconds to sleep in between ping requests with a connected peer.
    const PING_SLEEP_IN_SECS: u64 = 60; // 1 minute

    /// Handles the inbound message from the peer.
    async fn inbound(&self, peer_addr: SocketAddr, message: Message<N>) -> Result<()> {
        // Retrieve the listener IP for the peer.
        let peer_ip = match self.router().resolve_to_listener(&peer_addr) {
            Some(peer_ip) => peer_ip,
            None => bail!("Unable to resolve the (ambiguous) peer address '{peer_addr}'"),
        };

        // Update the last seen timestamp of the peer.
        self.router().update_connected_peer(peer_ip, |peer: &mut Peer| {
            peer.set_last_seen(Instant::now());
        });

        // Drop the peer, if they have sent more than 50 messages in the last 5 seconds.
        let num_messages = self.router().cache.insert_inbound_message(peer_ip, 5);
        if num_messages >= 50 {
            bail!("Dropping {peer_ip} for spamming messages (num_messages = {num_messages})")
        }

        trace!("Received '{}' from '{peer_ip}'", message.name());

        // Process the message.
        match message {
            Message::BlockRequest(message) => match self.block_request(peer_ip, message) {
                true => Ok(()),
                false => bail!("Peer {peer_ip} sent an invalid block request"),
            },
            Message::BlockResponse(message) => match self.block_response(peer_ip, message) {
                true => Ok(()),
                false => bail!("Peer {peer_ip} sent an invalid block response"),
            },
            Message::ChallengeRequest(..) | Message::ChallengeResponse(..) => {
                // Disconnect as the peer is not following the protocol.
                bail!("Peer {peer_ip} is not following the protocol")
            }
            Message::Disconnect(message) => {
                bail!("Disconnecting peer {peer_ip} for the following reason: {:?}", message.reason)
            }
            Message::PeerRequest(..) => {
                // Retrieve the connected peers.
                let peers = self.router().connected_peers();
                // Send a `PeerResponse` message to the peer.
                self.send(peer_ip, Message::PeerResponse(PeerResponse { peers }));
                Ok(())
            }
            Message::PeerResponse(message) => {
                // Adds the given peer IPs to the list of candidate peers.
                self.router().insert_candidate_peers(&message.peers);
                Ok(())
            }
            Message::Ping(message) => match self.ping(peer_ip, message) {
                true => Ok(()),
                false => bail!("Peer {peer_ip} sent an invalid ping"),
            },
            Message::Pong(message) => match self.pong(peer_ip, message) {
                true => Ok(()),
                false => bail!("Peer {peer_ip} sent an invalid pong"),
            },
            Message::PuzzleRequest(..) => {
                // Insert the puzzle request for the peer, and fetch the recent frequency.
                let frequency = self.router().cache.insert_inbound_puzzle_request(peer_ip);
                // Check if the number of puzzle requests is within the limit.
                if frequency > Self::MAXIMUM_PUZZLE_REQUESTS_PER_INTERVAL {
                    bail!("Peer {peer_ip} is not following the protocol (excessive puzzle requests)")
                }
                // Process the puzzle request.
                match self.puzzle_request(peer_ip) {
                    true => Ok(()),
                    false => bail!("Peer {peer_ip} sent an invalid puzzle request"),
                }
            }
            Message::PuzzleResponse(message) => {
                // Check that this node previously sent a puzzle request to this peer.
                if !self.router().cache.contains_outbound_puzzle_request(&peer_ip) {
                    bail!("Peer {peer_ip} is not following the protocol (unexpected puzzle response)")
                }
                // Decrement the number of puzzle requests.
                self.router().cache.decrement_outbound_puzzle_requests(peer_ip);

                // Clone the message.
                let message_clone = message.clone();
                // Perform the deferred non-blocking deserialization of the block.
                let block = match message.block.deserialize().await {
                    Ok(block) => block,
                    Err(error) => bail!("[PuzzleResponse] {error}"),
                };
                // Process the puzzle response.
                match self.puzzle_response(peer_ip, message_clone, block) {
                    true => Ok(()),
                    false => bail!("Peer {peer_ip} sent an invalid puzzle response"),
                }
            }
            Message::UnconfirmedBlock(message) => {
                // Clone the message.
                let message_clone = message.clone();
                // Update the timestamp for the unconfirmed block.
                let seen_before = self.router().cache.insert_inbound_block(message.block_hash).is_some();
                // Determine whether to propagate the block.
                if seen_before {
                    bail!("Skipping 'UnconfirmedBlock' from '{peer_ip}'")
                }
                // Perform the deferred non-blocking deserialization of the block.
                let block = match message.block.deserialize().await {
                    Ok(block) => block,
                    Err(error) => bail!("[UnconfirmedBlock] {error}"),
                };
                // Check that the block parameters match.
                if message.block_height != block.height() || message.block_hash != block.hash() {
                    bail!("Peer {peer_ip} is not following the 'UnconfirmedBlock' protocol")
                }
                // Handle the unconfirmed block.
                match self.unconfirmed_block(peer_ip, message_clone, block) {
                    true => Ok(()),
                    false => bail!("Peer {peer_ip} sent an invalid unconfirmed block"),
                }
            }
            Message::UnconfirmedSolution(message) => {
                // Clone the message.
                let message_clone = message.clone();
                // Update the timestamp for the unconfirmed solution.
                let seen_before = self.router().cache.insert_inbound_solution(message.puzzle_commitment).is_some();
                // Determine whether to propagate the solution.
                if seen_before {
                    bail!("Skipping 'UnconfirmedSolution' from '{peer_ip}'")
                }
                // Perform the deferred non-blocking deserialization of the solution.
                let solution = match message.solution.deserialize().await {
                    Ok(solution) => solution,
                    Err(error) => bail!("[UnconfirmedSolution] {error}"),
                };
                // Check that the solution parameters match.
                if message.puzzle_commitment != solution.commitment() {
                    bail!("Peer {peer_ip} is not following the 'UnconfirmedSolution' protocol")
                }
                // Handle the unconfirmed solution.
                match self.unconfirmed_solution(peer_ip, message_clone, solution).await {
                    true => Ok(()),
                    false => bail!("Peer {peer_ip} sent an invalid unconfirmed solution"),
                }
            }
            Message::UnconfirmedTransaction(message) => {
                // Clone the message.
                let message_clone = message.clone();
                // Update the timestamp for the unconfirmed transaction.
                let seen_before = self.router().cache.insert_inbound_transaction(message.transaction_id).is_some();
                // Determine whether to propagate the transaction.
                if seen_before {
                    bail!("Skipping 'UnconfirmedTransaction' from '{peer_ip}'")
                }
                // Perform the deferred non-blocking deserialization of the transaction.
                let transaction = match message.transaction.deserialize().await {
                    Ok(transaction) => transaction,
                    Err(error) => bail!("[UnconfirmedTransaction] {error}"),
                };
                // Check that the transaction parameters match.
                if message.transaction_id != transaction.id() {
                    bail!("Peer {peer_ip} is not following the 'UnconfirmedTransaction' protocol")
                }
                // Handle the unconfirmed transaction.
                match self.unconfirmed_transaction(peer_ip, message_clone, transaction) {
                    true => Ok(()),
                    false => bail!("Peer {peer_ip} sent an invalid unconfirmed transaction"),
                }
            }
        }
    }

    fn block_request(&self, peer_ip: SocketAddr, _message: BlockRequest) -> bool {
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

    fn block_response(&self, peer_ip: SocketAddr, _message: BlockResponse<N>) -> bool {
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
        self.router().update_connected_peer(peer_ip, |peer: &mut Peer| {
            // Update the last seen timestamp of the peer.
            peer.set_last_seen(Instant::now());
            // Update the version of the peer.
            peer.set_version(message.version);
            // Update the node type of the peer.
            peer.set_node_type(message.node_type);
            // Update the status of the peer.
            peer.set_status(RawStatus::from_status(message.status));
        });

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

    fn pong(&self, peer_ip: SocketAddr, _message: Pong) -> bool {
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
            tokio::time::sleep(Duration::from_secs(Self::PING_SLEEP_IN_SECS)).await;

            // Prepare the `Ping` message.
            let message = Message::Ping(Ping {
                version: Message::<N>::VERSION,
                fork_depth: ALEO_MAXIMUM_FORK_DEPTH,
                node_type: self_clone.router().node_type(),
                status: self_clone.router().status(),
            });

            // Send a `Ping` message to the peer.
            self_clone.send(peer_ip, message);
        });
        true
    }

    fn puzzle_request(&self, peer_ip: SocketAddr) -> bool {
        debug!("Disconnecting '{peer_ip}' for the following reason - {:?}", DisconnectReason::ProtocolViolation);
        false
    }

    fn puzzle_response(&self, peer_ip: SocketAddr, _message: PuzzleResponse<N>, _block: Block<N>) -> bool {
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
}
