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
use rand::{distributions::Standard, thread_rng, Rng};

use snarkos_testing::network::{blocks::*, handshaken_node_and_peer, test_node, ConsensusSetup, TestSetup};

fn sync_blocks(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    // prepare the block provider node and a fake requester node
    let (provider, requester) = rt.block_on(handshaken_node_and_peer(TestSetup::default()));

    const NUM_BLOCKS: usize = 2;

    let blocks = TestBlocks::load(NUM_BLOCKS);
    for block in &blocks.0 {
        provider
            .environment
            .consensus_parameters()
            .receive_block(
                provider.environment.dpc_parameters(),
                &provider.environment.storage().read(),
                &mut provider.environment.memory_pool().lock(),
                &block,
            )
            .unwrap();
    }
    wait_until!(1, provider.environment.current_block_height() == NUM_BLOCKS);

    c.bench_function("sync_blocks", move |b| {
        b.to_async(&rt).iter(|| async {
            let get_sync = Payload::GetSync(vec![]);
            requester.write_message_to_stream(&get_sync);
            wait_until!(10, requester.environment.current_block_height == NUM_BLOCKS);
            requester.environment.consensus().memory_pool.lock().cleanse().unwrap();
        })
    });
}

criterion_group!(block_sync_benches, sync_blocks);

criterion_main!(block_sync_benches);
