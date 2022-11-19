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

use snarkos_node_messages::{ChallengeRequest, ChallengeResponse, Data, Message, MessageCodec, NodeType, Status};
use snarkos_node_tcp::{protocols::Handshake, Config, Connection, Tcp, P2P};
use snarkvm::prelude::{Block, FromBytes, Network, Testnet3 as CurrentNetwork};

use futures_util::{sink::SinkExt, TryStreamExt};
use std::{
    io,
    net::{IpAddr, Ipv4Addr},
};
use tokio_util::codec::Framed;

const ALEO_MAXIMUM_FORK_DEPTH: u32 = 4096;

#[derive(Clone)]
pub struct TestPeer {
    tcp: Tcp,
    node_type: NodeType,
}

impl P2P for TestPeer {
    fn tcp(&self) -> &Tcp {
        &self.tcp
    }
}

impl TestPeer {
    pub async fn new(node_type: NodeType) -> Self {
        let peer = Self {
            tcp: Tcp::new(Config {
                listener_ip: Some(IpAddr::V4(Ipv4Addr::LOCALHOST)),
                max_connections: 200,
                ..Default::default()
            })
            .await
            .expect("couldn't create test peer"),
            node_type,
        };

        peer.enable_handshake().await;
        //  client.enable_reading().await;
        //  client.enable_writing().await;
        //  client.enable_disconnect().await;

        peer
    }

    pub fn node_type(&self) -> NodeType {
        self.node_type
    }
}

#[async_trait::async_trait]
impl Handshake for TestPeer {
    async fn perform_handshake(&self, mut conn: Connection) -> io::Result<Connection> {
        let local_ip = self.tcp().listening_addr().expect("listening address should be present");

        let stream = self.borrow_stream(&mut conn);
        let mut framed = Framed::new(stream, MessageCodec::<CurrentNetwork>::default());

        // TODO (howardwu): Make this step more efficient (by not deserializing every time).
        // Retrieve the genesis block header.
        let genesis_header = *Block::<CurrentNetwork>::from_bytes_le(CurrentNetwork::genesis_bytes())
            .expect("genesis block bytes should be valid")
            .header();

        // Send a challenge request to the peer.
        let message = Message::<CurrentNetwork>::ChallengeRequest(ChallengeRequest {
            version: Message::<CurrentNetwork>::VERSION,
            fork_depth: ALEO_MAXIMUM_FORK_DEPTH,
            node_type: self.node_type(),
            status: Status::Peering,
            listener_port: local_ip.port(),
        });
        framed.send(message).await?;

        // Receive the challenge request.
        let _challenge_request = framed.try_next().await?.unwrap();

        // TODO(nkls): add assertions on the contents.

        // Send the challenge response.
        let message = Message::ChallengeResponse(ChallengeResponse { header: Data::Object(genesis_header) });
        framed.send(message).await?;

        // Receive the challenge response.
        let Message::ChallengeResponse(challenge_response) = framed.try_next().await.unwrap().unwrap() else {
            panic!("didn't get challenge response")
        };

        // Perform the deferred non-blocking deserialization of the block header.
        let Ok(block_header) = challenge_response.header.deserialize().await else {
            panic!("block header not present")
        };

        assert_eq!(block_header, genesis_header);

        Ok(conn)
    }
}
