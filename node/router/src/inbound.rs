// Copyright 2024 Aleo Network Foundation
// This file is part of the snarkOS library.

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at:

// http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::{
    messages::{
        BlockRequest,
        BlockResponse,
        DataBlocks,
        Message,
        PeerResponse,
        Ping,
        Pong,
        UnconfirmedSolution,
        UnconfirmedTransaction,
    },
    Outbound,
    Peer,
};
use snarkos_node_tcp::protocols::Reading;
use snarkvm::prelude::{
    block::{Block, Header, Transaction},
    puzzle::Solution,
    Network,
};

use anyhow::{anyhow, bail, Result};
use snarkos_node_tcp::is_bogon_ip;
use std::net::SocketAddr;
use tokio::task::spawn_blocking;

/// The max number of peers to send in a `PeerResponse` message.
const MAX_PEERS_TO_SEND: usize = u8::MAX as usize;

/// The maximum number of blocks the client can be behind it's latest peer before it skips
/// processing incoming transactions and solutions.
pub const SYNC_LENIENCY: u32 = 10;

#[async_trait]
pub trait Inbound<N: Network>: Reading + Outbound<N> {
    /// The maximum number of puzzle requests per interval.
    const MAXIMUM_PUZZLE_REQUESTS_PER_INTERVAL: usize = 5;
    /// The maximum number of block requests per interval.
    const MAXIMUM_BLOCK_REQUESTS_PER_INTERVAL: usize = 256;
    /// The duration in seconds to sleep in between ping requests with a connected peer.
    const PING_SLEEP_IN_SECS: u64 = 20; // 20 seconds
    /// The time frame to enforce the `MESSAGE_LIMIT`.
    const MESSAGE_LIMIT_TIME_FRAME_IN_SECS: i64 = 5;
    /// The maximum number of messages accepted within `MESSAGE_LIMIT_TIME_FRAME_IN_SECS`.
    const MESSAGE_LIMIT: usize = 500;

