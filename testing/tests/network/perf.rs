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

use snarkos_testing::ClientNode;

use std::time::Instant;

#[tokio::test]
#[ignore = "this test is purely informational; latest result: ~675ms"]
async fn measure_node_startup() {
    let now = Instant::now();

    // Start a snarkOS node.
    let _snarkos_node = ClientNode::default().await;

    // Display the result.
    println!("snarkOS start-up time: {}ms", now.elapsed().as_millis());
}
