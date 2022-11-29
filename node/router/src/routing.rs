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
use snarkos_node_messages::Message;
use snarkos_node_tcp::{
    protocols::{Disconnect, Handshake},
    P2P,
};
use snarkvm::prelude::Network;

use core::time::Duration;

#[async_trait]
pub trait Routing<N: Network>: P2P + Disconnect + Handshake + Inbound<N> + Outbound<N> + Heartbeat<N> {
    /// Initialize the routing.
    async fn initialize_routing(&self) {
        // Enable the TCP protocols.
        self.enable_handshake().await;
        self.enable_reading().await;
        self.enable_writing().await;
        self.enable_disconnect().await;
        // Initialize the heartbeat.
        self.initialize_heartbeat();
        // Initialize the report.
        self.initialize_report();
    }

    /// Initialize a new instance of the heartbeat.
    fn initialize_heartbeat(&self) {
        let self_clone = self.clone();
        self.router().spawn(async move {
            loop {
                // Process a heartbeat in the router.
                self_clone.heartbeat();
                // Sleep for `HEARTBEAT_IN_SECS` seconds.
                tokio::time::sleep(Duration::from_secs(Self::HEARTBEAT_IN_SECS)).await;
            }
        });
    }

    /// Initialize a new instance of the report.
    fn initialize_report(&self) {
        let self_clone = self.clone();
        self.router().spawn(async move {
            loop {
                // Prepare the report.
                let mut report = std::collections::HashMap::new();
                report.insert("message_version".to_string(), Message::<N>::VERSION.to_string());
                report.insert("node_address".to_string(), self_clone.router().address().to_string());
                report.insert("node_type".to_string(), self_clone.router().node_type().to_string());
                report.insert("is_dev".to_string(), self_clone.router().is_dev().to_string());
                // Transmit the report.
                let url = "https://vm.aleo.org/testnet3/report";
                let _ = reqwest::Client::new().post(url).json(&report).send().await;
                // Sleep for a fixed duration in seconds.
                tokio::time::sleep(Duration::from_secs(600)).await;
            }
        });
    }
}
