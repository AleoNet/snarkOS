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

#![forbid(unsafe_code)]

#[macro_use]
extern crate tracing;

pub mod helpers;

use crate::helpers::{NodeType, RawStatus, Resources};
use snarkvm::prelude::Network;

use core::{fmt::Debug, marker::PhantomData};
use once_cell::sync::OnceCell;
use rayon::{ThreadPool, ThreadPoolBuilder};
use std::{
    collections::HashSet,
    net::SocketAddr,
    sync::{atomic::AtomicBool, Arc},
};

#[rustfmt::skip]
pub trait Environment: 'static + Clone + Debug + Send + Sync {
    type Network: Network;
    /// The specified type of node.
    const NODE_TYPE: NodeType;
    /// The version of the network protocol; it can be incremented in order to force users to update.
    const MESSAGE_VERSION: u32 = 0;
    /// If `true`, a mining node will craft public coinbase transactions.
    const COINBASE_IS_PUBLIC: bool = false;

    /// The port for communicating with the node server.
    const DEFAULT_NODE_PORT: u16 = 4130 + Self::Network::ID;
    /// The port for communicating with the RPC server.
    const DEFAULT_RPC_PORT: u16 = 3030 + Self::Network::ID;

    /// The list of sync nodes to bootstrap the node server with.
    const BEACON_NODES: &'static [&'static str] = &["127.0.0.1:4135"];
    /// The list of nodes to attempt to maintain connections with.
    const TRUSTED_NODES: &'static [&'static str] = &[];

    /// The duration in seconds to sleep in between heartbeat executions.
    const HEARTBEAT_IN_SECS: u64 = 9;
    /// The maximum duration in seconds permitted for establishing a connection with a node,
    /// before dropping the connection; it should be no greater than the `HEARTBEAT_IN_SECS`.
    const CONNECTION_TIMEOUT_IN_MILLIS: u64 = 500;
    /// The duration in seconds to sleep in between ping requests with a connected peer.
    const PING_SLEEP_IN_SECS: u64 = 60;
    /// The duration in seconds after which a connected peer is considered inactive or
    /// disconnected if no message has been received in the meantime.
    const RADIO_SILENCE_IN_SECS: u64 = 210; // 3.5 minutes
    /// The duration in seconds after which to expire a failure from a peer.
    const FAILURE_EXPIRY_TIME_IN_SECS: u64 = 7200; // 2 hours

    /// The minimum number of peers required to maintain connections with.
    const MINIMUM_NUMBER_OF_PEERS: usize;
    /// The maximum number of peers permitted to maintain connections with.
    const MAXIMUM_NUMBER_OF_PEERS: usize;
    /// The maximum number of connection failures permitted by an inbound connecting peer.
    const MAXIMUM_CONNECTION_FAILURES: u32 = 3;
    /// The maximum number of candidate peers permitted to be stored in the node.
    const MAXIMUM_CANDIDATE_PEERS: usize = 10_000;

    /// The maximum number of blocks that may be fetched in one request.
    const MAXIMUM_BLOCK_REQUEST: u32 = 250;
    /// The maximum number of failures tolerated before disconnecting from a peer.
    const MAXIMUM_NUMBER_OF_FAILURES: usize = 1024;

    /// Returns the list of sync nodes to bootstrap the node server with.
    fn beacon_nodes() -> &'static HashSet<SocketAddr> {
        static NODES: OnceCell<HashSet<SocketAddr>> = OnceCell::new();
        NODES.get_or_init(|| Self::BEACON_NODES.iter().map(|ip| ip.parse().unwrap()).collect())
    }

    /// Returns the list of trusted nodes.
    fn trusted_nodes() -> &'static HashSet<SocketAddr> {
        static NODES: OnceCell<HashSet<SocketAddr>> = OnceCell::new();
        NODES.get_or_init(|| Self::TRUSTED_NODES.iter().map(|ip| ip.parse().unwrap()).collect())
    }

    /// Returns the resource handler for the node.
    fn resources() -> &'static Resources {
        static RESOURCES: OnceCell<Resources> = OnceCell::new();
        RESOURCES.get_or_init(Resources::default)
    }

    /// Returns the status of the node.
    fn status() -> &'static RawStatus {
        static STATUS: OnceCell<RawStatus> = OnceCell::new();
        STATUS.get_or_init(RawStatus::default)
    }

    /// Returns the terminator bit for the prover.
    fn terminator() -> &'static Arc<AtomicBool> {
        static TERMINATOR: OnceCell<Arc<AtomicBool>> = OnceCell::new();
        TERMINATOR.get_or_init(|| Arc::new(AtomicBool::new(false)))
    }

    /// Returns a thread pool for the node to perform intensive operations.
    fn thread_pool() -> &'static Arc<ThreadPool> {
        static POOL: OnceCell<Arc<ThreadPool>> = OnceCell::new();
        POOL.get_or_init(|| {
            Arc::new(ThreadPoolBuilder::new()
                .stack_size(8 * 1024 * 1024)
                .num_threads((num_cpus::get() * 7 / 8).max(2))
                .build()
                .expect("Failed to initialize a thread pool for the node"))
        })
    }
}

#[derive(Clone, Debug, Default)]
pub struct Client<N: Network>(PhantomData<N>);

