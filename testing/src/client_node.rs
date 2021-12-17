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

use snarkos::{helpers::Tasks, Client, Server};
use snarkvm::dpc::testnet2::Testnet2;

use std::{fs, net::SocketAddr};
use structopt::StructOpt;

/// A facade for a snarkOS client node.
pub struct ClientNode {
    pub server: Server<Testnet2, Client<Testnet2>>,
}

impl ClientNode {
    /// Returns the local listening address of the node.
    pub fn local_addr(&self) -> SocketAddr {
        self.server.local_ip()
    }

    /// Returns the list of connected peers of the node.
    pub async fn connected_peers(&self) -> Vec<SocketAddr> {
        self.server.peers().connected_peers().await
    }

    /// Resets the node's known peers. This is practical, as it makes the node not reconnect
    /// to known peers in test cases where it's undesirable.
    pub async fn reset_known_peers(&self) {
        self.server.peers().reset_known_peers().await
    }

    /// Attempts to connect the node to the given address.
    pub async fn connect(&self, addr: SocketAddr) -> anyhow::Result<()> {
        self.server.connect_to(addr).await
    }

    /// Starts a snarkOS node with all the default characteristics from `ClientNode::with_args`.
    pub async fn default() -> Self {
        ClientNode::with_args(&["--node", "127.0.0.1:0"]).await
    }

    /// Starts a snarkOS node with a local address and the RPC server disabled;
    /// extra arguments may be passed via `extra_args`.
    pub async fn with_args(extra_args: &[&str]) -> Self {
        let permanent_args = &["snarkos", "--norpc"];
        let combined_args = permanent_args.iter().chain(extra_args.iter());
        let config = snarkos::Node::from_iter(combined_args);
        let server = Server::<Testnet2, Client<Testnet2>>::initialize(&config, None, None, Tasks::new())
            .await
            .unwrap();

        ClientNode { server }
    }
}

// Remove the storage artifacts after each test.
impl Drop for ClientNode {
    fn drop(&mut self) {
        // TODO (howardwu): @ljedrz to implement a wrapping scope for Display within Node/Server.
        #[allow(unused_must_use)]
        {
            self.server.shut_down();
        }

        let db_path = format!("/tmp/snarkos-test-ledger-{}", self.local_addr().port());
        assert!(
            fs::remove_dir_all(&db_path).is_ok(),
            "Storage cleanup failed! The expected path \"{}\" doesn't exist",
            db_path
        );
    }
}
