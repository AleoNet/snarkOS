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
extern crate tracing;

pub mod helpers;

pub mod ledger;
pub use ledger::*;

pub mod operator;
pub use operator::*;

pub(crate) mod peer;
pub(crate) use peer::*;

pub mod peers;
pub use peers::*;

pub mod prover;
pub use prover::*;

pub mod state;
pub use state::*;
