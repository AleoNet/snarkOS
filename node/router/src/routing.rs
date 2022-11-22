// Copyright (C) 2019-2022 Aleo Systems Inc.
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

use crate::{Heartbeat, Inbound, Outbound};
use snarkos_node_tcp::{
    protocols::{Disconnect, Handshake},
    P2P,
};
use snarkvm::prelude::Network;

use core::time::Duration;

#[async_trait]
pub trait Routing<N: Network>: P2P + Disconnect + Handshake + Inbound<N> + Outbound<N> + Heartbeat<N> {
    /// The frequency at which the node sends a puzzle request.
    const PUZZLE_REQUEST_IN_SECS: u64 = N::ANCHOR_TIME as u64;

    /// Initialize the routing.
    async fn initialize_routing(&self) {
        // Enable the TCP protocols.
        self.enable_handshake().await;
        self.enable_reading().await;
        self.enable_writing().await;
        self.enable_disconnect().await;
        // Initialize the heartbeat.
        self.initialize_heartbeat();
        // Initialize the puzzle request.
        self.initialize_puzzle_request();
        // Initialize the report.
        self.initialize_report();
    }

    /// Initialize a new instance of the heartbeat.
    fn initialize_heartbeat(&self) {
        let self_clone = self.clone();
        self.router().spawn(async move {
            loop {
                // Process a heartbeat in the router.
                self_clone.heartbeat().await;
                // Sleep for `HEARTBEAT_IN_SECS` seconds.
                tokio::time::sleep(Duration::from_secs(Self::HEARTBEAT_IN_SECS)).await;
            }
        });
    }

    /// TODO (howardwu): Change this for Phase 3.
    /// Initialize a new instance of the puzzle request.
    fn initialize_puzzle_request(&self) {
        if self.router().node_type().is_prover() && Self::PUZZLE_REQUEST_IN_SECS > 0 {
            let self_clone = self.clone();
            self.router().spawn(async move {
                loop {
                    // Handle the bootstrap peers.
                    self_clone.handle_bootstrap_peers().await;
                    // Sleep for brief period.
                    tokio::time::sleep(Duration::from_millis(2500)).await;
                }
            });
        }
        if !self.router().node_type().is_beacon() && Self::PUZZLE_REQUEST_IN_SECS > 0 {
            let self_clone = self.clone();
            self.router().spawn(async move {
                loop {
                    // Send a "PuzzleRequest".
                    self_clone.send_puzzle_request();
                    // Sleep for `PUZZLE_REQUEST_IN_SECS` seconds.
                    tokio::time::sleep(Duration::from_secs(Self::PUZZLE_REQUEST_IN_SECS)).await;
                }
            });
        }
    }

    /// Initialize a new instance of the report.
    fn initialize_report(&self) {
        let self_clone = self.clone();
        self.router().spawn(async move {
            let url = "https://vm.aleo.org/testnet3/report";
            loop {
                // Prepare the report.
                let mut report = std::collections::HashMap::new();
                report.insert("node_address".to_string(), self_clone.router().address().to_string());
                report.insert("node_type".to_string(), self_clone.router().node_type().to_string());
                report.insert("is_dev".to_string(), self_clone.router().is_dev().to_string());
                // Transmit the report.
                if reqwest::Client::new().post(url).json(&report).send().await.is_err() {
                    warn!("Failed to send report");
                }
                // Sleep for a fixed duration in seconds.
                tokio::time::sleep(Duration::from_secs(600)).await;
            }
        });
    }
}
