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

pub const GAUGE_NAMES: [&str; 5] =
    [blocks::HEIGHT, blocks::TRANSACTIONS, peers::CONNECTED, peers::CANDIDATE, peers::RESTRICTED];

pub mod blocks {
    pub const HEIGHT: &str = "snarkos_blocks_height_total";
    pub const TRANSACTIONS: &str = "snarkos_blocks_transactions_total";
}

pub mod primary {
    pub const CURRENT_ROUND: &str = "snarkos_primary_current_round";
}

pub mod peers {
    pub const CONNECTED: &str = "snarkos_peers_connected_total";
    pub const CANDIDATE: &str = "snarkos_peers_candidate_total";
    pub const RESTRICTED: &str = "snarkos_peers_restricted_total";
}
