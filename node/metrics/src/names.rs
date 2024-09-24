// Copyright 2024 Aleo Network Foundation
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

pub(super) const COUNTER_NAMES: [&str; 2] = [bft::LEADERS_ELECTED, consensus::STALE_UNCONFIRMED_TRANSMISSIONS];

pub(super) const GAUGE_NAMES: [&str; 26] = [
    bft::CONNECTED,
    bft::CONNECTING,
    bft::LAST_STORED_ROUND,
    bft::PROPOSAL_ROUND,
    bft::CERTIFIED_BATCHES,
    bft::HEIGHT,
    bft::LAST_COMMITTED_ROUND,
    bft::IS_SYNCED,
    blocks::SOLUTIONS,
    blocks::TRANSACTIONS,
    blocks::ACCEPTED_DEPLOY,
    blocks::ACCEPTED_EXECUTE,
    blocks::REJECTED_DEPLOY,
    blocks::REJECTED_EXECUTE,
    blocks::ABORTED_TRANSACTIONS,
    blocks::ABORTED_SOLUTIONS,
    blocks::PROOF_TARGET,
    blocks::COINBASE_TARGET,
    blocks::CUMULATIVE_PROOF_TARGET,
    consensus::COMMITTED_CERTIFICATES,
    consensus::UNCONFIRMED_SOLUTIONS,
    consensus::UNCONFIRMED_TRANSACTIONS,
    router::CONNECTED,
    router::CANDIDATE,
    router::RESTRICTED,
    tcp::TCP_TASKS,
];

pub(super) const HISTOGRAM_NAMES: [&str; 3] =
    [bft::COMMIT_ROUNDS_LATENCY, consensus::CERTIFICATE_COMMIT_LATENCY, consensus::BLOCK_LATENCY];

pub mod bft {
    pub const COMMIT_ROUNDS_LATENCY: &str = "snarkos_bft_commit_rounds_latency_secs"; // <-- This one doesn't even make sense.
    pub const CONNECTED: &str = "snarkos_bft_connected_total";
    pub const CONNECTING: &str = "snarkos_bft_connecting_total";
    pub const LAST_STORED_ROUND: &str = "snarkos_bft_last_stored_round";
    pub const LEADERS_ELECTED: &str = "snarkos_bft_leaders_elected_total";
    pub const PROPOSAL_ROUND: &str = "snarkos_bft_primary_proposal_round";
    pub const CERTIFIED_BATCHES: &str = "snarkos_bft_primary_certified_batches";
    pub const HEIGHT: &str = "snarkos_bft_height_total";
    pub const LAST_COMMITTED_ROUND: &str = "snarkos_bft_last_committed_round";
    pub const IS_SYNCED: &str = "snarkos_bft_is_synced";
}

pub mod blocks {
    pub const TRANSACTIONS: &str = "snarkos_blocks_transactions_total";
    pub const SOLUTIONS: &str = "snarkos_blocks_solutions_total";
    pub const ACCEPTED_DEPLOY: &str = "snarkos_blocks_accepted_deploy";
    pub const ACCEPTED_EXECUTE: &str = "snarkos_blocks_accepted_execute";
    pub const REJECTED_DEPLOY: &str = "snarkos_blocks_rejected_deploy";
    pub const REJECTED_EXECUTE: &str = "snarkos_blocks_rejected_execute";
    pub const ABORTED_TRANSACTIONS: &str = "snarkos_blocks_aborted_transactions";
    pub const ABORTED_SOLUTIONS: &str = "snarkos_blocks_aborted_solutions";
    pub const PROOF_TARGET: &str = "snarkos_blocks_proof_target";
    pub const COINBASE_TARGET: &str = "snarkos_blocks_coinbase_target";
    pub const CUMULATIVE_PROOF_TARGET: &str = "snarkos_blocks_cumulative_proof_target";
}

pub mod consensus {
    pub const CERTIFICATE_COMMIT_LATENCY: &str = "snarkos_consensus_certificate_commit_latency_secs";
    pub const COMMITTED_CERTIFICATES: &str = "snarkos_consensus_committed_certificates_total";
    pub const BLOCK_LATENCY: &str = "snarkos_consensus_block_latency_secs";
    pub const UNCONFIRMED_TRANSACTIONS: &str = "snarkos_consensus_unconfirmed_transactions_total";
    pub const UNCONFIRMED_SOLUTIONS: &str = "snarkos_consensus_unconfirmed_solutions_total";
    pub const TRANSMISSION_LATENCY: &str = "snarkos_consensus_transmission_latency";
    pub const STALE_UNCONFIRMED_TRANSMISSIONS: &str = "snarkos_consensus_stale_unconfirmed_transmissions";
}

pub mod router {
    pub const CONNECTED: &str = "snarkos_router_connected_total";
    pub const CANDIDATE: &str = "snarkos_router_candidate_total";
    pub const RESTRICTED: &str = "snarkos_router_restricted_total";
}

pub mod tcp {
    pub const TCP_TASKS: &str = "snarkos_tcp_tasks_total";
}
