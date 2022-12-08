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

#[allow(dead_code)]
pub mod router;
pub use router::*;

use std::{
    env,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    str::FromStr,
};

use snarkos_account::Account;
use snarkos_node_messages::NodeType;
use snarkos_node_router::Router;
use snarkvm::prelude::{Block, FromBytes, Network, Testnet3 as CurrentNetwork};

/// A helper macro to print the TCP listening address, along with the connected and connecting peers.
#[macro_export]
macro_rules! print_tcp {
    ($node:expr) => {
        println!(
            "{}: Active - {:?}, Pending - {:?}",
            $node.local_ip(),
            $node.tcp().connected_addrs(),
            $node.tcp().connecting_addrs()
        );
    };
}

/// Returns a fixed account.
pub fn sample_account() -> Account<CurrentNetwork> {
    Account::<CurrentNetwork>::from_str("APrivateKey1zkp2oVPTci9kKcUprnbzMwq95Di1MQERpYBhEeqvkrDirK1").unwrap()
}

/// Loads the current network's genesis block.
pub fn sample_genesis_block<N: Network>() -> Block<N> {
    Block::<N>::from_bytes_le(N::genesis_bytes()).unwrap()
}

/// Enables logging in tests.
#[allow(dead_code)]
pub fn initialize_logger(level: u8) {
    match level {
        0 => env::set_var("RUST_LOG", "info"),
        1 => env::set_var("RUST_LOG", "debug"),
        2 | 3 => env::set_var("RUST_LOG", "trace"),
        _ => env::set_var("RUST_LOG", "info"),
    };

    // Filter out undesirable logs.
    let filter = tracing_subscriber::EnvFilter::from_default_env()
        .add_directive("tokio_util=off".parse().unwrap())
        .add_directive("mio=off".parse().unwrap());

    // Initialize tracing.
    let _ = tracing_subscriber::fmt().with_env_filter(filter).with_target(level == 3).try_init();
}

/// Initializes a beacon router. Setting the `listening_port = 0` will result in a random port being assigned.
#[allow(dead_code)]
pub async fn beacon(listening_port: u16, max_peers: u16) -> TestRouter<CurrentNetwork> {
    Router::new(
        SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), listening_port),
        NodeType::Beacon,
        sample_account(),
        &[],
        max_peers,
        true,
    )
    .await
    .expect("couldn't create beacon router")
    .into()
}

/// Initializes a client router. Setting the `listening_port = 0` will result in a random port being assigned.
#[allow(dead_code)]
pub async fn client(listening_port: u16, max_peers: u16) -> TestRouter<CurrentNetwork> {
    Router::new(
        SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), listening_port),
        NodeType::Client,
        sample_account(),
        &[],
        max_peers,
        true,
    )
    .await
    .expect("couldn't create client router")
    .into()
}

/// Initializes a prover router. Setting the `listening_port = 0` will result in a random port being assigned.
#[allow(dead_code)]
pub async fn prover(listening_port: u16, max_peers: u16) -> TestRouter<CurrentNetwork> {
    Router::new(
        SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), listening_port),
        NodeType::Prover,
        sample_account(),
        &[],
        max_peers,
        true,
    )
    .await
    .expect("couldn't create prover router")
    .into()
}

/// Initializes a validator router. Setting the `listening_port = 0` will result in a random port being assigned.
#[allow(dead_code)]
pub async fn validator(listening_port: u16, max_peers: u16) -> TestRouter<CurrentNetwork> {
    Router::new(
        SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), listening_port),
        NodeType::Validator,
        sample_account(),
        &[],
        max_peers,
        true,
    )
    .await
    .expect("couldn't create validator router")
    .into()
}
