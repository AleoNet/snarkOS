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
use rand::distributions::Standard;
use rand::thread_rng;
use rand::Rng;

use snarkos_network::Payload;
use snarkos_testing::network::spawn_2_fake_nodes;

fn send_small_messages(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    let (node0, node1) = rt.block_on(spawn_2_fake_nodes());
    let node0 = tokio::sync::Mutex::new(node0);
    let node1 = tokio::sync::Mutex::new(node1);

    let fake_block_bytes: Vec<u8> = (&mut thread_rng())
        .sample_iter(Standard)
        .take(u16::MAX as usize)
        .collect();

    c.bench_function("send_small_messages", move |b| {
        b.to_async(&rt).iter(|| async {
            let block_size: u16 = thread_rng().gen();
            let big_block = Payload::Block(fake_block_bytes[..block_size as usize].to_vec());

            // send it from node0 to node1
            node0.lock().await.write_message(&big_block).await;
            let _payload = node1.lock().await.read_payload().await.unwrap();
        })
    });
}

criterion_group!(send_receive_benches, send_small_messages);

criterion_main!(send_receive_benches);
