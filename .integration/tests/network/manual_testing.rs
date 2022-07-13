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

use crate::common::start_logger;
use snarkos_integration::TestNode;

use pea2pea::{
    protocols::{Handshake, Reading, Writing},
    Config,
    Node as Pea2PeaNode,
};
use std::net::{IpAddr, Ipv4Addr};

/// This test is intended to be run manually in order to monitor the behavior of
/// a full snarkOS node started independently.
#[ignore]
#[tokio::test]
async fn spawn_inert_node_at_port() {
    start_logger();

    const PORT: u16 = 4135;

    let config = Config {
        listener_ip: Some(IpAddr::V4(Ipv4Addr::LOCALHOST)),
        desired_listening_port: Some(PORT),
        ..Default::default()
    };

    let test_node = TestNode::new(Pea2PeaNode::new(config).await.unwrap(), Default::default());
    test_node.enable_handshake().await;
    test_node.enable_reading().await;
    test_node.enable_writing().await;
    // test_node.run_periodic_tasks();

    std::future::pending::<()>().await;
}
