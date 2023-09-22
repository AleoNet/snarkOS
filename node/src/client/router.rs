// Copyright (C) 2019-2023 Aleo Systems Inc.
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
    messages::{BlockRequest, DisconnectReason, MessageCodec, Ping, Pong, UnconfirmedTransaction},
    Routing,
};
use snarkos_node_tcp::{Connection, ConnectionSide, Tcp};
use snarkvm::prelude::{block::Transaction, Network};

use snarkos_node_sync::communication_service::CommunicationService;
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
        self.router.handshake(peer_addr, stream, conn_side, genesis_header).await?;

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
        // Process the message. Disconnect if the peer violated the protocol.
        if let Err(error) = self.inbound(peer_addr, message).await {
            if let Some(peer_ip) = self.router().resolve_to_listener(&peer_addr) {
                warn!("Disconnecting from '{peer_ip}' - {error}");
                Outbound::send(self, peer_ip, Message::Disconnect(DisconnectReason::ProtocolViolation.into()));
                // Disconnect from this peer.
                self.router().disconnect(peer_ip);
            }
        }
        Ok(())
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
}

#[async_trait]
impl<N: Network, C: ConsensusStorage<N>> Inbound<N> for Client<N, C> {
    /// Handles a `BlockRequest` message.
    fn block_request(&self, peer_ip: SocketAddr, _message: BlockRequest) -> bool {
        debug!("Disconnecting '{peer_ip}' for the following reason - {:?}", DisconnectReason::ProtocolViolation);
        false
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

    /// Disconnects on receipt of a `PuzzleRequest` message.
    fn puzzle_request(&self, peer_ip: SocketAddr) -> bool {
        debug!("Disconnecting '{peer_ip}' for the following reason - {:?}", DisconnectReason::ProtocolViolation);
        false
    }

    /// Saves the latest epoch challenge and latest block header in the node.
    fn puzzle_response(&self, peer_ip: SocketAddr, epoch_challenge: EpochChallenge<N>, header: Header<N>) -> bool {
        // Retrieve the epoch number.
        let epoch_number = epoch_challenge.epoch_number();
        // Retrieve the block height.
        let block_height = header.height();

        info!(
            "Coinbase Puzzle (Epoch {epoch_number}, Block {block_height}, Coinbase Target {}, Proof Target {})",
            header.coinbase_target(),
            header.proof_target()
        );

        // Save the latest epoch challenge in the node.
        self.latest_epoch_challenge.write().replace(epoch_challenge);
        // Save the latest block header in the node.
        self.latest_block_header.write().replace(header);

        trace!("Received 'PuzzleResponse' from '{peer_ip}' (Epoch {epoch_number}, Block {block_height})");
        true
    }

    /// Propagates the unconfirmed solution to all connected validators.
    async fn unconfirmed_solution(
        &self,
        peer_ip: SocketAddr,
        serialized: UnconfirmedSolution<N>,
        solution: ProverSolution<N>,
    ) -> bool {
        // Retrieve the latest epoch challenge.
        let epoch_challenge = self.latest_epoch_challenge.read().clone();
        // Retrieve the latest proof target.
        let proof_target = self.latest_block_header.read().as_ref().map(|header| header.proof_target());

        if let (Some(epoch_challenge), Some(proof_target)) = (epoch_challenge, proof_target) {
            // Ensure that the prover solution is valid for the given epoch.
            let coinbase_puzzle = self.coinbase_puzzle.clone();
            let is_valid = tokio::task::spawn_blocking(move || {
                solution.verify(coinbase_puzzle.coinbase_verifying_key(), &epoch_challenge, proof_target)
            })
            .await;

            match is_valid {
                // If the solution is valid, propagate the `UnconfirmedSolution`.
                Ok(Ok(true)) => {
                    let message = Message::UnconfirmedSolution(serialized);
                    // Propagate the "UnconfirmedSolution" to the connected validators.
                    self.propagate_to_validators(message, &[peer_ip]);
                }
                Ok(Ok(false)) | Ok(Err(_)) => {
                    trace!("Invalid prover solution '{}' for the proof target.", solution.commitment())
                }
                Err(error) => warn!("Failed to verify the prover solution: {error}"),
            }
        }
        true
    }

    /// Handles an `UnconfirmedTransaction` message.
    async fn unconfirmed_transaction(
        &self,
        peer_ip: SocketAddr,
        serialized: UnconfirmedTransaction<N>,
        _transaction: Transaction<N>,
    ) -> bool {
        // Propagate the `UnconfirmedTransaction`.
        self.propagate(Message::UnconfirmedTransaction(serialized), &[peer_ip]);
        true
    }
}
