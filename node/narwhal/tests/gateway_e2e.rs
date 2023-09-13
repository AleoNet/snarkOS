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

#[allow(dead_code)]
mod common;

use crate::common::{primary::new_test_committee, test_peer::TestPeer, CurrentNetwork, MockLedgerService};
use snarkos_node_narwhal::{
    helpers::{EventOrBytes, NoiseCodec, NoiseState},
    Disconnect,
    DisconnectReason,
    Event,
    Gateway,
};
use snarkos_node_tcp::P2P;

use std::{io, net::SocketAddr, str::FromStr, sync::Arc, time::Duration};

use bytes::Bytes;
use deadline::deadline;
use futures_util::SinkExt;
use pea2pea::{protocols::Handshake, Connection, Pea2Pea};
use snow::{params::NoiseParams, Builder};
use tokio_stream::StreamExt;
use tokio_util::codec::Framed;

const NOISE_HANDSHAKE_TYPE: &str = "Noise_XX_25519_ChaChaPoly_BLAKE2s";

// The test peer connects to the gateway, completes the noise handshake but doesn't send any
// further messages. The gateway's handshake should timeout.
#[tokio::test(flavor = "multi_thread")]
async fn handshake_responder_side_timeout() {
    const NUM_NODES: u16 = 4;

    // Set up the gateway instance.
    let (accounts, committee) = new_test_committee(NUM_NODES);
    let ledger = Arc::new(MockLedgerService::new(committee.clone()));
    let addr = SocketAddr::from_str("127.0.0.1:0").ok();
    let trusted_validators = [];
    let gateway = Gateway::new(accounts.first().unwrap().clone(), ledger, addr, &trusted_validators, None).unwrap();
    gateway.run([].into()).await;

    // Implement the test peer's handshake logic.
    #[async_trait::async_trait]
    impl Handshake for TestPeer {
        // Set the timeout on the test peer to be longer than the gateway's timeout.
        const TIMEOUT_MS: u64 = 10_000;

        async fn perform_handshake(&self, mut connection: Connection) -> io::Result<Connection> {
            let stream = self.borrow_stream(&mut connection);

            // Set up the noise codec.
            let params: NoiseParams = NOISE_HANDSHAKE_TYPE.parse().unwrap();
            let noise_builder = Builder::new(params);
            let kp = noise_builder.generate_keypair().unwrap();
            let initiator = noise_builder.local_private_key(&kp.private).build_initiator().unwrap();
            let codec = NoiseCodec::<CurrentNetwork>::new(NoiseState::Handshake(Box::new(initiator)));

            // Construct the stream.
            let mut framed = Framed::new(stream, codec);

            /* Noise handshake */

            // -> e
            framed.send(EventOrBytes::Bytes(Bytes::new())).await?;

            // <- e, ee, s, es
            framed.try_next().await?;

            // -> s, se
            framed.send(EventOrBytes::Bytes(Bytes::new())).await?;

            // Set the codec to post-handshake mode.
            let mut framed =
                framed.map_codec(|c| NoiseCodec::<CurrentNetwork>::new(c.noise_state.into_post_handshake_state()));

            // Send a disconnect event.
            framed
                .send(EventOrBytes::Event(Event::Disconnect(Disconnect::from(DisconnectReason::NoReasonGiven))))
                .await?;

            Ok(connection)
        }
    }

    // Set up the test peer.
    let test_peer = TestPeer::new().await;
    test_peer.enable_handshake().await;

    // Initiate a connection with the gateway, this will only return once the handshake has
    // completed on the test peer's side.
    assert!(test_peer.node().connect(gateway.local_ip()).await.is_ok());

    // Check the test peer hasn't been added to the gateway's connected peers.
    assert!(gateway.connected_peers().read().is_empty());
    // TODO: we might want this to be set earlier in the handshake, if so, we should check before
    // and after the tcp assertion.
    // assert!(gateway.connecting_peers().lock().is_empty());

    // Check the tcp stack's connection counts, wait longer than the gateway's timeout to ensure
    // connecting peers are cleared.
    deadline!(Duration::from_secs(5), move || gateway.tcp().num_connecting() == 0);
}

#[tokio::test]
async fn handshake_responder_side() {}
