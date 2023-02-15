// Copyright (C) 2019-2023 Aleo Systems Inc.
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

mod common;
use common::*;

use deadline::deadline;
use peak_alloc::PeakAlloc;
use snarkos_node_router::Routing;
use snarkos_node_tcp::{
    protocols::{Disconnect, Handshake},
    P2P,
};
use snarkvm::prelude::Rng;
use snarkvm_utilities::TestRng;

use core::time::Duration;

#[global_allocator]
static PEAK_ALLOC: PeakAlloc = PeakAlloc;

#[tokio::test]
async fn test_connection_cleanups() {
    // The number of connections to start and close.
    const NUM_CONNECTIONS: usize = 10;

    // Initialize an Rng.
    let mut rng = TestRng::default();

    // Create 2 routers of random types.
    let mut nodes = Vec::with_capacity(2);
    for _ in 0..2 {
        let node = match rng.gen_range(0..4) % 4 {
            0 => beacon(0, 1).await,
            1 => client(0, 1).await,
            2 => prover(0, 1).await,
            3 => validator(0, 1).await,
            _ => unreachable!(),
        };

        nodes.push(node);
    }

    // Enable handshake handling.
    nodes[0].enable_handshake().await;
    nodes[1].enable_handshake().await;

    nodes[0].enable_disconnect().await;
    nodes[1].enable_disconnect().await;

    nodes[0].enable_listener().await;
    nodes[1].enable_listener().await;

    // We'll want to register heap use after a single connection, after the related collections are initialized.
    let mut heap_after_one_conn = None;

    // Connect and disconnect in a loop.
    for i in 0..NUM_CONNECTIONS {
        // Connect one of the nodes to the other one.
        nodes[1].connect(nodes[0].local_ip());

        // Wait until the connection is complete.
        let tcp0 = nodes[0].tcp().clone();
        let tcp1 = nodes[1].tcp().clone();
        deadline!(Duration::from_secs(3), move || tcp0.num_connected() == 1 && tcp1.num_connected() == 1);

        // Since the connectee doesn't read from the connector, it can't tell that the connector disconnected
        // from it, so it needs to disconnect from it manually.
        nodes[0].disconnect(nodes[1].local_ip());
        nodes[1].disconnect(nodes[0].local_ip());

        // Wait until the disconnect is complete.
        let tcp0 = nodes[0].tcp().clone();
        let tcp1 = nodes[1].tcp().clone();
        deadline!(Duration::from_secs(3), move || tcp0.num_connected() == 0 && tcp1.num_connected() == 0);

        // Register heap use after a single connection.
        if i == 0 {
            heap_after_one_conn = Some(PEAK_ALLOC.current_usage());
        }
    }

    // Register final heap use.
    let heap_after_loop = PEAK_ALLOC.current_usage();

    // Final heap use should equal that after the first connection.
    assert_eq!(heap_after_one_conn.unwrap(), heap_after_loop);
}
