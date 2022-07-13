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

use pea2pea::{protocols::Writing, Pea2Pea};
use snarkos_environment::{
    helpers::{NodeType, Status},
    network::{Data, Message},
    Client,
    CurrentNetwork,
    Environment,
};
use snarkos_integration::{wait_until, ClientNode, TestNode};
use snarkvm::dpc::traits::network::Network;

use std::time::{Duration, Instant};

#[tokio::test]
#[ignore = "this test is purely informational; latest result: ~540ms"]
async fn measure_node_startup() {
    const NUM_ITERATIONS: usize = 10;
    let mut avg_start_up_time = Duration::default();

    for _ in 0..NUM_ITERATIONS {
        let start = Instant::now();
        let _snarkos_node = ClientNode::default().await;
        avg_start_up_time += start.elapsed();
    }
    avg_start_up_time /= NUM_ITERATIONS as u32;

    // Display the result.
    println!("snarkOS start-up time: {:?}", avg_start_up_time);
}

#[tokio::test]
#[ignore = "this test is purely informational; latest result: ~185ms"]
async fn measure_connection_time() {
    const NUM_ITERATIONS: usize = 10;
    let mut avg_conn_time = Duration::default();

    let connector = ClientNode::default().await;
    let connector_addr = connector.local_addr();
    let connectee = ClientNode::default().await;
    let connectee_addr = connectee.local_addr();

    for _ in 0..NUM_ITERATIONS {
        let start = Instant::now();
        connector.connect(connectee_addr).await.unwrap();
        avg_conn_time += start.elapsed();
        wait_until!(1, connectee.number_of_connected_peers().await == 1);

        connector.disconnect(connectee_addr).await;
        connectee.disconnect(connector_addr).await;
        wait_until!(1, connector.number_of_connected_peers().await == 0);
        wait_until!(1, connectee.number_of_connected_peers().await == 0);
        connector.reset_known_peers().await;
        connectee.reset_known_peers().await;
    }
    avg_conn_time /= NUM_ITERATIONS as u32;

    // Display the result.
    println!("snarkOS connection time: {:?}", avg_conn_time);
}

#[tokio::test]
#[ignore = "this test is purely informational; latest result: ~2ms"]
async fn measure_peer_request_time() {
    const NUM_ITERATIONS: usize = 10;
    let mut avg_request_time = Duration::default();

    let test_node = TestNode::default().await;
    let client_node = ClientNode::default().await;
    let client_addr = client_node.local_addr();
    test_node.node().connect(client_addr).await.unwrap();
    wait_until!(1, client_node.number_of_connected_peers().await == 1);
    wait_until!(1, test_node.node().stats().received().0 as usize == 1);

    let init_recv_count = test_node.node().stats().received().0 as usize;
    for i in 0..NUM_ITERATIONS {
        let start = Instant::now();
        test_node
            .unicast(client_addr, Message::PeerRequest)
            .unwrap()
            .await
            .unwrap()
            .unwrap();
        wait_until!(1, test_node.node().stats().received().0 as usize == init_recv_count + i + 1);
        avg_request_time += start.elapsed();
    }
    avg_request_time /= NUM_ITERATIONS as u32;

    // Display the result.
    println!("snarkOS peer request time: {:?}", avg_request_time);
}

#[tokio::test]
#[ignore = "this test is purely informational; latest result: ~55ms"]
async fn measure_ping_time() {
    const NUM_ITERATIONS: usize = 10;
    let mut avg_request_time = Duration::default();

    let test_node = TestNode::default().await;
    let client_node = ClientNode::default().await;
    let client_addr = client_node.local_addr();
    test_node.node().connect(client_addr).await.unwrap();
    wait_until!(1, client_node.number_of_connected_peers().await == 1);
    wait_until!(1, test_node.node().stats().received().0 as usize == 1);

    let ping = Message::Ping(
        <Client<CurrentNetwork>>::MESSAGE_VERSION,
        CurrentNetwork::ALEO_MAXIMUM_FORK_DEPTH,
        NodeType::Client,
        Status::Ready,
        CurrentNetwork::genesis_block().hash(),
        Data::Object(CurrentNetwork::genesis_block().header().clone()),
    );

    let init_recv_count = test_node.node().stats().received().0 as usize;
    for i in 0..NUM_ITERATIONS {
        let start = Instant::now();
        test_node.unicast(client_addr, ping.clone()).unwrap().await.unwrap().unwrap();
        wait_until!(1, test_node.node().stats().received().0 as usize == init_recv_count + i + 1);
        avg_request_time += start.elapsed();
    }
    avg_request_time /= NUM_ITERATIONS as u32;

    // Display the result.
    println!("snarkOS ping time: {:?}", avg_request_time);
}
