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

use snarkos_environment::{
    helpers::{NodeType, State, Status},
    Client,
    CurrentNetwork,
    Environment,
};
use snarkos_network::{Data, Message};
use snarkvm::traits::Network;

use pea2pea::{
    protocols::{Disconnect, Handshake, Writing},
    Connection,
    Node as Pea2PeaNode,
    Pea2Pea,
};
use rand::{thread_rng, Rng};
use std::{convert::TryInto, io, net::SocketAddr, sync::Arc};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::Mutex,
};
use tracing::*;

// Consts & aliases.
pub const MESSAGE_LENGTH_PREFIX_SIZE: usize = 4;
pub const CHALLENGE_HEIGHT: u32 = 0;
pub const MESSAGE_VERSION: u32 = <Client<CurrentNetwork>>::MESSAGE_VERSION;
pub const MAXIMUM_FORK_DEPTH: u32 = CurrentNetwork::ALEO_MAXIMUM_FORK_DEPTH;

pub const MAXIMUM_NUMBER_OF_PEERS: usize = <Client<CurrentNetwork>>::MAXIMUM_NUMBER_OF_PEERS;

pub type ClientMessage = Message<CurrentNetwork, Client<CurrentNetwork>>;
pub type ClientNonce = u64;

/// The test node; it consists of a `Node` that handles networking and `State`
/// that can be extended freely based on test requirements.
#[derive(Clone)]
pub struct SynthNode {
    node: Pea2PeaNode,
    pub state: ClientState,
}

/// Represents a connected snarkOS client peer.
pub struct ClientPeer {
    connected_addr: SocketAddr,
    pub listening_addr: SocketAddr,
    nonce: ClientNonce,
}

/// snarkOS client state required for test purposes.
#[derive(Clone)]
pub struct ClientState {
    pub local_nonce: ClientNonce,
    /// The list of known peers; `Pea2Pea` includes its own internal peer handling,
    /// but snarkOS nodes must discover the listening address and unique nonce of each peer;
    /// this collection facilitates the snarkOS peering experience to align with snarkOS logic.
    pub peers: Arc<Mutex<Vec<ClientPeer>>>,
    pub status: Status,
}

impl Default for ClientState {
    fn default() -> Self {
        Self {
            local_nonce: thread_rng().gen(),
            peers: Default::default(),
            status: Status::new(),
        }
    }
}

impl Pea2Pea for SynthNode {
    fn node(&self) -> &Pea2PeaNode {
        &self.node
    }
}

impl SynthNode {
    /// Creates a test node using the given `Pea2Pea` node.
    pub fn new(node: Pea2PeaNode, state: ClientState) -> Self {
        Self { node, state }
    }

    pub fn node_type(&self) -> NodeType {
        NodeType::Client
    }

    pub fn state(&self) -> State {
        self.state.status.get()
    }
}

