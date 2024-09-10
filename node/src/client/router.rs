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
use snarkos_node_router::{
    messages::{
        BlockRequest,
        BlockResponse,
        DataBlocks,
        DisconnectReason,
        MessageCodec,
        Ping,
        Pong,
        PuzzleResponse,
        UnconfirmedTransaction,
    },
    Routing,
};
use snarkos_node_sync::communication_service::CommunicationService;
use snarkos_node_tcp::{Connection, ConnectionSide, Tcp};
use snarkvm::{
    ledger::narwhal::Data,
    prelude::{block::Transaction, Network},
};

use std::{io, net::SocketAddr, time::Duration};

impl<N: Network, C: ConsensusStorage<N>> P2P for Client<N, C> {
    /// Returns a reference to the TCP instance.
    fn tcp(&self) -> &Tcp {
        self.router.tcp()
    }
}

#[async_trait]
impl<N: Network, C: ConsensusStorage<N>> Handshake for Client<N, C> {
    /// Performs the handshake protocol.
    async fn perform_handshake(&self, mut connection: Connection) -> io::Result<Connection> {
        // Perform the handshake.
        let peer_addr = connection.addr();
        let conn_side = connection.side();
        let stream = self.borrow_stream(&mut connection);
        let genesis_header = *self.genesis.header();
        let restrictions_id = self.ledger.vm().restrictions().restrictions_id();
        self.router.handshake(peer_addr, stream, conn_side, genesis_header, restrictions_id).await?;

        Ok(connection)
    }
}

#[async_trait]
impl<N: Network, C: ConsensusStorage<N>> OnConnect for Client<N, C>
where
    Self: Outbound<N>,
{
    async fn on_connect(&self, peer_addr: SocketAddr) {
        // Resolve the peer address to the listener address.
        let Some(peer_ip) = self.router.resolve_to_listener(&peer_addr) else { return };
        // Retrieve the block locators.
        let block_locators = match self.sync.get_block_locators() {
            Ok(block_locators) => Some(block_locators),
            Err(e) => {
                error!("Failed to get block locators: {e}");
                return;
            }
        };
        // Send the first `Ping` message to the peer.
        self.send_ping(peer_ip, block_locators);
    }
}

#[async_trait]
impl<N: Network, C: ConsensusStorage<N>> Disconnect for Client<N, C> {
    /// Any extra operations to be performed during a disconnect.
    async fn handle_disconnect(&self, peer_addr: SocketAddr) {
        if let Some(peer_ip) = self.router.resolve_to_listener(&peer_addr) {
            self.sync.remove_peer(&peer_ip);
            self.router.remove_connected_peer(peer_ip);
        }
    }
}

#[async_trait]
impl<N: Network, C: ConsensusStorage<N>> Writing for Client<N, C> {
    type Codec = MessageCodec<N>;
    type Message = Message<N>;

    /// Creates an [`Encoder`] used to write the outbound messages to the target stream.
    /// The `side` parameter indicates the connection side **from the node's perspective**.
    fn codec(&self, _addr: SocketAddr, _side: ConnectionSide) -> Self::Codec {
        Default::default()
    }
}

#[async_trait]
impl<N: Network, C: ConsensusStorage<N>> Reading for Client<N, C> {
    type Codec = MessageCodec<N>;
    type Message = Message<N>;

    /// Creates a [`Decoder`] used to interpret messages from the network.
    /// The `side` param indicates the connection side **from the node's perspective**.
    fn codec(&self, _peer_addr: SocketAddr, _side: ConnectionSide) -> Self::Codec {
        Default::default()
    }

    /// Processes a message received from the network.
    async fn process_message(&self, peer_addr: SocketAddr, message: Self::Message) -> io::Result<()> {
        let clone = self.clone();
        if matches!(message, Message::BlockRequest(_) | Message::BlockResponse(_)) {
            // Handle BlockRequest and BlockResponse messages in a separate task to not block the
            // inbound queue.
            tokio::spawn(async move {
                clone.process_message_inner(peer_addr, message).await;
            });
        } else {
            self.process_message_inner(peer_addr, message).await;
        }
        Ok(())
    }
}

impl<N: Network, C: ConsensusStorage<N>> Client<N, C> {
    async fn process_message_inner(
        &self,
        peer_addr: SocketAddr,
        message: <Client<N, C> as snarkos_node_tcp::protocols::Reading>::Message,
    ) {
        // Process the message. Disconnect if the peer violated the protocol.
        if let Err(error) = self.inbound(peer_addr, message).await {
            if let Some(peer_ip) = self.router().resolve_to_listener(&peer_addr) {
                warn!("Disconnecting from '{peer_ip}' - {error}");
                Outbound::send(self, peer_ip, Message::Disconnect(DisconnectReason::ProtocolViolation.into()));
                // Disconnect from this peer.
                self.router().disconnect(peer_ip);
            }
        }
    }
}

#[async_trait]
impl<N: Network, C: ConsensusStorage<N>> CommunicationService for Client<N, C> {
    /// The message type.
    type Message = Message<N>;

    /// Prepares a block request to be sent.
    fn prepare_block_request(start_height: u32, end_height: u32) -> Self::Message {
        debug_assert!(start_height < end_height, "Invalid block request format");
        Message::BlockRequest(BlockRequest { start_height, end_height })
    }

