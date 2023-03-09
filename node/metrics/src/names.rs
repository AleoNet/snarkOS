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

pub const GAUGE_NAMES: [&str; 8] = [
    blocks::HEIGHT,
    consensus::COMMITTED_CERTIFICATES,
    consensus::LAST_COMMITTED_ROUND,
    network::NETWORK_PEERS,
    peers::CANDIDATE,
    peers::CONNECTED,
    peers::RESTRICTED,
    primary::CURRENT_ROUND,
];
pub const COUNTER_NAMES: [&str; 1] = [consensus::LEADERS_ELECTED];
pub const HISTOGRAM_NAMES: [&str; 3] =
    [consensus::CERTIFICATE_COMMIT_LATENCY, consensus::COMMIT_ROUNDS_LATENCY, subscribers::CERTIFICATE_LATENCY];

pub mod blocks {
    pub const HEIGHT: &str = "snarkos_blocks_height_total";
}

pub mod peers {
    pub const CONNECTED: &str = "snarkos_peers_connected_total";
    pub const CANDIDATE: &str = "snarkos_peers_candidate_total";
    pub const RESTRICTED: &str = "snarkos_peers_restricted_total";
}

pub mod consensus {
    pub const COMMITTED_CERTIFICATES: &str = "snarkos_consensus_committed_certificates_total";
    pub const CERTIFICATE_COMMIT_LATENCY: &str = "snarkos_consensus_certificate_commit_latency_secs";
    pub const COMMIT_ROUNDS_LATENCY: &str = "snarkos_consensus_commit_rounds_latency_secs";
    pub const LEADERS_ELECTED: &str = "snarkos_consensus_leaders_elected_total";
    pub const LAST_COMMITTED_ROUND: &str = "snarkos_consensus_last_committed_round";
}

pub mod network {
    pub const NETWORK_PEERS: &str = "snarkos_network_peers_connected_total";
    pub const NETWORK_PEER_CONNECTED: &str = "snarkos_network_peer_connected";

    pub mod labels {
        pub const PEER_ID: &str = "peer_id";
    }
}

pub mod subscribers {
    pub const CERTIFICATE_LATENCY: &str = "snarkos_subscribers_certificate_latency_secs";
}

pub mod primary {
    pub const CURRENT_ROUND: &str = "snarkos_primary_current_round";
}
