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
#![allow(clippy::module_inception)]
#![allow(clippy::suspicious_else_formatting)]
#![allow(clippy::type_complexity)]

#[macro_use]
extern crate thiserror;
#[macro_use]
extern crate tracing;

pub(crate) mod display;
pub(crate) use display::*;

pub mod node;
pub use node::*;

pub mod server;
pub use server::*;

pub mod updater;
pub use updater::*;

pub use snarkos_environment::*;
pub use snarkos_storage::*;
pub use snarkos_utilities::*;

#[cfg(feature = "rpc")]
pub use snarkos_rpc::*;

pub use snarkvm::dpc::{testnet2::Testnet2, Address, Network};
