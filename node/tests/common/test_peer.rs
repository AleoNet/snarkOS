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

use snarkos_account::Account;
use snarkos_node_messages::{ChallengeRequest, ChallengeResponse, Data, Message, MessageCodec, NodeType};
use snarkvm::prelude::{error, Address, Block, FromBytes, Network, TestRng, Testnet3 as CurrentNetwork};

use std::{
    io,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    str::FromStr,
};

use futures_util::{sink::SinkExt, TryStreamExt};
use pea2pea::{
    protocols::{Disconnect, Handshake, Reading, Writing},
    Config,
    Connection,
    ConnectionSide,
    Node,
    Pea2Pea,
};
use rand::Rng;
use tokio_util::codec::Framed;

const ALEO_MAXIMUM_FORK_DEPTH: u32 = 4096;

/// Returns a fixed account.
pub fn sample_account() -> Account<CurrentNetwork> {
    Account::<CurrentNetwork>::from_str("APrivateKey1zkp2oVPTci9kKcUprnbzMwq95Di1MQERpYBhEeqvkrDirK1").unwrap()
}

/// Loads the current network's genesis block.
pub fn sample_genesis_block() -> Block<CurrentNetwork> {
    Block::<CurrentNetwork>::from_bytes_le(CurrentNetwork::genesis_bytes()).unwrap()
}

#[derive(Clone)]
pub struct TestPeer {
    node: Node,
    node_type: NodeType,
    account: Account<CurrentNetwork>,
}

impl Pea2Pea for TestPeer {
    fn node(&self) -> &Node {
        &self.node
    }
}

impl TestPeer {
    pub async fn beacon() -> Self {
        Self::new(NodeType::Beacon, sample_account()).await
    }

    pub async fn client() -> Self {
        Self::new(NodeType::Client, sample_account()).await
    }

    pub async fn prover() -> Self {
        Self::new(NodeType::Prover, sample_account()).await
    }

    pub async fn validator() -> Self {
        Self::new(NodeType::Validator, sample_account()).await
    }

    pub async fn new(node_type: NodeType, account: Account<CurrentNetwork>) -> Self {
        let peer = Self {
            node: Node::new(Config {
                listener_ip: Some(IpAddr::V4(Ipv4Addr::LOCALHOST)),
                max_connections: 200,
                ..Default::default()
            })
            .await
            .expect("couldn't create test peer"),
            node_type,
            account,
        };

        peer.enable_handshake().await;
        peer.enable_reading().await;
        peer.enable_writing().await;
        peer.enable_disconnect().await;

        peer
    }

    pub fn node_type(&self) -> NodeType {
        self.node_type
    }

    pub fn account(&self) -> &Account<CurrentNetwork> {
        &self.account
    }

    pub fn address(&self) -> Address<CurrentNetwork> {
        self.account.address()
    }
}

#[async_trait::async_trait]
impl Handshake for TestPeer {
    async fn perform_handshake(&self, mut conn: Connection) -> io::Result<Connection> {
        let rng = &mut TestRng::default();

        let local_ip = self.node().listening_addr().expect("listening address should be present");

        let stream = self.borrow_stream(&mut conn);
        let mut framed = Framed::new(stream, MessageCodec::<CurrentNetwork>::default());

        // Send a challenge request to the peer.
        let message = Message::<CurrentNetwork>::ChallengeRequest(ChallengeRequest {
            version: Message::<CurrentNetwork>::VERSION,
            listener_port: local_ip.port(),
            node_type: self.node_type(),
            address: self.address(),
            nonce: rng.gen(),
        });
        framed.send(message).await?;

        // Listen for the challenge request.
        let request_b = match framed.try_next().await? {
            // Received the challenge request message, proceed.
            Some(Message::ChallengeRequest(data)) => data,
            // Received a disconnect message, abort.
            Some(Message::Disconnect(reason)) => return Err(error(format!("disconnected: {reason:?}"))),
            // Received an unexpected message, abort.
            _ => return Err(error("didn't send a challenge request")),
        };

        // TODO(nkls): add assertions on the contents.

        // Sign the nonce.
        let signature = self.account().sign_bytes(&request_b.nonce.to_le_bytes(), rng).unwrap();

        // Retrieve the genesis block header.
        let genesis_header = *sample_genesis_block().header();
        // Send the challenge response.
        let message =
            Message::ChallengeResponse(ChallengeResponse { genesis_header, signature: Data::Object(signature) });
        framed.send(message).await?;

        // Receive the challenge response.
        let Message::ChallengeResponse(challenge_response) = framed.try_next().await.unwrap().unwrap() else {
            panic!("didn't get challenge response")
        };

        assert_eq!(challenge_response.genesis_header, genesis_header);

        Ok(conn)
    }
}

#[async_trait::async_trait]
impl Writing for TestPeer {
    type Codec = MessageCodec<CurrentNetwork>;
    type Message = Message<CurrentNetwork>;

    fn codec(&self, _addr: SocketAddr, _side: ConnectionSide) -> Self::Codec {
        Default::default()
    }
}

#[async_trait::async_trait]
impl Reading for TestPeer {
    type Codec = MessageCodec<CurrentNetwork>;
    type Message = Message<CurrentNetwork>;

    fn codec(&self, _peer_addr: SocketAddr, _side: ConnectionSide) -> Self::Codec {
        Default::default()
    }

    async fn process_message(&self, _peer_ip: SocketAddr, _message: Self::Message) -> io::Result<()> {
        Ok(())
    }
}

#[async_trait::async_trait]
impl Disconnect for TestPeer {
    async fn handle_disconnect(&self, _peer_addr: SocketAddr) {}
}
