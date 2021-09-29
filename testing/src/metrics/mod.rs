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

use crate::{
    network::{handshaken_peer, test_node, TestSetup},
    wait_until,
};
use snarkos_metrics::stats::NODE_STATS;

#[tokio::test]
async fn connect_and_disconnect_responder_side() {
    let setup: TestSetup = Default::default();
    let node = test_node(setup).await;
    node.initialize_metrics().await.unwrap();

    // The fake node connects to the node's listener...
    let peer = handshaken_peer(node.expect_local_addr()).await;

    // Needed to make sure the values have been updated.
    wait_until!(5, node.peer_book.get_connected_peer_count() == 1);

    // ...the metrics should reflect this.
    let metrics = NODE_STATS.snapshot();

    assert_eq!(metrics.connections.all_accepted, 1);
    assert_eq!(metrics.connections.connected_peers, 1);
    assert_eq!(metrics.handshakes.successes_resp, 1);

    // Break the connection by dropping the peer.
    drop(peer);

    // Wait until the node has handled the broken connection.
    wait_until!(5, node.peer_book.get_connected_peer_count() == 0);

    let metrics = NODE_STATS.snapshot();

    assert_eq!(metrics.connections.connected_peers, 0);
    assert_eq!(metrics.connections.disconnected_peers, 1);

    // Make sure the global metrics state is reset as it will leak.
    NODE_STATS.clear();
}
