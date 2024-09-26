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

use crate::common::sample_genesis_block;
use snarkos_node_router::{
    messages::{
        BlockRequest,
        DisconnectReason,
        Message,
        MessageCodec,
        Ping,
        Pong,
        UnconfirmedSolution,
        UnconfirmedTransaction,
    },
    Heartbeat,
    Inbound,
    Outbound,
    Router,
    Routing,
};
use snarkos_node_tcp::{
    protocols::{Disconnect, Handshake, OnConnect, Reading, Writing},
    Connection,
    ConnectionSide,
    Tcp,
    P2P,
};
use snarkvm::prelude::{
    block::{Block, Header, Transaction},
    puzzle::Solution,
    Field,
    Network,
};

use async_trait::async_trait;
use std::{io, net::SocketAddr, str::FromStr};
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
        let restrictions_id =
            Field::<N>::from_str("7562506206353711030068167991213732850758501012603348777370400520506564970105field")
                .unwrap();
        self.router().handshake(peer_addr, stream, conn_side, genesis_header, restrictions_id).await?;

        Ok(connection)
    }
}

#[async_trait]
impl<N: Network> OnConnect for TestRouter<N> {
    async fn on_connect(&self, _peer_addr: SocketAddr) {
        // This behavior is currently not tested.
    }
}

#[async_trait]
impl<N: Network> Disconnect for TestRouter<N> {
    /// Any extra operations to be performed during a disconnect.
    async fn handle_disconnect(&self, peer_addr: SocketAddr) {
        if let Some(peer_ip) = self.router().resolve_to_listener(&peer_addr) {
            self.router().remove_connected_peer(peer_ip);
        }
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
impl<N: Network> Inbound<N> for TestRouter<N> {
    /// Handles a `BlockRequest` message.
    fn block_request(&self, _peer_ip: SocketAddr, _message: BlockRequest) -> bool {
        true
    }

    /// Handles a `BlockResponse` message.
    fn block_response(&self, _peer_ip: SocketAddr, _blocks: Vec<Block<N>>) -> bool {
        true
    }

    /// Handles an `Ping` message.
    fn ping(&self, _peer_ip: SocketAddr, _message: Ping<N>) -> bool {
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
    fn puzzle_response(&self, _peer_ip: SocketAddr, _epoch_hash: N::BlockHash, _header: Header<N>) -> bool {
        true
    }

    /// Handles an `UnconfirmedSolution` message.
    async fn unconfirmed_solution(
        &self,
        _peer_ip: SocketAddr,
        _serialized: UnconfirmedSolution<N>,
        _solution: Solution<N>,
    ) -> bool {
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