#[rustfmt::skip]
impl<N: Network> Environment for Client<N> {
    type Network = N;
    const NODE_TYPE: NodeType = NodeType::Client;
    const MINIMUM_NUMBER_OF_PEERS: usize = 2;
    const MAXIMUM_NUMBER_OF_PEERS: usize = 21;
}

#[derive(Clone, Debug, Default)]
pub struct Validator<N: Network>(PhantomData<N>);

#[rustfmt::skip]
impl<N: Network> Environment for Validator<N> {
    type Network = N;
    const NODE_TYPE: NodeType = NodeType::Validator;
    const COINBASE_IS_PUBLIC: bool = true;
    const MINIMUM_NUMBER_OF_PEERS: usize = 21;
    const MAXIMUM_NUMBER_OF_PEERS: usize = 100;
}

#[derive(Clone, Debug, Default)]
pub struct Prover<N: Network>(PhantomData<N>);

#[rustfmt::skip]
impl<N: Network> Environment for Prover<N> {
    type Network = N;
    const NODE_TYPE: NodeType = NodeType::Prover;
    const COINBASE_IS_PUBLIC: bool = true;
    const MINIMUM_NUMBER_OF_PEERS: usize = 2;
    const MAXIMUM_NUMBER_OF_PEERS: usize = 21;
}

#[derive(Clone, Debug, Default)]
pub struct Beacon<N: Network>(PhantomData<N>);

#[rustfmt::skip]
impl<N: Network> Environment for Beacon<N> {
    type Network = N;
    const NODE_TYPE: NodeType = NodeType::Beacon;
    const MINIMUM_NUMBER_OF_PEERS: usize = 35;
    const MAXIMUM_NUMBER_OF_PEERS: usize = 1024;
    const HEARTBEAT_IN_SECS: u64 = 5;
}

#[derive(Clone, Debug, Default)]
pub struct ClientTrial<N: Network>(PhantomData<N>);

#[rustfmt::skip]
impl<N: Network> Environment for ClientTrial<N> {
    type Network = N;
    const NODE_TYPE: NodeType = NodeType::Client;
    const BEACON_NODES: &'static [&'static str] = &[
        "144.126.219.193:4132", "165.232.145.194:4132", "143.198.164.241:4132", "188.166.7.13:4132", "167.99.40.226:4132",
        "159.223.124.150:4132", "137.184.192.155:4132", "147.182.213.228:4132", "137.184.202.162:4132", "159.223.118.35:4132",
        "161.35.106.91:4132", "157.245.133.62:4132", "143.198.166.150:4132",
    ];
    const MINIMUM_NUMBER_OF_PEERS: usize = 11;
    const MAXIMUM_NUMBER_OF_PEERS: usize = 31;
}

#[derive(Clone, Debug, Default)]
pub struct ValidatorTrial<N: Network>(PhantomData<N>);

#[rustfmt::skip]
impl<N: Network> Environment for ValidatorTrial<N> {
    type Network = N;
    const NODE_TYPE: NodeType = NodeType::Validator;
    const BEACON_NODES: &'static [&'static str] = &[
        "144.126.219.193:4132", "165.232.145.194:4132", "143.198.164.241:4132", "188.166.7.13:4132", "167.99.40.226:4132",
        "159.223.124.150:4132", "137.184.192.155:4132", "147.182.213.228:4132", "137.184.202.162:4132", "159.223.118.35:4132",
        "161.35.106.91:4132", "157.245.133.62:4132", "143.198.166.150:4132",
    ];
    const MINIMUM_NUMBER_OF_PEERS: usize = 11;
    const MAXIMUM_NUMBER_OF_PEERS: usize = 1000;
    const COINBASE_IS_PUBLIC: bool = true;
}

#[derive(Clone, Debug, Default)]
pub struct ProverTrial<N: Network>(PhantomData<N>);

#[rustfmt::skip]
impl<N: Network> Environment for ProverTrial<N> {
    type Network = N;
    const NODE_TYPE: NodeType = NodeType::Prover;
    const BEACON_NODES: &'static [&'static str] = &[
        "144.126.219.193:4132", "165.232.145.194:4132", "143.198.164.241:4132", "188.166.7.13:4132", "167.99.40.226:4132",
        "159.223.124.150:4132", "137.184.192.155:4132", "147.182.213.228:4132", "137.184.202.162:4132", "159.223.118.35:4132",
        "161.35.106.91:4132", "157.245.133.62:4132", "143.198.166.150:4132",
    ];
    const MINIMUM_NUMBER_OF_PEERS: usize = 11;
    const MAXIMUM_NUMBER_OF_PEERS: usize = 21;
    const COINBASE_IS_PUBLIC: bool = true;
}

#[derive(Clone, Debug, Default)]
pub struct TestEnvironment<N: Network>(PhantomData<N>);

#[rustfmt::skip]
impl<N: Network> Environment for TestEnvironment<N> {
    type Network = N;
    const NODE_TYPE: NodeType = NodeType::Prover;
    const BEACON_NODES: &'static [&'static str] = &[];
    const MINIMUM_NUMBER_OF_PEERS: usize = 1;
    const MAXIMUM_NUMBER_OF_PEERS: usize = 5;
    const COINBASE_IS_PUBLIC: bool = true;
}
