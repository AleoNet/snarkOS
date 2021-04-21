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

use criterion::*;

use snarkos_network::Payload;
use snarkos_testing::network::{blocks::*, handshaken_node_and_peer, TestSetup};

// FIXME(ljedrz/nkls): gracefully shut the node and peer down once shutdown is implemented
fn providing_sync_blocks(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    let test_setup = TestSetup {
        tokio_handle: Some(rt.handle().clone()),
        ..Default::default()
    };

    // prepare the block provider node and a fake requester node
    let (provider, requester) = rt.block_on(handshaken_node_and_peer(test_setup));
    let requester = tokio::sync::Mutex::new(requester);

    const NUM_BLOCKS: usize = 10;

    let blocks = TestBlocks::load(NUM_BLOCKS);
    for block in &blocks.0 {
        provider.expect_consensus().consensus.receive_block(&block).unwrap();
    }
    assert_eq!(provider.expect_consensus().current_block_height() as usize, NUM_BLOCKS);

    c.bench_function("providing_sync_blocks", move |b| {
        b.to_async(&rt).iter(|| async {
            let get_sync = Payload::GetSync(vec![]);
            requester.lock().await.write_message(&get_sync).await;

            // requester obtains hashes
            let hashes = if let Payload::Sync(hashes) = requester.lock().await.read_payload().await.unwrap() {
                hashes
            } else {
                unreachable!();
            };

            let get_blocks = Payload::GetBlocks(hashes);
            requester.lock().await.write_message(&get_blocks).await;

            let mut sync_blocks_count = 0;
            loop {
                let payload = requester.lock().await.read_payload().await.unwrap();
                if let Payload::SyncBlock(_) = payload {
                    sync_blocks_count += 1;
                }
                if sync_blocks_count == NUM_BLOCKS {
                    break;
                }
            }
        })
    });
}

criterion_group!(block_sync_benches, providing_sync_blocks);

criterion_main!(block_sync_benches);
