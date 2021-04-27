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

// Compilation
#![allow(clippy::module_inception)]
#![warn(unused_extern_crates)]
#![forbid(unsafe_code)]
// Documentation
#![cfg_attr(nightly, feature(doc_cfg, external_doc))]
#![cfg_attr(nightly, doc(include = "../documentation/concepts/network_server.md"))]

#[macro_use]
extern crate derivative;
#[macro_use]
extern crate tracing;
#[macro_use]
extern crate snarkos_metrics;

pub mod config;
pub use config::*;

pub mod errors;
pub use errors::*;

pub mod inbound;
pub use inbound::*;

pub mod message;
pub use message::*;

pub mod node;
pub use node::*;

pub mod outbound;
pub use outbound::*;

pub mod peers;
pub use peers::*;

pub mod sync;
pub use sync::*;

pub const HANDSHAKE_PATTERN: &str = "Noise_XXpsk3_25519_ChaChaPoly_SHA256";
pub const HANDSHAKE_PSK: &[u8] = b"b765e427e836e0029a1e2a22ba60c52a"; // the PSK must be 32B
pub const HANDSHAKE_TIME_LIMIT_SECS: u8 = 5;
pub const MAX_MESSAGE_SIZE: usize = 8 * 1024 * 1024; // 8MiB
pub const NOISE_BUF_LEN: usize = 65535;
pub const NOISE_TAG_LEN: usize = 16;
/// The maximum number of block hashes that can be requested or provided in a single batch.
pub const MAX_BLOCK_SYNC_COUNT: u32 = 250;
/// The maximum number of peers shared at once in response to a `GetPeers` message.
pub const SHARED_PEER_COUNT: usize = 25;

pub(crate) type Sender = tokio::sync::mpsc::Sender<Message>;

pub(crate) type Receiver = tokio::sync::mpsc::Receiver<Message>;