    /// Sends the given message to specified peer.
    ///
    /// This function returns as soon as the message is queued to be sent,
    /// without waiting for the actual delivery; instead, the caller is provided with a [`oneshot::Receiver`]
    /// which can be used to determine when and whether the message has been delivered.
    async fn send(
        &self,
        peer_ip: SocketAddr,
        message: Self::Message,
    ) -> Option<tokio::sync::oneshot::Receiver<io::Result<()>>> {
        Outbound::send(self, peer_ip, message)
    }
}

#[async_trait]
impl<N: Network, C: ConsensusStorage<N>> Routing<N> for Client<N, C> {}

impl<N: Network, C: ConsensusStorage<N>> Heartbeat<N> for Client<N, C> {}

impl<N: Network, C: ConsensusStorage<N>> Outbound<N> for Client<N, C> {
    /// Returns a reference to the router.
    fn router(&self) -> &Router<N> {
        &self.router
    }

    /// Returns `true` if the node is synced up to the latest block (within the given tolerance).
    fn is_block_synced(&self) -> bool {
        self.sync.is_block_synced()
    }

    /// Returns the number of blocks this node is behind the greatest peer height.
    fn num_blocks_behind(&self) -> u32 {
        self.sync.num_blocks_behind()
    }
}

#[async_trait]
impl<N: Network, C: ConsensusStorage<N>> Inbound<N> for Client<N, C> {
    /// Handles a `BlockRequest` message.
    fn block_request(&self, peer_ip: SocketAddr, message: BlockRequest) -> bool {
        let BlockRequest { start_height, end_height } = &message;

        // Retrieve the blocks within the requested range.
        let blocks = match self.ledger.get_blocks(*start_height..*end_height) {
            Ok(blocks) => Data::Object(DataBlocks(blocks)),
            Err(error) => {
                error!("Failed to retrieve blocks {start_height} to {end_height} from the ledger - {error}");
                return false;
            }
        };
        // Send the `BlockResponse` message to the peer.
        Outbound::send(self, peer_ip, Message::BlockResponse(BlockResponse { request: message, blocks }));
        true
    }

    /// Handles a `BlockResponse` message.
    fn block_response(&self, peer_ip: SocketAddr, blocks: Vec<Block<N>>) -> bool {
        // Tries to advance with blocks from the sync module.
        match self.sync.advance_with_sync_blocks(peer_ip, blocks) {
            Ok(()) => true,
            Err(error) => {
                warn!("{error}");
                false
            }
        }
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
        let self_ = self.clone();
        tokio::spawn(async move {
            // Sleep for the preset time before sending a `Ping` request.
            tokio::time::sleep(Duration::from_secs(Self::PING_SLEEP_IN_SECS)).await;
            // Check that the peer is still connected.
            if self_.router().is_connected(&peer_ip) {
                // Retrieve the block locators.
                match self_.sync.get_block_locators() {
                    // Send a `Ping` message to the peer.
                    Ok(block_locators) => self_.send_ping(peer_ip, Some(block_locators)),
                    Err(e) => error!("Failed to get block locators - {e}"),
                }
            }
        });
        true
    }

    /// Retrieves the latest epoch hash and latest block header, and returns the puzzle response to the peer.
    fn puzzle_request(&self, peer_ip: SocketAddr) -> bool {
        // Retrieve the latest epoch hash.
        let epoch_hash = match self.ledger.latest_epoch_hash() {
            Ok(epoch_hash) => epoch_hash,
            Err(error) => {
                error!("Failed to prepare a puzzle request for '{peer_ip}': {error}");
                return false;
            }
        };
        // Retrieve the latest block header.
        let block_header = Data::Object(self.ledger.latest_header());
        // Send the `PuzzleResponse` message to the peer.
        Outbound::send(self, peer_ip, Message::PuzzleResponse(PuzzleResponse { epoch_hash, block_header }));
        true
    }

    /// Saves the latest epoch hash and latest block header in the node.
    fn puzzle_response(&self, peer_ip: SocketAddr, _epoch_hash: N::BlockHash, _header: Header<N>) -> bool {
        debug!("Disconnecting '{peer_ip}' for the following reason - {:?}", DisconnectReason::ProtocolViolation);
        false
    }

    /// Propagates the unconfirmed solution to all connected validators.
    async fn unconfirmed_solution(
        &self,
        peer_ip: SocketAddr,
        serialized: UnconfirmedSolution<N>,
        solution: Solution<N>,
    ) -> bool {
        // Retrieve the latest epoch hash.
        if let Ok(epoch_hash) = self.ledger.latest_epoch_hash() {
            // Retrieve the latest proof target.
            let proof_target = self.ledger.latest_block().header().proof_target();
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
                    if self.ledger.latest_height() % N::NUM_BLOCKS_PER_EPOCH > 10 {
                        warn!("Failed to verify the solution - {error}")
                    }
                }
            }
        }
        true
    }

    /// Handles an `UnconfirmedTransaction` message.
    async fn unconfirmed_transaction(
        &self,
        peer_ip: SocketAddr,
        serialized: UnconfirmedTransaction<N>,
        transaction: Transaction<N>,
    ) -> bool {
        // Check that the transaction is not a fee transaction.
        if transaction.is_fee() {
            return true; // Maintain the connection.
        }
        // Check that the transaction is well-formed and unique.
        if self.ledger.check_transaction_basic(&transaction, None, &mut rand::thread_rng()).is_ok() {
            // Propagate the `UnconfirmedTransaction`.
            self.propagate(Message::UnconfirmedTransaction(serialized), &[peer_ip]);
        }
        true
    }
}
