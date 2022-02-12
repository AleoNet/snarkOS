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

pub(crate) mod context;
pub(crate) use context::*;

pub(crate) mod error;
pub(crate) use error::*;

pub(crate) mod resources;
pub(crate) use resources::*;

pub(crate) mod rpc_impl;

pub(crate) mod rpc_trait;
pub(crate) use rpc_trait::*;

#[cfg(test)]
mod tests;
