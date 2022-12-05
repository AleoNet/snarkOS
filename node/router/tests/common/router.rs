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

use crate::common::sample_genesis_block;
use snarkos_node_messages::{
    BlockRequest,
    DisconnectReason,
    Message,
    MessageCodec,
    Ping,
    Pong,
    PuzzleResponse,
    UnconfirmedSolution,
    UnconfirmedTransaction,
};
use snarkos_node_router::{Heartbeat, Inbound, Outbound, Router, Routing};
use snarkos_node_tcp::{
    protocols::{Disconnect, Handshake, Reading, Writing},
    Connection,
    ConnectionSide,
    Tcp,
    P2P,
};
use snarkvm::prelude::{Block, Header, Network, ProverSolution, Transaction};

use async_trait::async_trait;
use futures_util::sink::SinkExt;
use std::{io, net::SocketAddr};
use tracing::*;

#[derive(Clone)]
pub struct TestRouter<N: Network>(Router<N>);

impl<N: Network> From<Router<N>> for TestRouter<N> {
    fn from(router: Router<N>) -> Self {
        Self(router)
    }
}

impl<N: Network> core::ops::Deref for TestRouter<N> {
    type Target = Router<N>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<N: Network> P2P for TestRouter<N> {
    /// Returns a reference to the TCP instance.
    fn tcp(&self) -> &Tcp {
        self.router().tcp()
    }
}

#[async_trait]
impl<N: Network> Handshake for TestRouter<N> {
    /// Performs the handshake protocol.
    async fn perform_handshake(&self, mut connection: Connection) -> io::Result<Connection> {
        // Perform the handshake.
        let peer_addr = connection.addr();
        let conn_side = connection.side();
        let stream = self.borrow_stream(&mut connection);
        let genesis_header = *sample_genesis_block().header();
        let (peer_ip, mut framed) = self.router().handshake(peer_addr, stream, conn_side, genesis_header).await?;

        // Send the first `Ping` message to the peer.
        let message = Message::Ping(Ping::<N> {
            version: Message::<N>::VERSION,
            node_type: self.node_type(),
            block_locators: None,
        });
        trace!("Sending '{}' to '{peer_ip}'", message.name());
        framed.send(message).await?;

        Ok(connection)
    }
}

#[async_trait]
impl<N: Network> Disconnect for TestRouter<N> {
    /// Any extra operations to be performed during a disconnect.
    async fn handle_disconnect(&self, peer_addr: SocketAddr) {
        self.router().remove_connected_peer(peer_addr);
    }
}

#[async_trait]
impl<N: Network> Writing for TestRouter<N> {
    type Codec = MessageCodec<N>;
    type Message = Message<N>;

    /// Creates an [`Encoder`] used to write the outbound messages to the target stream.
    /// The `side` parameter indicates the connection side **from the node's perspective**.
    fn codec(&self, _addr: SocketAddr, _side: ConnectionSide) -> Self::Codec {
        Default::default()
    }
}

#[async_trait]
impl<N: Network> Reading for TestRouter<N> {
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
            self.router().disconnect(peer_ip);
        }
        Ok(())
    }
}

#[async_trait]
impl<N: Network> Routing<N> for TestRouter<N> {}

impl<N: Network> Heartbeat<N> for TestRouter<N> {}

impl<N: Network> Outbound<N> for TestRouter<N> {
    /// Returns a reference to the router.
    fn router(&self) -> &Router<N> {
        &self.0
    }
}

#[async_trait]
impl<N: Network> Inbound<N> for TestRouter<N> {
    /// Handles a `BlockRequest` message.
    fn block_request(&self, _peer_ip: SocketAddr, _message: BlockRequest) -> bool {
        true
    }

    /// Handles a `BlockResponse` message.
    fn block_response(&self, _peer_ip: SocketAddr, _blocks: Vec<Block<N>>) -> bool {
        true
    }

    /// Handles an `Pong` message.
    fn pong(&self, _peer_ip: SocketAddr, _message: Pong) -> bool {
        true
    }

    /// Handles an `PuzzleRequest` message.
    fn puzzle_request(&self, _peer_ip: SocketAddr) -> bool {
        true
    }

    /// Handles an `PuzzleResponse` message.
    fn puzzle_response(&self, _peer_ip: SocketAddr, _serialized: PuzzleResponse<N>, _header: Header<N>) -> bool {
        true
    }

    /// Handles an `UnconfirmedSolution` message.
    async fn unconfirmed_solution(
        &self,
        _peer_ip: SocketAddr,
        _serialized: UnconfirmedSolution<N>,
        _solution: ProverSolution<N>,
    ) -> bool {
        true
    }

    /// Handles an `UnconfirmedTransaction` message.
    fn unconfirmed_transaction(
        &self,
        _peer_ip: SocketAddr,
        _serialized: UnconfirmedTransaction<N>,
        _transaction: Transaction<N>,
    ) -> bool {
        true
    }
}
