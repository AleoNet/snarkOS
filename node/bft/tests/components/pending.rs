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

use crate::components::sample_ledger;
use snarkos_node_bft::helpers::max_redundant_requests;
use snarkvm::prelude::TestRng;

#[test]
fn test_max_redundant_requests() {
    const NUM_NODES: u16 = 10;

    // Initialize the RNG.
    let rng = &mut TestRng::default();
    // Sample a ledger.
    let ledger = sample_ledger(NUM_NODES, rng);
    // Ensure the maximum number of redundant requests is correct and consistent across iterations.
    assert_eq!(max_redundant_requests(ledger, 0), 34, "Update me if the formula changes");
}
