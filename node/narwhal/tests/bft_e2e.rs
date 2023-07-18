// Copyright (C) 2019-2023 Aleo Systems Inc.
// This file is part of the snarkOS library.

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at:
// http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

mod common;

use crate::common::primary::{TestNetwork, TestNetworkConfig};

#[tokio::test(flavor = "multi_thread")]
#[ignore = "long-running e2e test"]
async fn test_state_coherence() {
    const N: u16 = 4;
    let mut network = TestNetwork::new(TestNetworkConfig {
        num_nodes: N,
        bft: true,
        connect_all: true,
        fire_cannons: true,
        // Set this to Some(0..=4) to see the logs.
        log_level: Some(0),
        log_connections: true,
    });

    network.start().await;

    std::future::pending::<()>().await;
}
