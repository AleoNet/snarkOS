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

#[macro_use]
extern crate tracing;

use snarkos::{connect_to_leader, handle_listener, send_pings, Ledger};
use snarkvm::prelude::*;

use anyhow::Result;
use dotenv::dotenv;
use std::env;

/// The IP and port of the leader node to connect to.
const LEADER_IP: &str = "159.203.77.113:3000";

pub fn initialize_logger(verbosity: u8) {
    match verbosity {
        0 => env::set_var("RUST_LOG", "info"),
        1 => env::set_var("RUST_LOG", "debug"),
        2 | 3 => env::set_var("RUST_LOG", "trace"),
        _ => env::set_var("RUST_LOG", "info"),
    };

    // Filter out undesirable logs.
    let filter = tracing_subscriber::EnvFilter::from_default_env()
        .add_directive("hyper::client=off".parse().unwrap())
        .add_directive("hyper::proto=off".parse().unwrap())
        .add_directive("jsonrpsee=off".parse().unwrap())
        .add_directive("mio=off".parse().unwrap())
        .add_directive("rusoto_core=off".parse().unwrap())
        .add_directive("tokio_util=off".parse().unwrap())
        .add_directive("want=off".parse().unwrap())
        .add_directive("reqwest=off".parse().unwrap());

    // Initialize tracing.
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(verbosity == 3)
        .try_init();
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize the logger.
    initialize_logger(2);

    // Verify that the requisite environment variables are present.
    dotenv().ok();

    // Initialize the private key.
    let private_key = PrivateKey::<Testnet3>::from_str(&env::var("VALIDATOR_PRIVATE_KEY").expect("VALIDATOR_PRIVATE_KEY must be set"))?;

    // Initialize the ledger.
    let ledger = Ledger::<Testnet3>::load(&private_key).await?;

    // Fetch the command line arguments. The listener port can be specified with the following: `cargo run --release -- <listener_port>`
    // If no listener port is specified, the default is 3000.
    let args = env::args().collect::<Vec<_>>();

    // Establish a TCP Listener.
    let listener_port = match args.get(1) {
        Some(port) => port.parse::<u16>().unwrap(),
        _ => 3000u16,
    };
    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", listener_port)).await?;

    // Handle incoming connections.
    let _handle_listener = handle_listener::<Testnet3>(listener, ledger.clone()).await;

    // Connect to the leader node and listen for new blocks.
    let leader_addr = std::net::SocketAddr::from_str(&LEADER_IP)?;
    let _ = connect_to_leader::<Testnet3>(leader_addr, ledger.clone()).await;

    info!("Running a client node... attempting connection with leader: {}", leader_addr);

    // This will prevent the node from generating blocks and will maintain a connection with the leader.
    // Send pings to all peers every 10 seconds.
    let _pings = send_pings::<Testnet3>(ledger.clone()).await;

    Ok(())
}
