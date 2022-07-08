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

use crate::Server;
use snarkos_environment::{CurrentNetwork, TestEnvironment};
use snarkos_network::DisconnectReason;

use clap::Parser;
use std::{fs, net::SocketAddr};

/// A snarkOS Node used for local testing.
pub struct TestNode {
    pub server: Server<CurrentNetwork, TestEnvironment<CurrentNetwork>>,
}

impl TestNode {
    /// Returns the local listening address of the node.
    pub fn local_addr(&self) -> SocketAddr {
        self.server.local_ip()
    }

    /// Returns the list of connected peers of the node.
    pub async fn connected_peers(&self) -> Vec<SocketAddr> {
        self.server.state.peers().connected_peers().await
    }

    /// Returns the number of connected peers of the node.
    pub async fn number_of_connected_peers(&self) -> usize {
        self.server.state.peers().number_of_connected_peers().await
    }

    /// Resets the node's known peers. This is practical, as it makes the node not reconnect
    /// to known peers in test cases where it's undesirable.
    pub async fn reset_known_peers(&self) {
        self.server.state.peers().reset_known_peers().await
    }

    /// Attempts to connect the node to the given address.
    pub async fn connect(&self, addr: SocketAddr) -> anyhow::Result<()> {
        self.server.connect_to(addr).await
    }

    /// Disconnects the node from the given address.
    pub async fn disconnect(&self, addr: SocketAddr) {
        self.server.disconnect_from(addr, DisconnectReason::NoReasonGiven).await
    }

    /// Starts a snarkOS node with all the default characteristics from `TestNode::with_args`.
    pub async fn default() -> Self {
        TestNode::with_args(&["--node", "127.0.0.1:0"]).await
    }

    /// Starts a snarkOS node with a manually specified ip and port.
    pub(crate) async fn new_with_custom_ip(ip: &str, port: u16) -> Self {
        TestNode::with_args(&["--node", &format!("{ip}:{port}")]).await
    }

    /// Starts a snarkOS node with a local address and the RPC server disabled;
    /// extra arguments may be passed via `extra_args`.
    pub async fn with_args(extra_args: &[&str]) -> Self {
        let permanent_args = &["snarkos", "--norpc"];
        let combined_args = permanent_args.iter().chain(extra_args.iter());
        let config = crate::Node::parse_from(combined_args);
        let server = Server::<CurrentNetwork, TestEnvironment<CurrentNetwork>>::initialize(&config, None)
            .await
            .unwrap();

        TestNode { server }
    }

    pub async fn shut_down(&self) {
        self.server.shut_down().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn nodes_can_connect_to_each_other() {
        // Start 2 snarkOS nodes.
        let test_node1 = TestNode::new_with_custom_ip("127.0.0.1", 3000).await;
        let test_node2 = TestNode::new_with_custom_ip("127.0.0.1", 4000).await;

        // Connect one to the other.
        test_node1.connect(test_node2.local_addr()).await.unwrap();

        assert_eq!(test_node1.number_of_connected_peers().await, 1);
        assert!(test_node1.connected_peers().await.contains(&test_node2.local_addr()));

        assert_eq!(test_node2.number_of_connected_peers().await, 1);
        assert!(test_node2.connected_peers().await.contains(&test_node1.local_addr()));
    }
}
