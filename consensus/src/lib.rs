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

use std::hash::Hash;

mod bft;
mod block;

mod block_tree;
mod ledger;
pub mod message;
mod safety;
pub mod validator;

pub type N = snarkvm::console::network::Testnet3;
pub type Address = snarkvm::console::account::Address<N>;
pub type Signature = snarkvm::console::account::Signature<N>;

// TODO: what should the value of f be?
pub const F: usize = 11;

// FIXME: pick a hash function
pub fn hash<T: Hash>(object: &T) -> u64 {
    todo!()
}
