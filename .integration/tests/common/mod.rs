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

use snarkos_integration::test_node::TestNode;
use snarkos_synthetic_node::{ClientNonce, ClientState};

use pea2pea::{protocols::*, Config};
use std::net::{IpAddr, Ipv4Addr};
use tracing_subscriber::filter::EnvFilter;

/// Starts a logger if a test node needs to be inspected in greater detail.
// note: snarkOS node currently starts it by default, so it's not needed
pub fn start_logger() {
    let filter = match EnvFilter::try_from_default_env() {
        Ok(filter) => filter
            .add_directive("mio=off".parse().unwrap())
            .add_directive("tokio_util=off".parse().unwrap())
            .add_directive("hyper::proto::h1::conn=off".parse().unwrap())
            .add_directive("hyper::proto::h1::decode=off".parse().unwrap())
            .add_directive("hyper::proto::h1::io=off".parse().unwrap())
            .add_directive("hyper::proto::h1::role=off".parse().unwrap()),
        _ => EnvFilter::default()
            .add_directive("mio=off".parse().unwrap())
            .add_directive("mio=off".parse().unwrap())
            .add_directive("tokio_util=off".parse().unwrap())
            .add_directive("hyper::proto::h1::conn=off".parse().unwrap())
            .add_directive("hyper::proto::h1::decode=off".parse().unwrap())
            .add_directive("hyper::proto::h1::io=off".parse().unwrap())
            .add_directive("hyper::proto::h1::role=off".parse().unwrap()),
    };
    tracing_subscriber::fmt().with_env_filter(filter).with_target(false).init();
}

/// Spawns a `TestNode` with the given handshake nonce.
pub async fn spawn_test_node_with_nonce(local_nonce: ClientNonce) -> TestNode {
    let config = Config {
        listener_ip: Some(IpAddr::V4(Ipv4Addr::LOCALHOST)),
        ..Default::default()
    };

    let pea2pea_node = pea2pea::Node::new(config).await.unwrap();
    let client_state = ClientState {
        local_nonce,
        ..Default::default()
    };

    let node = TestNode::new(pea2pea_node, client_state);
    node.enable_handshake().await;
    node.enable_reading().await;
    node.enable_writing().await;
    node
}

/// A helper function making memory use values more human-readable.
pub fn display_bytes(bytes: f64) -> String {
    const GB: f64 = 1_000_000_000.0;
    const MB: f64 = 1_000_000.0;
    const KB: f64 = 1_000.0;

    if bytes >= GB {
        format!("{:.2} GB", bytes / GB)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes / MB)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes / KB)
    } else {
        format!("{:.2} B", bytes)
    }
}
