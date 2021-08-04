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
use itertools::Itertools;

use snarkos_network::{Connection, NetworkMetrics};

use std::{collections::HashSet, net::SocketAddr};

fn compute_dense_metrics(c: &mut Criterion) {
    // These won't be the exact counts since it isn't possible to generate perfectly dense
    // connection matrices for all connection counts, the exact count will be displayed in the
    // benchmark id.
    let sample_sizes = vec![25000, 50000, 100000, 200000, 400000, 800000];
    let connections: Vec<HashSet<Connection>> = sample_sizes
        .iter()
        .map(|size| generate_dense_connections(*size))
        .collect();

    let mut group = c.benchmark_group("compute_dense_metrics");

    for sample in connections {
        let size = sample.len();
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &_size| {
            b.iter(|| NetworkMetrics::new(sample.clone()))
        });
    }

    group.finish();
}

fn generate_dense_connections(n: usize) -> HashSet<Connection> {
    // The computation is likely to be faster for sparser connection sets, this is the worst case scenario.

    // Since `n` is the connection count which is linked to node count with `n = m * (m - 1) / 2`, we need `(1 + sqrt(1 + 8n)) / 2` nodes to create a dense
    // connection mesh (we only keep the positive variant of the quadratic formula since `1 + 8n` is always >= 1).
    let m = ((1.0 + (1.0 + 8.0 * n as f64).sqrt()) / 2.0).floor() as usize;

    // Create `m` addresses (using localhost since the actual ips are unimportant).
    let addrs: Vec<SocketAddr> = (0..m)
        .map(|i| SocketAddr::new("127.0.0.1".parse().unwrap(), i as u16))
        .collect();

    const COMBINATION_SIZE: usize = 2;
    addrs
        .iter()
        .combinations(COMBINATION_SIZE)
        .map(|pairs| Connection::new(*pairs[0], *pairs[1]))
        .collect()
}

criterion_group!(network_metrics_benches, compute_dense_metrics);
criterion_main!(network_metrics_benches);
