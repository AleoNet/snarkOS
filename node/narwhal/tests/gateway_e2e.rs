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
use snarkos_node_narwhal::{helpers::EventOrBytes, Disconnect, DisconnectReason, Event, Gateway, WorkerPing};
use snarkos_node_tcp::P2P;

use std::{net::SocketAddr, str::FromStr, sync::Arc, time::Duration};

use deadline::deadline;
use pea2pea::{protocols::Writing, Pea2Pea};

async fn new_test_gateway() -> Gateway<CurrentNetwork> {
    const NUM_NODES: u16 = 4;

    // Set up the gateway instance.
    let (accounts, committee) = new_test_committee(NUM_NODES);
    let ledger = Arc::new(MockLedgerService::new(committee.clone()));
    let addr = SocketAddr::from_str("127.0.0.1:0").ok();
    let trusted_validators = [];
    let gateway = Gateway::new(accounts.first().unwrap().clone(), ledger, addr, &trusted_validators, None).unwrap();
    gateway.run([].into()).await;

    gateway
}

// The test peer connects to the gateway, completes the noise handshake but doesn't send any
// further messages. The gateway's handshake should timeout.
#[tokio::test(flavor = "multi_thread")]
async fn handshake_responder_side_timeout() {
    let gateway = new_test_gateway().await;
    let test_peer = TestPeer::new().await;

    // Initiate a connection with the gateway, this will only return once the handshake has
    // completed on the test peer's side, which only includes the noise portion.
    assert!(test_peer.node().connect(gateway.local_ip()).await.is_ok());

    /* Don't send any further messages and wait for the gateway to timeout. */

    // Check the test peer hasn't been added to the gateway's connected peers.
    assert!(gateway.connected_peers().read().is_empty());

    // Check the tcp stack's connection counts, wait longer than the gateway's timeout to ensure
    // connecting peers are cleared.
    deadline!(Duration::from_secs(5), move || gateway.tcp().num_connecting() == 0);
}

// The test peer connects to the gateway, completes the noise handshake and sends an unexpected
// event. The gateway's handshake should be interrupted and the peer should be disconnected.
macro_rules! handshake_responder_side_unexpected_event {
    ($test_name:ident, $payload:expr) => {
        paste::paste! {
            #[tokio::test(flavor = "multi_thread")]
            async fn [<handshake_responder_side_unexpected_ $test_name>]() {
                let gateway = new_test_gateway().await;
                let test_peer = TestPeer::new().await;

                // Initiate a connection with the gateway, this will only return once the handshake has
                // completed on the test peer's side, which only includes the noise portion.
                assert!(test_peer.node().connect(gateway.local_ip()).await.is_ok());

                // Check the gateway is still handshaking with us.
                assert_eq!(gateway.tcp().num_connecting(), 1);

                // Send an unexpected event.
                let _ = test_peer.unicast(
                    gateway.local_ip(),
                    $payload
                );

                // Check the test peer hasn't been added to the gateway's connected peers.
                assert!(gateway.connected_peers().read().is_empty());

                // Check the tcp stack's connection counts, make sure the disconnect interrupted handshaking,
                // wait a short time to ensure the gateway has time to process the disconnect (note: this is
                // shorter than the gateway's timeout, so we can ensure that's not the reason for the
                // disconnect).
                deadline!(Duration::from_secs(1), move || gateway.tcp().num_connecting() == 0);
            }
        }
    };
}

/* Unexpected disconnects. */

macro_rules! handshake_responder_side_unexpected_disconnect {
    ($($reason:ident),*) => {
        $(
            paste::paste! {
                handshake_responder_side_unexpected_event!(
                    [<disconnect_ $reason:snake>],
                    EventOrBytes::Event(Event::Disconnect(Disconnect::from(DisconnectReason::$reason)))
                );
            }
        )*
    }
}

handshake_responder_side_unexpected_disconnect!(
    ProtocolViolation,
    NoReasonGiven,
    InvalidChallengeResponse,
    OutdatedClientVersion
);

/* Other unexpected event types */

handshake_responder_side_unexpected_event!(
    unexpected_worker_ping,
    EventOrBytes::Event(Event::WorkerPing(WorkerPing::new([].into())))
);

// TODO(nkls): other event types, can be done as a follow up.
