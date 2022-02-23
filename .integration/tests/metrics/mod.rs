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

use crate::wait_until;

use snarkos_metrics as metrics;
use snarkos_testing::{ClientNode, TestNode};

use pea2pea::Pea2Pea;

#[tokio::test]
async fn metrics_initialization() {
    // Start a test node.
    let _test_node = TestNode::default().await;

    // Initialise the metrics, we need to call this manually in tests.
    let snapshotter = metrics::initialize();

    // Start a snarkOS node.
    let _client_node = ClientNode::default().await;

    // Verify the metrics have been properly initialised, expect the block height to be set.
    assert_eq!(
        metrics::get_metric(&snapshotter, metrics::blocks::HEIGHT),
        metrics::MetricVal::Gauge(0.0)
    );
    assert_eq!(
        metrics::get_metric(&snapshotter, metrics::peers::CONNECTED),
        metrics::MetricVal::Gauge(0.0)
    );
    assert_eq!(
        metrics::get_metric(&snapshotter, metrics::peers::CANDIDATE),
        metrics::MetricVal::Gauge(0.0)
    );
    assert_eq!(
        metrics::get_metric(&snapshotter, metrics::peers::RESTRICTED),
        metrics::MetricVal::Gauge(0.0)
    );

    // Clear the recorder to avoid the global state bleeding into other tests.
    metrics::clear_recorder();
}

#[tokio::test]
async fn connect_disconnect() {
    // Start a test node.
    let test_node = TestNode::default().await;

    // Initialise the metrics, we need to call this manually in tests.
    let snapshotter = metrics::initialize();

    // Start a snarkOS node.
    let client_node = ClientNode::default().await;

    for _ in 0..3 {
        // The test node should be able to connect to the snarkOS node.
        test_node.node().connect(client_node.local_addr()).await.unwrap();

        // Double-check with the snarkOS node.
        wait_until!(1, client_node.connected_peers().await.len() == 1);

        // Check the metrics.
        assert_eq!(
            metrics::get_metric(&snapshotter, metrics::peers::CONNECTED),
            metrics::MetricVal::Gauge(1.0)
        );
        assert_eq!(
            metrics::get_metric(&snapshotter, metrics::peers::CANDIDATE),
            metrics::MetricVal::Gauge(0.0)
        );
        assert_eq!(
            metrics::get_metric(&snapshotter, metrics::peers::RESTRICTED),
            metrics::MetricVal::Gauge(0.0)
        );

        // Shut down the node, force a disconnect.
        test_node.node().disconnect(client_node.local_addr()).await;

        wait_until!(1, client_node.connected_peers().await.len() == 0);

        // Check the metrics.
        assert_eq!(
            metrics::get_metric(&snapshotter, metrics::peers::CONNECTED),
            metrics::MetricVal::Gauge(0.0)
        );
        assert_eq!(
            metrics::get_metric(&snapshotter, metrics::peers::CANDIDATE),
            metrics::MetricVal::Gauge(1.0)
        );
        assert_eq!(
            metrics::get_metric(&snapshotter, metrics::peers::RESTRICTED),
            metrics::MetricVal::Gauge(0.0)
        );
    }

    // Clear the recorder to avoid the global state bleeding into other tests.
    metrics::clear_recorder();
}
