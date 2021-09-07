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
use tokio::{runtime, task};

use snarkos_testing::{network::test_consensus, sync::TestBlocks};

fn processing_full_blocks(c: &mut Criterion) {
    let rt = runtime::Builder::new_multi_thread()
        .enable_all()
        .max_blocking_threads(std::cmp::max(num_cpus::get().saturating_sub(2), 1))
        .build()
        .unwrap();

    let consensus = rt.block_on(test_consensus(Default::default())).consensus;

    let mut group = c.benchmark_group("processing_full_blocks");

    let blocks = TestBlocks::load(Some(100), "test_blocks_100_1");

    // Import a range of consecutive canon blocks, resetting the ledger between runs
    let num_canon_blocks = 10;
    group.bench_with_input(
        BenchmarkId::new("consecutive_canon_blocks", num_canon_blocks),
        &num_canon_blocks,
        |b, _size| {
            b.to_async(&rt).iter(|| {
                let consensus_clone = consensus.clone();
                let blocks_clone = blocks.0[..num_canon_blocks].to_vec();

                async move {
                    for block in blocks_clone {
                        consensus_clone.receive_block(block).await;
                    }
                    consensus_clone.reset().await.unwrap();
                }
            })
        },
    );

    // Import a range of orphan blocks, resetting the ledger between runs but not after the last one)
    let num_orphan_blocks = 10;
    group.bench_with_input(
        BenchmarkId::new("orphan_blocks", num_orphan_blocks),
        &num_orphan_blocks,
        |b, _size| {
            b.to_async(&rt).iter(|| {
                let blocks_clone = blocks.0[1..][..num_orphan_blocks].to_vec();
                let consensus_clone = consensus.clone();

                let mut tasks = Vec::with_capacity(num_orphan_blocks);
                async move {
                    consensus_clone.reset().await.unwrap();
                    for block in blocks_clone {
                        let consensus_clone2 = consensus_clone.clone();
                        tasks.push(task::spawn(async move { consensus_clone2.receive_block(block).await }));
                    }
                    futures::future::join_all(tasks).await;
                }
            })
        },
    );

    // Import a range of duplicate blocks, not resetting the ledger at all
    let num_duplicates = num_orphan_blocks;
    group.bench_with_input(
        BenchmarkId::new("duplicate_blocks", num_duplicates),
        &num_duplicates,
        |b, _size| {
            b.to_async(&rt).iter(|| {
                let blocks_clone = blocks.0[1..][..num_duplicates].to_vec();
                let consensus_clone = consensus.clone();

                let mut tasks = Vec::with_capacity(num_duplicates);
                async move {
                    for block in blocks_clone {
                        let consensus_clone2 = consensus_clone.clone();
                        tasks.push(task::spawn(async move { consensus_clone2.receive_block(block).await }));
                    }
                    futures::future::join_all(tasks).await;
                }
            })
        },
    );

    group.finish();
}

criterion_group!(block_processing_benches, processing_full_blocks);

criterion_main!(block_processing_benches);
