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
use snarkos_node_router::Routing;
use snarkos_node_tcp::{Connection, ConnectionSide, Tcp};

use std::{io, net::SocketAddr};

impl<N: Network, C: ConsensusStorage<N>> P2P for Beacon<N, C> {
    /// Returns a reference to the TCP instance.
    fn tcp(&self) -> &Tcp {
        self.router.tcp()
    }
}

#[async_trait]
impl<N: Network, C: ConsensusStorage<N>> Handshake for Beacon<N, C> {
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
impl<N: Network, C: ConsensusStorage<N>> Disconnect for Beacon<N, C> {
    /// Any extra operations to be performed during a disconnect.
    async fn handle_disconnect(&self, peer_addr: SocketAddr) {
        self.router.remove_connected_peer(peer_addr);
    }
}

#[async_trait]
impl<N: Network, C: ConsensusStorage<N>> Writing for Beacon<N, C> {
    type Codec = MessageCodec<N>;
    type Message = Message<N>;

    /// Creates an [`Encoder`] used to write the outbound messages to the target stream.
    /// The `side` parameter indicates the connection side **from the node's perspective**.
    fn codec(&self, _addr: SocketAddr, _side: ConnectionSide) -> Self::Codec {
        Default::default()
    }
}

#[async_trait]
impl<N: Network, C: ConsensusStorage<N>> Reading for Beacon<N, C> {
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
        if let Err(error) = self.inbound(peer_ip, message).await {
            warn!("Disconnecting from '{peer_ip}' - {error}");
            self.send(peer_ip, Message::Disconnect(DisconnectReason::ProtocolViolation.into()));
            // Disconnect from this peer.
            self.router().disconnect(peer_ip).await;
        }
        Ok(())
    }
}

#[async_trait]
impl<N: Network, C: ConsensusStorage<N>> Routing<N> for Beacon<N, C> {}

#[async_trait]
impl<N: Network, C: ConsensusStorage<N>> Heartbeat<N> for Beacon<N, C> {
    /// The maximum number of peers permitted to maintain connections with.
    const MAXIMUM_NUMBER_OF_PEERS: usize = 10;
}

impl<N: Network, C: ConsensusStorage<N>> Outbound<N> for Beacon<N, C> {
    /// Returns a reference to the router.
    fn router(&self) -> &Router<N> {
        &self.router
    }
}

#[async_trait]
impl<N: Network, C: ConsensusStorage<N>> Inbound<N> for Beacon<N, C> {
    /// Retrieves the latest epoch challenge and latest block, and returns the puzzle response to the peer.
    fn puzzle_request(&self, peer_ip: SocketAddr) -> bool {
        // Retrieve the latest epoch challenge and latest block.
        let (epoch_challenge, block) = {
            // Retrieve the latest epoch challenge.
            let epoch_challenge = match self.ledger.latest_epoch_challenge() {
                Ok(block) => block,
                Err(error) => {
                    error!("Failed to retrieve latest epoch challenge for a puzzle request: {error}");
                    return false;
                }
            };
            // Retrieve the latest block.
            let block = self.ledger.latest_block();

            // Scope drops the read lock on the consensus module.
            (epoch_challenge, block)
        };
        // Send the `PuzzleResponse` message to the peer.
        self.send(peer_ip, Message::PuzzleResponse(PuzzleResponse { epoch_challenge, block: Data::Object(block) }));
        true
    }

    /// Adds the unconfirmed solution to the memory pool, and propagates the solution to all peers.
    async fn unconfirmed_solution(
        &self,
        _peer_ip: SocketAddr,
        _message: UnconfirmedSolution<N>,
        solution: ProverSolution<N>,
    ) -> bool {
        // Add the unconfirmed solution to the memory pool.
        if let Err(error) = self.consensus.add_unconfirmed_solution(&solution) {
            trace!("[UnconfirmedSolution] {error}");
            return true; // Maintain the connection.
        }
        // // Propagate the `UnconfirmedSolution` to connected beacons.
        // let request = RouterRequest::MessagePropagateBeacon(Message::UnconfirmedSolution(message), vec![peer_ip]);
        // if let Err(error) = router.process(request).await {
        //     warn!("[UnconfirmedSolution] {error}");
        // }
        true
    }

    /// Adds the unconfirmed transaction to the memory pool, and propagates the transaction to all peers.
    fn unconfirmed_transaction(
        &self,
        _peer_ip: SocketAddr,
        _message: UnconfirmedTransaction<N>,
        transaction: Transaction<N>,
    ) -> bool {
        // Add the unconfirmed transaction to the memory pool.
        if let Err(error) = self.consensus.add_unconfirmed_transaction(transaction) {
            trace!("[UnconfirmedTransaction] {error}");
            return true; // Maintain the connection.
        }
        // // Propagate the `UnconfirmedTransaction`.
        // let request = RouterRequest::MessagePropagate(Message::UnconfirmedTransaction(message), vec![peer_ip]);
        // if let Err(error) = router.process(request).await {
        //     warn!("[UnconfirmedTransaction] {error}");
        // }
        true
    }
}
