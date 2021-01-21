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

use snarkos_network::Server;
use snarkos_testing::{network::star_topology, wait_until};

#[tokio::test]
async fn star() {
    let nodes = star_topology(100).await;
    let core = nodes.first().unwrap();

    assert!(core.environment.is_bootnode());
    let has_n_peers = |node: Server, n: u16| node.peers.number_of_connected_peers() == n;
    wait_until!(5, has_n_peers(core.clone(), 99));
}
