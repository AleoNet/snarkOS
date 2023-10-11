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

pub const GAUGE_NAMES: [&str; 9] = [
    blocks::HEIGHT,
    blocks::TRANSACTIONS,
    consensus::COMMITTED_CERTIFICATES,
    consensus::LAST_COMMITTED_ROUND,
    peers::CONNECTED,
    peers::CANDIDATE,
    peers::RESTRICTED,
    primary::CURRENT_ROUND,
    network::TCP_CONNECTIONS,
];

pub const HISTOGRAM_NAMES: [&str; 6] = [
    consensus::CERTIFICATE_COMMIT_LATENCY,
    consensus::COMMIT_ROUNDS_LATENCY,
    network::NOISE_CODEC_ENCRYPTION_TIME,
    network::NOISE_CODEC_DECRYPTION_TIME,
    network::NOISE_CODEC_ENCRYPTION_SIZE,
    network::NOISE_CODEC_DECRYPTION_SIZE,
];

pub const COUNTER_NAMES: [&str; 1] = [consensus::LEADERS_ELECTED];

pub mod blocks {
    pub const HEIGHT: &str = "snarkos_blocks_height_total";
    pub const TRANSACTIONS: &str = "snarkos_blocks_transactions_total";
}

pub mod consensus {
    pub const COMMITTED_CERTIFICATES: &str = "snarkos_consensus_committed_certificates_total";
    pub const CERTIFICATE_COMMIT_LATENCY: &str = "snarkos_consensus_certificate_commit_latency_secs";
    pub const COMMIT_ROUNDS_LATENCY: &str = "snarkos_consensus_commit_rounds_latency_secs";
    pub const LEADERS_ELECTED: &str = "snarkos_consensus_leaders_elected_total";
    pub const LAST_COMMITTED_ROUND: &str = "snarkos_consensus_last_committed_round";
}

pub mod primary {
    pub const CURRENT_ROUND: &str = "snarkos_primary_current_round";
}

pub mod peers {
    pub const CONNECTED: &str = "snarkos_peers_connected_total";
    pub const CANDIDATE: &str = "snarkos_peers_candidate_total";
    pub const RESTRICTED: &str = "snarkos_peers_restricted_total";
}

pub mod network {
    pub const NOISE_CODEC_ENCRYPTION_TIME: &str = "snarkos_network_noise_codec_encryption_micros";
    pub const NOISE_CODEC_DECRYPTION_TIME: &str = "snarkos_network_noise_codec_decryption_micros";
    pub const NOISE_CODEC_ENCRYPTION_SIZE: &str = "snarkos_network_noise_codec_encryption_size";
    pub const NOISE_CODEC_DECRYPTION_SIZE: &str = "snarkos_network_noise_codec_decryption_size";
    pub const TCP_CONNECTIONS: &str = "snarkos_network_tcp_connections_total";
}
