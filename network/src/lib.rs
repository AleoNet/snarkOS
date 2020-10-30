// Copyright (C) 2019-2020 Aleo Systems Inc.
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
extern crate tracing;
extern crate log;
#[macro_use]
extern crate snarkos_metrics;

pub mod external;
pub mod peers;
pub mod request;

pub mod peer_manager;
pub use peer_manager::*;

pub mod environment;
pub use environment::*;

pub mod send_handler;
pub use send_handler::*;

pub mod receive_handler;
pub use receive_handler::*;

pub mod server;
pub use server::*;

pub mod sync_manager;
pub use sync_manager::*;
