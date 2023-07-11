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

use crate::common::{
    primary::start_n_primaries,
    utils::{fire_unconfirmed_solutions, fire_unconfirmed_transactions},
};

#[tokio::test]
#[ignore = "Long-running e2e test"]
async fn test_state_coherence() {
    crate::common::utils::initialize_logger(0);

    const N: u16 = 4;
    let primaries = start_n_primaries(N).await;

    // Start the tx cannons for each primary.
    for (id, primary) in primaries {
        let sender = primary.1;
        // Fire unconfirmed solutions.
        fire_unconfirmed_solutions(&sender, id);
        // Fire unconfirmed transactions.
        fire_unconfirmed_transactions(&sender, id);
    }

    // TODO(nkls): the easiest would be to assert on the anchor or bullshark's output, once
    // implemented.

    std::future::pending::<()>().await;
}
