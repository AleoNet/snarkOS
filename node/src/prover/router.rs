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

use snarkos_node_messages::{DisconnectReason, Message, MessageCodec};
use snarkos_node_tcp::{Connection, ConnectionSide, Tcp};
use snarkvm::prelude::Network;

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
        let peer_addr = connection.addr();
        let conn_side = connection.side();
        let stream = self.borrow_stream(&mut connection);
        self.router.handshake(peer_addr, stream, conn_side).await?;

        Ok(connection)
    }
}

#[async_trait]
impl<N: Network, C: ConsensusStorage<N>> Disconnect for Prover<N, C> {
    /// Any extra operations to be performed during a disconnect.
    async fn handle_disconnect(&self, peer_addr: SocketAddr) {
        self.router.remove_connected_peer(peer_addr);
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
    async fn process_message(&self, peer_ip: SocketAddr, message: Self::Message) -> io::Result<()> {
        // Process the message. Disconnect if the peer violated the protocol.
        if !self.inbound(peer_ip, message).await {
            warn!("Disconnecting from '{peer_ip}' (violated protocol)");
            self.send(peer_ip, Message::Disconnect(DisconnectReason::ProtocolViolation.into()));
            // Disconnect from this peer.
            self.router().disconnect(peer_ip).await;
        }
        Ok(())
    }
}

#[async_trait]
impl<N: Network, C: ConsensusStorage<N>> Routing<N> for Prover<N, C> {}

#[async_trait]
impl<N: Network, C: ConsensusStorage<N>> Heartbeat<N> for Prover<N, C> {}

impl<N: Network, C: ConsensusStorage<N>> Outbound<N> for Prover<N, C> {
    /// Returns a reference to the router.
    fn router(&self) -> &Router<N> {
        &self.router
    }
}

#[async_trait]
impl<N: Network, C: ConsensusStorage<N>> Inbound<N> for Prover<N, C> {
    /// Saves the latest epoch challenge and latest block in the prover.
    async fn puzzle_response(&self, peer_ip: SocketAddr, message: PuzzleResponse<N>) -> bool {
        let epoch_challenge = message.epoch_challenge;
        match message.block.deserialize().await {
            Ok(block) => {
                // Retrieve the epoch number.
                let epoch_number = epoch_challenge.epoch_number();
                // Retrieve the block height.
                let block_height = block.height();

                // Save the latest epoch challenge in the prover.
                self.latest_epoch_challenge.write().await.replace(epoch_challenge);
                // Save the latest block in the prover.
                self.latest_block.write().await.replace(block);

                trace!("Received 'PuzzleResponse' from '{peer_ip}' (Epoch {epoch_number}, Block {block_height})");
                true
            }
            Err(error) => {
                error!("Failed to deserialize the puzzle response from '{peer_ip}': {error}");
                false
            }
        }
    }

    /// If the last coinbase timestamp exceeds a multiple of the anchor time,
    /// then the prover will assist by propagating unconfirmed solutions.
    /// Otherwise, the prover will ignore the message.
    async fn unconfirmed_solution(
        &self,
        peer_ip: SocketAddr,
        message: UnconfirmedSolution<N>,
        _solution: ProverSolution<N>,
    ) -> bool {
        if let Some(block) = self.latest_block.read().await.as_ref() {
            // Compute the elapsed time since the last coinbase block.
            let elapsed = OffsetDateTime::now_utc().unix_timestamp().saturating_sub(block.last_coinbase_timestamp());
            // If the elapsed time exceeds a multiple of the anchor time, then assist in propagation.
            if elapsed > N::ANCHOR_TIME as i64 * 6 {
                // Propagate the `UnconfirmedSolution`.
                self.propagate(Message::UnconfirmedSolution(message), vec![peer_ip]);
            }
        }
        true
    }
}
