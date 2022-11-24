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

use anyhow::{bail, ensure, Result};
use std::{net::SocketAddr, time::Instant};

#[async_trait]
pub trait Inbound<N: Network>: Reading + Outbound<N> {
    /// The maximum number of puzzle requests per interval.
    const MAXIMUM_PUZZLE_REQUESTS_PER_INTERVAL: usize = 5;
    /// The duration in seconds to sleep in between ping requests with a connected peer.
    const PING_SLEEP_IN_SECS: u64 = 15; // 15 seconds

    /// Handles the inbound message from the peer.
    async fn inbound(&self, peer_addr: SocketAddr, message: Message<N>) -> Result<()> {
        // Retrieve the listener IP for the peer.
        let peer_ip = match self.router().resolve_to_listener(&peer_addr) {
            Some(peer_ip) => peer_ip,
            None => bail!("Unable to resolve the (ambiguous) peer address '{peer_addr}'"),
        };

        // Drop the peer, if they have sent more than 50 messages in the last 5 seconds.
        let num_messages = self.router().cache.insert_inbound_message(peer_ip, 5);
        if num_messages >= 50 {
            bail!("Dropping '{peer_ip}' for spamming messages (num_messages = {num_messages})")
        }

        trace!("Received '{}' from '{peer_ip}'", message.name());

        // Process the message.
        match message {
            Message::BlockRequest(message) => match self.block_request(peer_ip, message) {
                true => Ok(()),
                false => bail!("Peer '{peer_ip}' sent an invalid block request"),
            },
            Message::BlockResponse(message) => {
                let request = message.request;

                // Check that this node previously sent a block request to this peer.
                if !self.router().cache.contains_outbound_block_request(&peer_ip, &request) {
                    bail!("Peer '{peer_ip}' is not following the protocol (unexpected block response)")
                }
                // Remove the block request.
                self.router().cache.remove_outbound_block_request(peer_ip, &request);

                // Perform the deferred non-blocking deserialization of the blocks.
                let blocks = match message.blocks.deserialize().await {
                    Ok(blocks) => blocks,
                    Err(error) => bail!("[PuzzleResponse] {error}"),
                };

                // Ensure the blocks are not empty.
                ensure!(!blocks.is_empty(), "Peer '{peer_ip}' sent an empty block response (request = {request})");
                // Check that the blocks are sequentially ordered.
                if !blocks.windows(2).all(|w| w[0].height() + 1 == w[1].height()) {
                    bail!("Peer '{peer_ip}' sent an invalid block response (blocks are not sequentially ordered)")
                }

                // Retrieve the start (inclusive) and end (exclusive) block height.
                let start_height = blocks.first().map(|b| b.height()).unwrap_or(0);
                let end_height = 1 + blocks.last().map(|b| b.height()).unwrap_or(0);
                // Check that the range matches the block request.
                if start_height != request.start_height || end_height != request.end_height {
                    bail!("Peer '{peer_ip}' sent an invalid block response (range does not match the block request)")
                }

                // Process the block response.
                match self.block_response(peer_ip, blocks.0) {
                    true => Ok(()),
                    false => bail!("Peer '{peer_ip}' sent an invalid block response"),
                }
            }
            Message::ChallengeRequest(..) | Message::ChallengeResponse(..) => {
                // Disconnect as the peer is not following the protocol.
                bail!("Peer '{peer_ip}' is not following the protocol")
            }
            Message::Disconnect(message) => {
                bail!("Disconnecting peer '{peer_ip}' for the following reason: {:?}", message.reason)
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
                false => bail!("Peer '{peer_ip}' sent an invalid ping"),
            },
            Message::Pong(message) => match self.pong(peer_ip, message) {
                true => Ok(()),
                false => bail!("Peer '{peer_ip}' sent an invalid pong"),
            },
            Message::PuzzleRequest(..) => {
                // Insert the puzzle request for the peer, and fetch the recent frequency.
                let frequency = self.router().cache.insert_inbound_puzzle_request(peer_ip);
                // Check if the number of puzzle requests is within the limit.
                if frequency > Self::MAXIMUM_PUZZLE_REQUESTS_PER_INTERVAL {
                    bail!("Peer '{peer_ip}' is not following the protocol (excessive puzzle requests)")
                }
                // Process the puzzle request.
                match self.puzzle_request(peer_ip) {
                    true => Ok(()),
                    false => bail!("Peer '{peer_ip}' sent an invalid puzzle request"),
                }
            }
            Message::PuzzleResponse(message) => {
                // Check that this node previously sent a puzzle request to this peer.
                if !self.router().cache.contains_outbound_puzzle_request(&peer_ip) {
                    bail!("Peer '{peer_ip}' is not following the protocol (unexpected puzzle response)")
                }
                // Decrement the number of puzzle requests.
                self.router().cache.decrement_outbound_puzzle_requests(peer_ip);

                // Clone the serialized message.
                let serialized = message.clone();
                // Perform the deferred non-blocking deserialization of the block.
                let block = match message.block.deserialize().await {
                    Ok(block) => block,
                    Err(error) => bail!("[PuzzleResponse] {error}"),
                };
                // Process the puzzle response.
                match self.puzzle_response(peer_ip, serialized, block) {
                    true => Ok(()),
                    false => bail!("Peer '{peer_ip}' sent an invalid puzzle response"),
                }
            }
            Message::UnconfirmedBlock(message) => {
                // Clone the serialized message.
                let serialized = message.clone();
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
                    bail!("Peer '{peer_ip}' is not following the 'UnconfirmedBlock' protocol")
                }
                // Handle the unconfirmed block.
                match self.unconfirmed_block(peer_ip, serialized, block) {
                    true => Ok(()),
                    false => bail!("Peer '{peer_ip}' sent an invalid unconfirmed block"),
                }
            }
            Message::UnconfirmedSolution(message) => {
                // Clone the serialized message.
                let serialized = message.clone();
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
                    bail!("Peer '{peer_ip}' is not following the 'UnconfirmedSolution' protocol")
                }
                // Handle the unconfirmed solution.
                match self.unconfirmed_solution(peer_ip, serialized, solution).await {
                    true => Ok(()),
                    false => bail!("Peer '{peer_ip}' sent an invalid unconfirmed solution"),
                }
            }
            Message::UnconfirmedTransaction(message) => {
                // Clone the serialized message.
                let serialized = message.clone();
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
                    bail!("Peer '{peer_ip}' is not following the 'UnconfirmedTransaction' protocol")
                }
                // Handle the unconfirmed transaction.
                match self.unconfirmed_transaction(peer_ip, serialized, transaction) {
                    true => Ok(()),
                    false => bail!("Peer '{peer_ip}' sent an invalid unconfirmed transaction"),
                }
            }
        }
    }

    /// Handles a `BlockRequest` message.
    fn block_request(&self, peer_ip: SocketAddr, _message: BlockRequest) -> bool;

    /// Handles a `BlockResponse` message.
    fn block_response(&self, peer_ip: SocketAddr, _blocks: Vec<Block<N>>) -> bool;

    fn ping(&self, peer_ip: SocketAddr, message: Ping<N>) -> bool {
        // Ensure the message protocol version is not outdated.
        if message.version < Message::<N>::VERSION {
            warn!("Dropping '{peer_ip}' on version {} (outdated)", message.version);
            return false;
        }
        // Ensure the maximum fork depth is correct.
        if message.fork_depth != ALEO_MAXIMUM_FORK_DEPTH {
            warn!("Dropping '{peer_ip}' for an incorrect maximum fork depth of {}", message.fork_depth);
            return false;
        }
        // Ensure the peer status is known.
        if message.status.is_unknown() {
            warn!("Dropping '{peer_ip}' for an unknown status");
            return false;
        }

        // // If this node is not a sync node and is syncing, the peer is a sync node, and this node is ahead, proceed to disconnect.
        // if E::NODE_TYPE != NodeType::Beacon
        //     && E::status().is_syncing()
        //     && node_type == NodeType::Beacon
        //     && state.ledger().reader().latest_cumulative_weight() > block_header.cumulative_weight()
        // {
        //     trace!("Disconnecting from {} (ahead of sync node)", peer_ip);
        //     break;
        // }

        // If the peer is a beacon or validator, ensure there are block locators.
        if (message.node_type.is_beacon() || message.node_type.is_validator()) && message.block_locators.is_none() {
            warn!("Peer '{peer_ip}' is a beacon or validator, but no block locators were provided");
            return false;
        }
        // If the peer is a prover or client, ensure there are no block locators.
        if (message.node_type.is_prover() || message.node_type.is_client()) && message.block_locators.is_some() {
            warn!("Peer '{peer_ip}' is a prover or client, but block locators were provided");
            return false;
        }
        // If block locators were provided, then update the peer in the sync pool.
        if let Some(block_locators) = message.block_locators {
            // Check the block locators are valid, and update the peer in the sync pool.
            if let Err(disconnect_peers) = self.router().sync().update_locators(peer_ip, block_locators) {
                // On failure, disconnect the returned peer IPs.
                for disconnect_ip in disconnect_peers {
                    debug!(
                        "Disconnecting '{disconnect_ip}' for the following reason - {:?}",
                        DisconnectReason::ProtocolViolation
                    );
                    self.router().disconnect(disconnect_ip);
                }
                return false;
            }
        }

        // Update the connected peer.
        if let Err(error) = self.router().update_connected_peer(peer_ip, message.node_type, |peer: &mut Peer<N>| {
            // Update the version of the peer.
            peer.set_version(message.version);
            // Update the node type of the peer.
            peer.set_node_type(message.node_type);
            // Update the status of the peer.
            peer.set_status(RawStatus::from_status(message.status));
            // Update the last seen timestamp of the peer.
            peer.set_last_seen(Instant::now());
        }) {
            warn!("[Ping] {error}");
            return false;
        }

        let is_fork = Some(false);

        // Send a `Pong` message to the peer.
        self.send(peer_ip, Message::Pong(Pong { is_fork }));
        true
    }

    /// Sleeps for a period and then sends a `Ping` message to the peer.
    fn pong(&self, peer_ip: SocketAddr, _message: Pong) -> bool;

    fn puzzle_request(&self, peer_ip: SocketAddr) -> bool {
        debug!("Disconnecting '{peer_ip}' for the following reason - {:?}", DisconnectReason::ProtocolViolation);
        false
    }

    fn puzzle_response(&self, peer_ip: SocketAddr, _serialized: PuzzleResponse<N>, _block: Block<N>) -> bool {
        debug!("Disconnecting '{peer_ip}' for the following reason - {:?}", DisconnectReason::ProtocolViolation);
        false
    }

    /// TODO (howardwu): Consider deprecating this message type.
    /// Broadcasts the `UnconfirmedBlock` message to all connected peers within the fork depth of the given block.
    fn unconfirmed_block(&self, peer_ip: SocketAddr, serialized: UnconfirmedBlock<N>, block: Block<N>) -> bool {
        // Retrieve the connected peers by height.
        let mut peers = self.router().sync().get_peers_by_height();
        // Retain the peers that 1) not the sender, and 2) are within the fork depth of the given unconfirmed block.
        peers.retain(|(ip, height)| *ip != peer_ip && *height < block.height() + ALEO_MAXIMUM_FORK_DEPTH);

        // Broadcast the `UnconfirmedBlock` to the peers.
        if !peers.is_empty() {
            for (peer_ip, _) in peers {
                self.send(peer_ip, Message::UnconfirmedBlock(serialized.clone()));
            }
        }
        true
    }

    /// Handles an `UnconfirmedSolution` message.
    async fn unconfirmed_solution(
        &self,
        peer_ip: SocketAddr,
        serialized: UnconfirmedSolution<N>,
        solution: ProverSolution<N>,
    ) -> bool;

    fn unconfirmed_transaction(
        &self,
        peer_ip: SocketAddr,
        serialized: UnconfirmedTransaction<N>,
        _transaction: Transaction<N>,
    ) -> bool {
        // Propagate the `UnconfirmedTransaction`.
        self.propagate(Message::UnconfirmedTransaction(serialized), vec![peer_ip]);
        true
    }
}
