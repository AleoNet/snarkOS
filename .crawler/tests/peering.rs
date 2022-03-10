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

use clap::Parser;
use pea2pea::Pea2Pea;
use snarkos_crawler::crawler::Crawler;
use snarkos_integration::{wait_until, TestNode};

const NUM_NODES: usize = 10;

#[tokio::test]
async fn basics() {
    // tracing_subscriber::fmt::init();

    // Prepare some collections we'll be using.
    let mut test_nodes = Vec::with_capacity(NUM_NODES);
    let mut test_node_addrs = Vec::with_capacity(NUM_NODES);

    // Start the test nodes.
    for _ in 0..NUM_NODES {
        let test_node = TestNode::default().await;
        let test_node_addr = test_node.node().listening_addr().unwrap();

        test_nodes.push(test_node);
        test_node_addrs.push(test_node_addr);
    }

    // Connect the test nodes into a linear topology.
    for (node, prev_node_addr) in test_nodes.iter().skip(1).zip(&test_node_addrs) {
        node.node().connect(*prev_node_addr).await.unwrap();
    }

    // A small delay to make sure all connections are ready.
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;

    // Double-check the topology.
    for (i, node) in test_nodes.iter().enumerate() {
        if i == 0 || i == NUM_NODES - 1 {
            assert_eq!(node.node().num_connected(), 1);
        } else {
            assert_eq!(node.node().num_connected(), 2);
        }
    }

    // Start the crawler.
    let opts = Parser::parse_from(&["snarkos_crawler_test", "--addr", "127.0.0.1:0"]);
    let crawler = Crawler::new(opts, None).await;

    // "Seed" the crawler with the address of the first node.
    crawler.known_network.add_node(test_node_addrs[0]);

    // Initialize the crawler.
    crawler.run_periodic_tasks();

    wait_until!(5, crawler.node().num_connected() == NUM_NODES);

    assert_eq!(crawler.known_network.nodes().len(), NUM_NODES);
    assert_eq!(crawler.known_network.connections().len(), NUM_NODES - 1);

    for test_node in test_nodes {
        test_node.node().shut_down().await;
    }

    wait_until!(1, crawler.node().num_connected() == 0);
}
