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

#![forbid(unsafe_code)]
#![allow(clippy::type_complexity)]

#[macro_use]
extern crate async_trait;
#[macro_use]
extern crate tracing;

pub use snarkos_node_narwhal_events as events;
pub use snarkos_node_narwhal_ledger_service as ledger_service;

pub mod helpers;

mod bft;
pub use bft::*;

mod gateway;
pub use gateway::*;

mod primary;
pub use primary::*;

mod worker;
pub use worker::*;

pub const CONTEXT: &str = "[MemoryPool]";

/// The maximum number of milliseconds to wait before proposing a batch.
pub const MAX_BATCH_DELAY: u64 = 2500; // ms
/// The maximum number of seconds before a proposed batch is considered expired.
pub const MAX_EXPIRATION_TIME_IN_SECS: i64 = 10; // seconds
/// The maximum number of rounds to store before garbage collecting.
pub const MAX_GC_ROUNDS: u64 = 50; // rounds
/// The maximum number of seconds allowed for the leader to send their certificate.
pub const MAX_LEADER_CERTIFICATE_DELAY: i64 = 2 * MAX_BATCH_DELAY as i64 / 1000; // seconds
/// The maximum number of milliseconds to wait before sending a primary ping.
pub const MAX_PRIMARY_PING_DELAY: u64 = MAX_BATCH_DELAY; // ms
/// The maximum block height difference from peers before prioritizing syncing.
pub const MAX_SYNC_DIFFERENCE: u32 = 10;
/// The maximum number of seconds before the timestamp is considered expired.
pub const MAX_TIMESTAMP_DELTA_IN_SECS: i64 = 10; // seconds
/// The maximum number of transmissions allowed in a batch.
pub const MAX_TRANSMISSIONS_PER_BATCH: usize = 250; // transmissions
/// The maximum number of workers that can be spawned.
pub const MAX_WORKERS: u8 = 2; // workers
/// The port on which the memory pool listens for incoming connections.
pub const MEMORY_POOL_PORT: u16 = 5000; // port
/// The frequency at which each worker broadcasts a ping to every other node.
pub const WORKER_PING_INTERVAL: u64 = 1500; // ms

// TODO (howardwu): Add a mechanism to keep validators connected (add reconnect logic).
