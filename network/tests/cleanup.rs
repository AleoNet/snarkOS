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

use snarkos_testing::{
    network::{handshaken_peer, test_node, TestSetup},
    wait_until,
};

use peak_alloc::PeakAlloc;

// This only tests the connection acceptance side, but the cleanup logic
// is the same for connection intiation side: when the peer is disconnected,
// drop tasks dedicated to it.
#[tokio::test]
#[ignore]
async fn check_connection_task_cleanup() {
    // Start a node without sync.
    let setup = TestSetup {
        consensus_setup: None,
        peer_sync_interval: 3,
        ..Default::default()
    };
    let node = test_node(setup).await;

    // Breach the usual ulimit barriers.
    for _ in 0..10_000 {
        // Connect a peer.
        let peer = handshaken_peer(node.local_address().unwrap()).await;
        wait_until!(5, node.peer_book.get_active_peer_count() == 1);

        // Drop the peer stream.
        drop(peer);
        wait_until!(5, node.peer_book.get_active_peer_count() == 0);
    }
}

#[tokio::test]
#[ignore]
async fn check_inactive_conn_cleanup() {
    // Start a node without sync.
    let setup = TestSetup {
        consensus_setup: None,
        peer_sync_interval: 10,
        ..Default::default()
    };
    let node = test_node(setup).await;

    // A connection with a peer that will remain inactive.
    let _peer = handshaken_peer(node.local_address().unwrap()).await;

    // Wait until the connection is complete.
    wait_until!(1, node.peer_book.get_active_peer_count() == 1);

    // The peer should be dropped once `MAX_PEER_INACTIVITY_TIME_SECS` expires.
    wait_until!(
        snarkos_network::MAX_PEER_INACTIVITY_SECS as u64 * 2,
        node.peer_book.get_active_peer_count() == 0
    );
}

#[tokio::test]
#[ignore]
async fn check_node_cleanup() {
    #[global_allocator]
    static PEAK_ALLOC: PeakAlloc = PeakAlloc;

    const NUM_CONNS: usize = 4096;

    // Register the heap use before node setup.
    let initial_heap_use = PEAK_ALLOC.current_usage();

    // Start a node without sync.
    let setup = TestSetup {
        consensus_setup: None,
        ..Default::default()
    };
    let node = test_node(setup).await;

    // Register the heap use after node setup.
    let heap_after_node_setup = PEAK_ALLOC.current_usage();

    // Keep track of the average heap use.
    let mut heap_sizes = Vec::with_capacity(NUM_CONNS);

    // Don't count the vector's heap allocation in the measurments taken in the loop.
    let mem_deduction = PEAK_ALLOC.current_usage() - heap_after_node_setup;

    // Due to tokio channel internals, a small heap bump occurs after 32 calls to
    // `mpsc::Sender::send`. If it weren't for that, heap use after the 1st connection (i == 0)
    // would be registered instead. See: https://github.com/tokio-rs/tokio/issues/4031.
    let mut heap_after_32_conns = 0;

    for i in 0..NUM_CONNS {
        // Connect a peer.
        let peer = handshaken_peer(node.local_address().unwrap()).await;
        wait_until!(5, node.peer_book.get_active_peer_count() == 1);

        // Drop the peer stream.
        drop(peer);
        wait_until!(5, node.peer_book.get_active_peer_count() == 0);

        // Register the current heap size, after the connection has been dropped.
        let current_heap_size = PEAK_ALLOC.current_usage() - mem_deduction;

        // Register the heap size once the 33rd connection is established and dropped.
        if i == 32 {
            heap_after_32_conns = current_heap_size;
        }

        // Save the current heap size, used later on to calculate the average.
        heap_sizes.push(current_heap_size);
    }

    // Calculate the average heap use from the collection of tracked heap sizes.
    let avg_heap_use = heap_sizes.iter().sum::<usize>() / heap_sizes.len();

    // Drop the vector of heap sizes so as to leave further measurements unaffected.
    drop(heap_sizes);

    // Check the final heap use and calculate its total growth (excluding the tokio bump).
    let final_heap_use = PEAK_ALLOC.current_usage();
    let heap_growth = final_heap_use - heap_after_32_conns;

    // Division is safe since the heap use cannot be 0, absolute value isn't necessary since heap
    // use is always positive.
    let growth_percentage = 100.0 * (final_heap_use as f64 - heap_after_32_conns as f64) / heap_after_32_conns as f64;

    println!("---- heap use summary ----\n");
    println!("before node setup:     {}kB", initial_heap_use / 1000);
    println!("after node setup:      {}kB", heap_after_node_setup / 1000);
    println!("after 32 connections:  {}kB", heap_after_32_conns / 1000);
    println!("after {} connections: {}kB", NUM_CONNS, final_heap_use / 1000);
    println!();
    println!("average use: {}kB", avg_heap_use / 1000);
    println!("maximum use: {}kB", PEAK_ALLOC.peak_usage() / 1000);
    println!("growth:      {}B, {:.2}%", heap_growth, growth_percentage);
}
