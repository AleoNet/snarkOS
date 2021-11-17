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

use structopt::StructOpt;
use tokio::net::TcpListener;

use std::{fs, net::SocketAddr};

/// A facade for a snarkOS node.
// FIXME: there's not much room for introspection right now; it should be implemented in the existing API.
pub struct SnarkosNode {
    pub addr: SocketAddr,
}

impl SnarkosNode {
    /// Starts a snarkOS node with all the default characteristics from `SnarkosNode::with_args` plus any
    /// available port number picked for the listening address.
    pub async fn default() -> Self {
        // Procure a free port number for the snarkOS node.
        // FIXME: due to there being a delay between the port's discovery and its binding by the node, this
        // method can cause an `AddrInUse` error to occur when multiple tests are run at the same time; only
        // introspection of a ready node can fully avoid this.
        let free_port = {
            let temp_socket = TcpListener::bind("127.0.0.1:0").await.unwrap();
            temp_socket.local_addr().unwrap().port()
        };

        // Start a snarkOS node with that port.
        SnarkosNode::with_args(&["--node", &free_port.to_string()]).await
    }

    /// Starts a snarkOS node with a local address and the RPC server disabled; extra arguments can be passed
    /// via `extra_args`.
    pub async fn with_args(extra_args: &[&str]) -> Self {
        let permanent_args = &["snarkos", "--disable-rpc", "--ip", "127.0.0.1"];
        let combined_args = permanent_args.iter().chain(extra_args.iter());

        snarkos::Node::from_iter(combined_args).start().await.unwrap();

        let mut port = None;

        for (i, arg) in extra_args.iter().enumerate() {
            if *arg == "--node" {
                port = extra_args.get(i + 1);
                break;
            }
        }

        let addr = format!("127.0.0.1:{}", port.unwrap().parse::<u16>().unwrap()).parse().unwrap();

        SnarkosNode { addr }
    }
}

// Remove the storage artifacts after each test.
impl Drop for SnarkosNode {
    fn drop(&mut self) {
        let db_path = format!("{}/.ledger-{}", env!("CARGO_MANIFEST_DIR"), self.addr.port());

        if fs::remove_dir_all(&db_path).is_err() {
            panic!("Storage cleanup failed! The expected path \"{}\" doesn't exist", db_path);
        }
    }
}
