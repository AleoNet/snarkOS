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

use super::*;

use snarkos_node_router::messages::{
    BlockRequest,
    DisconnectReason,
    Message,
    MessageCodec,
    Ping,
    Pong,
    PuzzleRequest,
    UnconfirmedTransaction,
};
use snarkos_node_tcp::{Connection, ConnectionSide, Tcp};
use snarkvm::prelude::{block::Transaction, Field, Network, Zero};

use std::{io, net::SocketAddr};

impl<N: Network, C: ConsensusStorage<N>> P2P for Prover<N, C> {
    /// Returns a reference to the TCP instance.
    fn tcp(&self) -> &Tcp {
        self.router.tcp()
    }
}

#[async_trait]
impl<N: Network, C: ConsensusStorage<N>> Handshake for Prover<N, C> {
    /// Performs the handshake protocol.
    async fn perform_handshake(&self, mut connection: Connection) -> io::Result<Connection> {
        // Perform the handshake.
        let peer_addr = connection.addr();
        let conn_side = connection.side();
        let stream = self.borrow_stream(&mut connection);
        let genesis_header = *self.genesis.header();
        let restrictions_id = Field::zero(); // Provers may bypass restrictions, since they do not validate transactions.
        self.router.handshake(peer_addr, stream, conn_side, genesis_header, restrictions_id).await?;

        Ok(connection)
    }
}

#[async_trait]
impl<N: Network, C: ConsensusStorage<N>> OnConnect for Prover<N, C>
where
    Self: Outbound<N>,
{
    async fn on_connect(&self, peer_addr: SocketAddr) {
        // Resolve the peer address to the listener address.
        let Some(peer_ip) = self.router.resolve_to_listener(&peer_addr) else { return };
        // Send the first `Ping` message to the peer.
        self.send_ping(peer_ip, None);
    }
}

#[async_trait]
impl<N: Network, C: ConsensusStorage<N>> Disconnect for Prover<N, C> {
    /// Any extra operations to be performed during a disconnect.
    async fn handle_disconnect(&self, peer_addr: SocketAddr) {
        if let Some(peer_ip) = self.router.resolve_to_listener(&peer_addr) {
            self.sync.remove_peer(&peer_ip);
            self.router.remove_connected_peer(peer_ip);
        }
    }
}

#[async_trait]
impl<N: Network, C: ConsensusStorage<N>> Writing for Prover<N, C> {
    type Codec = MessageCodec<N>;
    type Message = Message<N>;

    /// Creates an [`Encoder`] used to write the outbound messages to the target stream.
    /// The `side` parameter indicates the connection side **from the node's perspective**.
    fn codec(&self, _addr: SocketAddr, _side: ConnectionSide) -> Self::Codec {
        Default::default()
    }
}

#[async_trait]
impl<N: Network, C: ConsensusStorage<N>> Reading for Prover<N, C> {
    type Codec = MessageCodec<N>;
    type Message = Message<N>;

    /// Creates a [`Decoder`] used to interpret messages from the network.
    /// The `side` param indicates the connection side **from the node's perspective**.
    fn codec(&self, _peer_addr: SocketAddr, _side: ConnectionSide) -> Self::Codec {
        Default::default()
    }

    /// Processes a message received from the network.
    async fn process_message(&self, peer_addr: SocketAddr, message: Self::Message) -> io::Result<()> {
        // Process the message. Disconnect if the peer violated the protocol.
        if let Err(error) = self.inbound(peer_addr, message).await {
            if let Some(peer_ip) = self.router().resolve_to_listener(&peer_addr) {
                warn!("Disconnecting from '{peer_addr}' - {error}");
                Outbound::send(self, peer_ip, Message::Disconnect(DisconnectReason::ProtocolViolation.into()));
                // Disconnect from this peer.
                self.router().disconnect(peer_ip);
            }
        }
        Ok(())
    }
}

#[async_trait]
impl<N: Network, C: ConsensusStorage<N>> Routing<N> for Prover<N, C> {}

impl<N: Network, C: ConsensusStorage<N>> Heartbeat<N> for Prover<N, C> {
    /// This function updates the puzzle if network has updated.
    fn handle_puzzle_request(&self) {
        // Find the sync peers.
        if let Some((sync_peers, _)) = self.sync.find_sync_peers() {
            // Choose the peer with the highest block height.
            if let Some((peer_ip, _)) = sync_peers.into_iter().max_by_key(|(_, height)| *height) {
                // Request the puzzle from the peer.
                Outbound::send(self, peer_ip, Message::PuzzleRequest(PuzzleRequest));
            }
        }
    }
}

impl<N: Network, C: ConsensusStorage<N>> Outbound<N> for Prover<N, C> {
    /// Returns a reference to the router.
    fn router(&self) -> &Router<N> {
        &self.router
    }

    /// Returns `true` if the node is synced up to the latest block (within the given tolerance).
    fn is_block_synced(&self) -> bool {
        true
    }

    /// Returns the number of blocks this node is behind the greatest peer height.
    fn num_blocks_behind(&self) -> u32 {
        0
    }
}

