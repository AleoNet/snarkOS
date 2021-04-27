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

#[tokio::test]
#[ignore]
async fn check_node_cleanup() {
    #[global_allocator]
    static PEAK_ALLOC: PeakAlloc = PeakAlloc;

    // Start a node without sync.
    let setup = TestSetup {
        consensus_setup: None,
        ..Default::default()
    };
    let node = test_node(setup).await;

    // Keep track of peak heap throughout the iterations.
    let mut peak_heap = PEAK_ALLOC.peak_usage();
    let mut peak_heap_post_1st_conn = 0;

    // Note: `ulimit` will be a limiting factor in how many peer connections can be opened.
    for i in 0u16..4096 {
        // Connect a peer.
        let peer = handshaken_peer(node.local_address().unwrap()).await;
        wait_until!(5, node.peer_book.number_of_connected_peers() == 1);

        // Drop the peer stream.
        drop(peer);
        wait_until!(5, node.peer_book.number_of_connected_peers() == 0);

        // Register heap bump after the connection was dropped.
        let curr_peak = PEAK_ALLOC.peak_usage();

        // println!(
        //     "heap bump: {}B at i={} (+{}%)",
        //     curr_peak,
        //     i,
        //     (curr_peak as f64 / peak_heap as f64 - 1.0) * 100.0
        // );

        if curr_peak > peak_heap {
            peak_heap = curr_peak;
        }

        // Register first peak heap for growth evaluation.
        if i == 0 {
            peak_heap_post_1st_conn = curr_peak;
        }
    }

    // Register peak heap use.
    let max_heap_use = PEAK_ALLOC.peak_usage();
    println!("peak heap use: {:.2}KiB", max_heap_use as f64 / 1024.0);

    // Allocation growth should be under 5%.
    let alloc_growth = max_heap_use as f64 / peak_heap_post_1st_conn as f64;
    println!("alloc growth: {}", alloc_growth);
    assert!(alloc_growth < 1.05);
}