/// Automated handshake handling for the test nodes.
#[async_trait::async_trait]
impl Handshake for SynthNode {
    async fn perform_handshake(&self, mut connection: Connection) -> io::Result<Connection> {
        // Guard against double (two-sided) connections.
        let mut locked_peers = self.state.peers.lock().await;

        let own_ip = self.node().listening_addr()?;
        let peer_ip = connection.addr;

        let genesis_block_header = CurrentNetwork::genesis_block().header();

        // Send a challenge request to the peer.
        let own_request = ClientMessage::ChallengeRequest(
            MESSAGE_VERSION,
            MAXIMUM_FORK_DEPTH,
            NodeType::Client,
            State::Ready,
            own_ip.port(),
            self.state.local_nonce,
            0,
        );
        trace!(parent: self.node().span(), "sending a challenge request to {}", peer_ip);
        let mut msg = Vec::new();
        own_request.serialize_into(&mut msg).unwrap();
        let len = u32::to_le_bytes(msg.len() as u32);
        connection.writer().write_all(&len).await?;
        connection.writer().write_all(&msg).await?;

        let mut buf = [0u8; 1024];

        // Read the challenge request from the peer.
        connection.reader().read_exact(&mut buf[..MESSAGE_LENGTH_PREFIX_SIZE]).await?;
        let len = u32::from_le_bytes(buf[..MESSAGE_LENGTH_PREFIX_SIZE].try_into().unwrap()) as usize;
        connection.reader().read_exact(&mut buf[..len]).await?;
        let peer_request = ClientMessage::deserialize(&mut io::Cursor::new(&buf[..len]));

        // Register peer's nonce.
        let (peer_listening_addr, peer_nonce) = if let Ok(Message::ChallengeRequest(
            peer_version,
            _peer_fork_depth,
            _peer_node_type,
            _peer_status,
            peer_listening_port,
            peer_nonce,
            _cumulative_weight,
        )) = peer_request
        {
            if peer_version < MESSAGE_VERSION {
                warn!(parent: self.node().span(), "dropping {} due to outdated version ({})", peer_ip, peer_version);
                return Err(io::ErrorKind::InvalidData.into());
            }

            let peer_listening_ip = SocketAddr::from((peer_ip.ip(), peer_listening_port));

            if locked_peers
                .iter()
                .any(|peer| peer.nonce == peer_nonce || peer.listening_addr == peer_listening_ip)
            {
                return Err(io::ErrorKind::AlreadyExists.into());
            }

            trace!(parent: self.node().span(), "received a challenge request from {}", peer_ip);

            (peer_listening_ip, peer_nonce)
        } else if let Ok(Message::Disconnect(reason)) = peer_request {
            warn!(parent: self.node().span(), "{} disconnected: {:?}", peer_ip, reason);
            return Err(io::ErrorKind::NotConnected.into());
        } else {
            error!(parent: self.node().span(), "invalid challenge request from {}", peer_ip);
            return Err(io::ErrorKind::InvalidData.into());
        };

        // Respond with own challenge request.
        let own_response = ClientMessage::ChallengeResponse(Data::Object(genesis_block_header.clone()));
        trace!(parent: self.node().span(), "sending a challenge response to {}", peer_ip);
        let mut msg = Vec::new();
        own_response.serialize_into(&mut msg).unwrap();
        let len = u32::to_le_bytes(msg.len() as u32);
        connection.writer().write_all(&len).await?;
        connection.writer().write_all(&msg).await?;

        // Wait for the challenge response to come in.
        connection.reader().read_exact(&mut buf[..MESSAGE_LENGTH_PREFIX_SIZE]).await?;
        let len = u32::from_le_bytes(buf[..MESSAGE_LENGTH_PREFIX_SIZE].try_into().unwrap()) as usize;
        connection.reader().read_exact(&mut buf[..len]).await?;
        let peer_response = ClientMessage::deserialize(&mut io::Cursor::new(&buf[..len]));

        if let Ok(Message::ChallengeResponse(block_header)) = peer_response {
            let block_header = block_header.deserialize().await.unwrap();

            trace!(parent: self.node().span(), "received a challenge response from {}", peer_ip);
            if block_header.height() == CHALLENGE_HEIGHT && &block_header == genesis_block_header && block_header.is_valid() {
                // Register the newly connected snarkOS peer.
                locked_peers.push(ClientPeer {
                    connected_addr: peer_ip,
                    listening_addr: peer_listening_addr,
                    nonce: peer_nonce,
                });
                debug!(parent: self.node().span(), "connected to {} (listening addr: {})", peer_ip, peer_listening_addr);

                Ok(connection)
            } else {
                error!(parent: self.node().span(), "invalid challenge response from {}", peer_ip);
                Err(io::ErrorKind::InvalidData.into())
            }
        } else if let Ok(Message::Disconnect(reason)) = peer_response {
            warn!(parent: self.node().span(), "{} disconnected: {:?}", peer_ip, reason);
            return Err(io::ErrorKind::NotConnected.into());
        } else {
            error!(parent: self.node().span(), "invalid challenge response from {}", peer_ip);
            Err(io::ErrorKind::InvalidData.into())
        }
    }
}

/// Outbound message processing logic for the test nodes.
impl Writing for SynthNode {
    type Message = ClientMessage;

    fn write_message<W: io::Write>(&self, _target: SocketAddr, payload: &Self::Message, writer: &mut W) -> io::Result<()> {
        let mut msg = Vec::new();
        payload.serialize_into(&mut msg).unwrap();
        let len = u32::to_le_bytes(msg.len() as u32);

        writer.write_all(&len)?;
        writer.write_all(&msg)
    }
}

/// Disconnect logic for the test nodes.
#[async_trait::async_trait]
impl Disconnect for SynthNode {
    async fn handle_disconnect(&self, disconnecting_addr: SocketAddr) {
        let mut locked_peers = self.state.peers.lock().await;
        let initial_len = locked_peers.len();
        locked_peers.retain(|peer| peer.connected_addr != disconnecting_addr);
        assert_eq!(locked_peers.len(), initial_len - 1)
    }
}

/// Enables tracing for all synth node instances (usually scoped by test).
pub fn enable_tracing() {
    use tracing_subscriber::{fmt, EnvFilter};

    fmt().with_test_writer().with_env_filter(EnvFilter::from_default_env()).init();
}
