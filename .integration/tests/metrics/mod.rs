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

use snarkos_integration::{wait_until, ClientNode, TestNode};
use snarkos_metrics as metrics;

use pea2pea::Pea2Pea;

#[tokio::test]
async fn metrics_initialization() {
    // Initialise the metrics, we need to call this manually in tests.
    let metrics = metrics::TestMetrics::default();

    // Verify the metrics have been properly initialised, expect the block height to be set.
    assert_eq!(metrics.get_val_for(metrics::blocks::HEIGHT), metrics::MetricVal::Gauge(0.0));
    assert_eq!(metrics.get_val_for(metrics::peers::CONNECTED), metrics::MetricVal::Gauge(0.0));
    assert_eq!(metrics.get_val_for(metrics::peers::CANDIDATE), metrics::MetricVal::Gauge(0.0));
    assert_eq!(metrics.get_val_for(metrics::peers::RESTRICTED), metrics::MetricVal::Gauge(0.0));
}

#[tokio::test]
async fn connect_disconnect() {
    // Start a test node.
    let test_node = TestNode::default().await;

    // Initialise the metrics, we need to call this manually in tests.
    let metrics = metrics::TestMetrics::default();

    // Start a snarkOS node.
    let client_node = ClientNode::default().await;

    for _ in 0..3 {
        // The test node should be able to connect to the snarkOS node.
        test_node.node().connect(client_node.local_addr()).await.unwrap();

        // Double-check with the snarkOS node.
        wait_until!(1, client_node.connected_peers().await.len() == 1);

        // Check the metrics.
        assert_eq!(metrics.get_val_for(metrics::peers::CONNECTED), metrics::MetricVal::Gauge(1.0));
        assert_eq!(metrics.get_val_for(metrics::peers::CANDIDATE), metrics::MetricVal::Gauge(0.0));
        assert_eq!(metrics.get_val_for(metrics::peers::RESTRICTED), metrics::MetricVal::Gauge(0.0));

        // Shut down the node, force a disconnect.
        test_node.node().disconnect(client_node.local_addr()).await;

        wait_until!(1, client_node.connected_peers().await.is_empty());

        // Check the metrics.
        assert_eq!(metrics.get_val_for(metrics::peers::CONNECTED), metrics::MetricVal::Gauge(0.0));
        assert_eq!(metrics.get_val_for(metrics::peers::CANDIDATE), metrics::MetricVal::Gauge(1.0));
        assert_eq!(metrics.get_val_for(metrics::peers::RESTRICTED), metrics::MetricVal::Gauge(0.0));
    }
}