    /// Handles the inbound message from the peer.
    async fn inbound(&self, peer_addr: SocketAddr, message: Message<N>) -> Result<()> {
        // Retrieve the listener IP for the peer.
        let peer_ip = match self.router().resolve_to_listener(&peer_addr) {
            Some(peer_ip) => peer_ip,
            None => bail!("Unable to resolve the (ambiguous) peer address '{peer_addr}'"),
        };

        // Drop the peer, if they have sent more than `MESSAGE_LIMIT` messages
        // in the last `MESSAGE_LIMIT_TIME_FRAME_IN_SECS` seconds.
        let num_messages = self.router().cache.insert_inbound_message(peer_ip, Self::MESSAGE_LIMIT_TIME_FRAME_IN_SECS);
        if num_messages > Self::MESSAGE_LIMIT {
            bail!("Dropping '{peer_ip}' for spamming messages (num_messages = {num_messages})")
        }

        trace!("Received '{}' from '{peer_ip}'", message.name());

        // Update the last seen timestamp of the peer.
        self.router().update_last_seen_for_connected_peer(peer_ip);

        // This match statement handles the inbound message by deserializing the message,
        // checking that the message is valid, and then calling the appropriate (trait) handler.
        match message {
            Message::BlockRequest(message) => {
                let BlockRequest { start_height, end_height } = &message;
                // Insert the block request for the peer, and fetch the recent frequency.
                let frequency = self.router().cache.insert_inbound_block_request(peer_ip);
                // Check if the number of block requests is within the limit.
                if frequency > Self::MAXIMUM_BLOCK_REQUESTS_PER_INTERVAL {
                    bail!("Peer '{peer_ip}' is not following the protocol (excessive block requests)")
                }
                // Ensure the block request is well-formed.
                if start_height >= end_height {
                    bail!("Block request from '{peer_ip}' has an invalid range ({start_height}..{end_height})")
                }
                // Ensure that the block request is within the allowed bounds.
                if end_height - start_height > DataBlocks::<N>::MAXIMUM_NUMBER_OF_BLOCKS as u32 {
                    bail!("Block request from '{peer_ip}' has an excessive range ({start_height}..{end_height})")
                }

                let node = self.clone();
                match spawn_blocking(move || node.block_request(peer_ip, message)).await? {
                    true => Ok(()),
                    false => bail!("Peer '{peer_ip}' sent an invalid block request"),
                }
            }
            Message::BlockResponse(message) => {
                let BlockResponse { request, blocks } = message;

                // Remove the block request, checking if this node previously sent a block request to this peer.
                if !self.router().cache.remove_outbound_block_request(peer_ip, &request) {
                    bail!("Peer '{peer_ip}' is not following the protocol (unexpected block response)")
                }
                // Perform the deferred non-blocking deserialization of the blocks.
                // The deserialization can take a long time (minutes). We should not be running
                // this on a blocking task, but on a rayon thread pool.
                let (send, recv) = tokio::sync::oneshot::channel();
                rayon::spawn_fifo(move || {
                    let blocks = blocks.deserialize_blocking().map_err(|error| anyhow!("[BlockResponse] {error}"));
                    let _ = send.send(blocks);
                });
                let blocks = match recv.await {
                    Ok(Ok(blocks)) => blocks,
                    Ok(Err(error)) => bail!("Peer '{peer_ip}' sent an invalid block response - {error}"),
                    Err(error) => bail!("Peer '{peer_ip}' sent an invalid block response - {error}"),
                };

                // Ensure the block response is well-formed.
                blocks.ensure_response_is_well_formed(peer_ip, request.start_height, request.end_height)?;

                // Process the block response.
                let node = self.clone();
                match spawn_blocking(move || node.block_response(peer_ip, blocks.0)).await? {
                    true => Ok(()),
                    false => bail!("Peer '{peer_ip}' sent an invalid block response"),
                }
            }
            Message::ChallengeRequest(..) | Message::ChallengeResponse(..) => {
                // Disconnect as the peer is not following the protocol.
                bail!("Peer '{peer_ip}' is not following the protocol")
            }
            Message::Disconnect(message) => {
                bail!("{:?}", message.reason)
            }
            Message::PeerRequest(..) => match self.peer_request(peer_ip) {
                true => Ok(()),
                false => bail!("Peer '{peer_ip}' sent an invalid peer request"),
            },
            Message::PeerResponse(message) => {
                if !self.router().cache.contains_outbound_peer_request(peer_ip) {
                    bail!("Peer '{peer_ip}' is not following the protocol (unexpected peer response)")
                }
                self.router().cache.decrement_outbound_peer_requests(peer_ip);
                if !self.router().allow_external_peers() {
                    bail!("Not accepting peer response from '{peer_ip}' (validator gossip is disabled)");
                }

                match self.peer_response(peer_ip, &message.peers) {
                    true => Ok(()),
                    false => bail!("Peer '{peer_ip}' sent an invalid peer response"),
                }
            }
            Message::Ping(message) => {
                // Ensure the message protocol version is not outdated.
                if message.version < Message::<N>::VERSION {
                    bail!("Dropping '{peer_ip}' on message version {} (outdated)", message.version);
                }

                // If the peer is a client or validator, ensure there are block locators.
                let is_client_or_validator = message.node_type.is_client() || message.node_type.is_validator();
                if is_client_or_validator && message.block_locators.is_none() {
                    bail!("Peer '{peer_ip}' is a {}, but no block locators were provided", message.node_type);
                }
                // If the peer is a prover, ensure there are no block locators.
                else if message.node_type.is_prover() && message.block_locators.is_some() {
                    bail!("Peer '{peer_ip}' is a prover or client, but block locators were provided");
                }

                // Update the connected peer.
                if let Err(error) =
                    self.router().update_connected_peer(peer_ip, message.node_type, |peer: &mut Peer<N>| {
                        // Update the version of the peer.
                        peer.set_version(message.version);
                        // Update the node type of the peer.
                        peer.set_node_type(message.node_type);
                    })
                {
                    bail!("[Ping] {error}");
                }

                // Process the ping message.
                match self.ping(peer_ip, message) {
                    true => Ok(()),
                    false => bail!("Peer '{peer_ip}' sent an invalid ping"),
                }
            }
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

                // Perform the deferred non-blocking deserialization of the block header.
                let header = match message.block_header.deserialize().await {
                    Ok(header) => header,
                    Err(error) => bail!("[PuzzleResponse] {error}"),
                };
                // Process the puzzle response.
                match self.puzzle_response(peer_ip, message.epoch_hash, header) {
                    true => Ok(()),
                    false => bail!("Peer '{peer_ip}' sent an invalid puzzle response"),
                }
            }
            Message::UnconfirmedSolution(message) => {
                // Do not process unconfirmed solutions if the node is too far behind.
                if self.num_blocks_behind() > SYNC_LENIENCY {
                    trace!("Skipped processing unconfirmed solution '{}' (node is syncing)", message.solution_id);
                    return Ok(());
                }
                // Update the timestamp for the unconfirmed solution.
                let seen_before = self.router().cache.insert_inbound_solution(peer_ip, message.solution_id).is_some();
                // Determine whether to propagate the solution.
                if seen_before {
                    trace!("Skipping 'UnconfirmedSolution' from '{peer_ip}'");
                    return Ok(());
                }
                // Clone the serialized message.
                let serialized = message.clone();
                // Perform the deferred non-blocking deserialization of the solution.
                let solution = match message.solution.deserialize().await {
                    Ok(solution) => solution,
                    Err(error) => bail!("[UnconfirmedSolution] {error}"),
                };
                // Check that the solution parameters match.
                if message.solution_id != solution.id() {
                    bail!("Peer '{peer_ip}' is not following the 'UnconfirmedSolution' protocol")
                }
                // Handle the unconfirmed solution.
                match self.unconfirmed_solution(peer_ip, serialized, solution).await {
                    true => Ok(()),
                    false => bail!("Peer '{peer_ip}' sent an invalid unconfirmed solution"),
                }
            }
            Message::UnconfirmedTransaction(message) => {
                // Do not process unconfirmed transactions if the node is too far behind.
                if self.num_blocks_behind() > SYNC_LENIENCY {
                    trace!("Skipped processing unconfirmed transaction '{}' (node is syncing)", message.transaction_id);
                    return Ok(());
                }
                // Update the timestamp for the unconfirmed transaction.
                let seen_before =
                    self.router().cache.insert_inbound_transaction(peer_ip, message.transaction_id).is_some();
                // Determine whether to propagate the transaction.
                if seen_before {
                    trace!("Skipping 'UnconfirmedTransaction' from '{peer_ip}'");
                    return Ok(());
                }
                // Clone the serialized message.
                let serialized = message.clone();
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
                match self.unconfirmed_transaction(peer_ip, serialized, transaction).await {
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

    /// Handles a `PeerRequest` message.
    fn peer_request(&self, peer_ip: SocketAddr) -> bool {
        // Retrieve the connected peers.
        let peers = self.router().connected_peers();
        // Filter out invalid addresses.
        let peers = match self.router().is_dev() {
            // In development mode, relax the validity requirements to make operating devnets more flexible.
            true => {
                peers.into_iter().filter(|ip| *ip != peer_ip && !is_bogon_ip(ip.ip())).take(MAX_PEERS_TO_SEND).collect()
            }
            // In production mode, ensure the peer IPs are valid.
            false => peers
                .into_iter()
                .filter(|ip| *ip != peer_ip && self.router().is_valid_peer_ip(ip))
                .take(MAX_PEERS_TO_SEND)
                .collect(),
        };
        // Send a `PeerResponse` message to the peer.
        self.send(peer_ip, Message::PeerResponse(PeerResponse { peers }));
        true
    }

    /// Handles a `PeerResponse` message.
    fn peer_response(&self, _peer_ip: SocketAddr, peers: &[SocketAddr]) -> bool {
        // Check if the number of peers received is less than MAX_PEERS_TO_SEND.
        if peers.len() > MAX_PEERS_TO_SEND {
            return false;
        }
        // Filter out invalid addresses.
        let peers = match self.router().is_dev() {
            // In development mode, relax the validity requirements to make operating devnets more flexible.
            true => peers.iter().copied().filter(|ip| !is_bogon_ip(ip.ip())).collect::<Vec<_>>(),
            // In production mode, ensure the peer IPs are valid.
            false => peers.iter().copied().filter(|ip| self.router().is_valid_peer_ip(ip)).collect(),
        };
        // Adds the given peer IPs to the list of candidate peers.
        self.router().insert_candidate_peers(&peers);
        true
    }

    /// Handles a `Ping` message.
    fn ping(&self, peer_ip: SocketAddr, message: Ping<N>) -> bool;

    /// Sleeps for a period and then sends a `Ping` message to the peer.
    fn pong(&self, peer_ip: SocketAddr, _message: Pong) -> bool;

    /// Handles a `PuzzleRequest` message.
    fn puzzle_request(&self, peer_ip: SocketAddr) -> bool;

    /// Handles a `PuzzleResponse` message.
    fn puzzle_response(&self, peer_ip: SocketAddr, _epoch_hash: N::BlockHash, _header: Header<N>) -> bool;

    /// Handles an `UnconfirmedSolution` message.
    async fn unconfirmed_solution(
        &self,
        peer_ip: SocketAddr,
        serialized: UnconfirmedSolution<N>,
        solution: Solution<N>,
    ) -> bool;

    /// Handles an `UnconfirmedTransaction` message.
    async fn unconfirmed_transaction(
        &self,
        peer_ip: SocketAddr,
        serialized: UnconfirmedTransaction<N>,
        _transaction: Transaction<N>,
    ) -> bool;
}