#[async_trait]
impl<N: Network, C: ConsensusStorage<N>> Inbound<N> for Prover<N, C> {
    /// Handles a `BlockRequest` message.
    fn block_request(&self, peer_ip: SocketAddr, _message: BlockRequest) -> bool {
        debug!("Disconnecting '{peer_ip}' for the following reason - {:?}", DisconnectReason::ProtocolViolation);
        false
    }

    /// Handles a `BlockResponse` message.
    fn block_response(&self, peer_ip: SocketAddr, _blocks: Vec<Block<N>>) -> bool {
        debug!("Disconnecting '{peer_ip}' for the following reason - {:?}", DisconnectReason::ProtocolViolation);
        false
    }

    /// Processes the block locators and sends back a `Pong` message.
    fn ping(&self, peer_ip: SocketAddr, message: Ping<N>) -> bool {
        // Check if the sync module is in router mode.
        if self.sync.mode().is_router() {
            // If block locators were provided, then update the peer in the sync pool.
            if let Some(block_locators) = message.block_locators {
                // Check the block locators are valid, and update the peer in the sync pool.
                if let Err(error) = self.sync.update_peer_locators(peer_ip, block_locators) {
                    warn!("Peer '{peer_ip}' sent invalid block locators: {error}");
                    return false;
                }
            }
        }

        // Send a `Pong` message to the peer.
        Outbound::send(self, peer_ip, Message::Pong(Pong { is_fork: Some(false) }));
        true
    }

    /// Sleeps for a period and then sends a `Ping` message to the peer.
    fn pong(&self, peer_ip: SocketAddr, _message: Pong) -> bool {
        // Spawn an asynchronous task for the `Ping` request.
        let self_clone = self.clone();
        tokio::spawn(async move {
            // Sleep for the preset time before sending a `Ping` request.
            tokio::time::sleep(Duration::from_secs(Self::PING_SLEEP_IN_SECS)).await;
            // Check that the peer is still connected.
            if self_clone.router().is_connected(&peer_ip) {
                // Send a `Ping` message to the peer.
                self_clone.send_ping(peer_ip, None);
            }
        });
        true
    }

    /// Disconnects on receipt of a `PuzzleRequest` message.
    fn puzzle_request(&self, peer_ip: SocketAddr) -> bool {
        debug!("Disconnecting '{peer_ip}' for the following reason - {:?}", DisconnectReason::ProtocolViolation);
        false
    }

    /// Saves the latest epoch hash and latest block header in the node.
    fn puzzle_response(&self, peer_ip: SocketAddr, epoch_hash: N::BlockHash, header: Header<N>) -> bool {
        // Retrieve the block height.
        let block_height = header.height();

        info!(
            "Puzzle (Block {block_height}, Coinbase Target {}, Proof Target {})",
            header.coinbase_target(),
            header.proof_target()
        );

        // Save the latest epoch hash in the node.
        self.latest_epoch_hash.write().replace(epoch_hash);
        // Save the latest block header in the node.
        self.latest_block_header.write().replace(header);

        trace!("Received 'PuzzleResponse' from '{peer_ip}' (Block {block_height})");
        true
    }

    /// Propagates the unconfirmed solution to all connected validators.
    async fn unconfirmed_solution(
        &self,
        peer_ip: SocketAddr,
        serialized: UnconfirmedSolution<N>,
        solution: Solution<N>,
    ) -> bool {
        // Retrieve the latest epoch hash.
        let epoch_hash = *self.latest_epoch_hash.read();
        // Retrieve the latest proof target.
        let proof_target = self.latest_block_header.read().as_ref().map(|header| header.proof_target());

        if let (Some(epoch_hash), Some(proof_target)) = (epoch_hash, proof_target) {
            // Ensure that the solution is valid for the given epoch.
            let puzzle = self.puzzle.clone();
            let is_valid =
                tokio::task::spawn_blocking(move || puzzle.check_solution(&solution, epoch_hash, proof_target)).await;

            match is_valid {
                // If the solution is valid, propagate the `UnconfirmedSolution`.
                Ok(Ok(())) => {
                    let message = Message::UnconfirmedSolution(serialized);
                    // Propagate the "UnconfirmedSolution".
                    self.propagate(message, &[peer_ip]);
                }
                Ok(Err(_)) => {
                    trace!("Invalid solution '{}' for the proof target.", solution.id())
                }
                // If error occurs after the first 10 blocks of the epoch, log it as a warning, otherwise ignore.
                Err(error) => {
                    if let Some(height) = self.latest_block_header.read().as_ref().map(|header| header.height()) {
                        if height % N::NUM_BLOCKS_PER_EPOCH > 10 {
                            warn!("Failed to verify the solution - {error}")
                        }
                    }
                }
            }
        }
        true
    }

    /// Handles an `UnconfirmedTransaction` message.
    async fn unconfirmed_transaction(
        &self,
        _peer_ip: SocketAddr,
        _serialized: UnconfirmedTransaction<N>,
        _transaction: Transaction<N>,
    ) -> bool {
        true
    }
}
