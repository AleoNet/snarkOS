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

use parking_lot::RwLock;
use peak_alloc::PeakAlloc;
use snarkos::{handle_listener, Ledger as SnarkosLedger};
use snarkvm::{
    console::network::Testnet3,
    prelude::{BlockMemory, BlockStore, Ledger as SnarkvmLedger, PrivateKey, ProgramMemory, ProgramStore},
    utilities::TestRng,
};
use std::{sync::Arc, time::Duration};
use tokio::{
    io::AsyncReadExt,
    net::{TcpListener, TcpStream},
    time::{sleep, timeout},
};

#[global_allocator]
static PEAK_ALLOC: PeakAlloc = PeakAlloc;

// This value doesn't need to be large, but it should be at least 5.
// The test passed locally with values up to 1000.
const NUM_CONNS: usize = 20;

type CurrentNetwork = Testnet3;

#[tokio::test]
#[ignore = "This test requires the Ledger to use the in-memory storage,\
    which is currently not exposed. It also doesn't need to be\
    run unless peering/connection-handling logic is altered."]
async fn connections_dont_leak() {
    // Prepare an in-memory test ledger.
    let private_key = PrivateKey::<CurrentNetwork>::new(&mut TestRng::default()).unwrap();
    let internal_ledger = Arc::new(RwLock::new(
        SnarkvmLedger::from(
            BlockStore::<_, BlockMemory<_>>::open(None).unwrap(),
            ProgramStore::<_, ProgramMemory<_>>::open(None).unwrap(),
        )
        .unwrap(),
    ));
    let ledger = SnarkosLedger::from(internal_ledger, private_key).unwrap();

    // Create a node connection handler.
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let node_addr = listener.local_addr().unwrap();
    let _conn_handler = handle_listener::<CurrentNetwork>(listener, ledger.clone());

    // Register initial heap use and prepare leak-checking objects.
    let initial_heap_use = PEAK_ALLOC.current_usage();
    let mut prev_mem_use = initial_heap_use;
    let mut curr_mem_use;
    let mut last_heap_bump = 0;

    // Simulate NUM_COMS inbound connections with the node.
    for i in 1..=NUM_CONNS {
        // Start a connection.
        let mut test_conn = TcpStream::connect(node_addr).await.unwrap();

        // Wait until the "peer" receives a Ping, but only half the time.
        if i % 2 == 0 {
            timeout(Duration::from_millis(100), async move {
                let _ = test_conn.read(&mut [0u8; 128]).await;
            })
            .await
            .unwrap();
        }

        // Check if heap use increased. It's ok if it happens once or twice,
        // but should no longer happen after several connections.
        curr_mem_use = PEAK_ALLOC.current_usage();
        if curr_mem_use > prev_mem_use {
            prev_mem_use = curr_mem_use;
            last_heap_bump = i;
        }
    }

    // A short sleep to allow all the auto-cleanups to happen.
    sleep(Duration::from_secs(1)).await;

    // In local tests the last heap bump happened after 2 connections.
    println!(
        "Simulated {NUM_CONNS} connections. Heap use change: +{}B.",
        PEAK_ALLOC.current_usage().saturating_sub(initial_heap_use)
    );
    println!("Last heap bump happened after {last_heap_bump} connection(s).");

    // Ensure that the final connection didn't cause a bump in heap use.
    assert!(last_heap_bump != NUM_CONNS);
}
