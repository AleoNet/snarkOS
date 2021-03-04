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

use prometheus::{Encoder, IntCounter, IntGauge, Registry};
use warp::{Rejection, Reply};

lazy_static! {
    /// The Prometheus registry that metrics are registered in with.
    pub static ref REGISTRY: Registry = Registry::new();

    /// Counts the number of peers currently connected to the node server.
    pub static ref CONNECTED_PEERS: IntGauge = IntGauge::new("connected_peers", "Connected Peers").expect("connected_peers to be created");

    /// Counts the number of requests sent to the node server.
    pub static ref NODE_REQUESTS: IntCounter = IntCounter::new("node_requests", "Node Requests").expect("node_requests to be created");

    /// Counts the number of requests sent to the RPC server.
    pub static ref RPC_REQUESTS: IntCounter = IntCounter::new("rpc_requests", "RPC Requests").expect("rpc_requests to be created");
}

/// Initialize the metrics by registering them with the `Registry`.
/// Use of `expect` is acceptable as if metrics collection fails, we should not start the node.
pub fn initialize() {
    REGISTRY
        .register(Box::new(CONNECTED_PEERS.clone()))
        .expect("CONNECTED_PEERS to be registered");

    REGISTRY
        .register(Box::new(NODE_REQUESTS.clone()))
        .expect("NODE_REQUESTS to be registered");

    REGISTRY
        .register(Box::new(RPC_REQUESTS.clone()))
        .expect("RPC_REQUESTS to be registered");
}

pub async fn metrics_handler() -> Result<impl Reply, Rejection> {
    let encoder = prometheus::TextEncoder::new();

    // Encode our metrics into a response.
    let mut res = {
        let mut buffer = Vec::new();
        if let Err(e) = encoder.encode(&REGISTRY.gather(), &mut buffer) {
            eprintln!("could not encode custom metrics: {}", e);
        };
        let res = match String::from_utf8(buffer.clone()) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("custom metrics could not be from_utf8'd: {}", e);
                String::default()
            }
        };
        buffer.clear();
        res
    };

    // Encode Prometheus' metrics into a response.
    let res_prom = {
        let mut buffer = Vec::new();
        if let Err(e) = encoder.encode(&prometheus::gather(), &mut buffer) {
            eprintln!("could not encode prometheus metrics: {}", e);
        };
        let res_prom = match String::from_utf8(buffer.clone()) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("prometheus metrics could not be from_utf8'd: {}", e);
                String::default()
            }
        };
        buffer.clear();
        res_prom
    };

    res.push_str(&res_prom);
    Ok(res)
}
