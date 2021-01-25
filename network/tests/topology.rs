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

use snarkos_testing::{network::star_topology, wait_until};

#[tokio::test(flavor = "multi_thread")]
async fn star() {
    // Note: `ulimit` can be a limiting factor here.
    let nodes = star_topology(5).await;
    let core = nodes.first().unwrap();

    assert!(core.environment.is_bootnode());
    wait_until!(5, core.peers.number_of_connected_peers() == 4);
}
