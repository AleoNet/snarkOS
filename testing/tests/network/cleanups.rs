// Copyright (C) 2019-2021 Aleo Systems Inc.
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

use pea2pea::Pea2Pea;
use peak_alloc::PeakAlloc;
use snarkos_testing::{SnarkosNode, TestNode};

#[tokio::test(flavor = "multi_thread")]
#[ignore = "this test is currently non-deterministic; needs more tinkering"]
async fn inbound_connect_and_disconnect_doesnt_leak() {
    // Configure a custom allocator that will measure memory use.
    #[global_allocator]
    static PEAK_ALLOC: PeakAlloc = PeakAlloc;

    // Register initial memory use.
    let initial_mem = PEAK_ALLOC.current_usage();

    // Start a test node.
    let test_node = TestNode::default().await;

    // Register memory use by the test node.
    let post_test_node_mem = PEAK_ALLOC.current_usage();
    println!("Memory increase from the test node: {}B", post_test_node_mem - initial_mem);

    // Start a snarkOS node.
    let snarkos_node = SnarkosNode::default().await;

    // Register memory use before any connections.
    let pre_connection_mem = PEAK_ALLOC.current_usage();
    println!(
        "Memory increase from the snarkOS node: {}B",
        pre_connection_mem - post_test_node_mem
    );

    // Connect the test node to the snarkOS node (inbound for snarkOS).
    test_node.node().connect(snarkos_node.addr).await.unwrap();

    // Disconnect the test node from the snarkOS node.
    assert!(test_node.node().disconnect(snarkos_node.addr).await);

    // Measure memory use after the 1st connect and disconnect.
    let first_conn_mem = PEAK_ALLOC.current_usage();
    println!(
        "Memory increase from a single inbound connection: {}B",
        first_conn_mem - pre_connection_mem
    );

    // Perform a connect and disconnect a few more times.
    for _ in 0..5 {
        test_node.node().connect(snarkos_node.addr).await.unwrap();
        assert!(test_node.node().disconnect(snarkos_node.addr).await);
    }

    // Measure memory use after the repeated connections.
    let final_mem = PEAK_ALLOC.current_usage();

    // Check if there is a connection-related leak.
    let leaked_mem = final_mem.saturating_sub(first_conn_mem);
    assert_eq!(leaked_mem, 0);
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "currently not possible to connect on demand"]
async fn outbound_connect_and_disconnect_doesnt_leak() {}
