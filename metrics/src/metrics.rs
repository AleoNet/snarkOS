// Copyright (C) 2019-2020 Aleo Systems Inc.
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

use crate::prometheus::{self, metrics_handler, CONNECTED_PEERS};

use warp::Filter;

#[derive(Default)]
pub struct Metrics {}

impl Metrics {
    pub fn new() -> Self {
        Default::default()
    }

    pub async fn start(self) {
        prometheus::initialize();
        // Initialize the route handlers
        let metrics_route = warp::path!("metrics").and_then(metrics_handler);
        // Serve them
        println!("Started on port 8080");
        warp::serve(metrics_route).run(([0, 0, 0, 0], 8080)).await;
    }

    pub fn get_connected_peers() -> i64 {
        CONNECTED_PEERS.get()
    }

    pub fn connected_peers_inc() {
        CONNECTED_PEERS.inc();
    }

    pub fn connected_peers_dec() {
        CONNECTED_PEERS.dec();
    }
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use snarkos_derives::test_with_metrics;

    use serial_test::serial;

    #[test_with_metrics]
    fn test_connected_peers() {
        // Increment by 1.
        Metrics::connected_peers_inc();
        assert_eq!(1, Metrics::get_connected_peers());

        // Increment by 1.
        Metrics::connected_peers_inc();
        assert_eq!(2, Metrics::get_connected_peers());

        // Decrement by 1.
        Metrics::connected_peers_dec();
        assert_eq!(1, Metrics::get_connected_peers());

        // Increment by 1.
        Metrics::connected_peers_inc();
        assert_eq!(2, Metrics::get_connected_peers());

        // Decrement by 2.
        Metrics::connected_peers_dec();
        Metrics::connected_peers_dec();
        assert_eq!(0, Metrics::get_connected_peers());

        // Decrement by 1.
        Metrics::connected_peers_dec();
        assert_eq!(-1, Metrics::get_connected_peers());

        // Increment by 1.
        Metrics::connected_peers_inc();
        assert_eq!(0, Metrics::get_connected_peers());
    }
}
