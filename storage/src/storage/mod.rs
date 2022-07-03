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

#[cfg(feature = "rocks")]
pub mod rocksdb;
#[cfg(feature = "rocks")]
pub type DataMap<K, V, A> = crate::storage::rocksdb::DataMap<K, V, A>;

pub mod traits;
pub use traits::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u16)]
pub enum DataID {
    BlockHeaders = 0,
    BlockHeights,
    BlockTransactions,
    Commitments,
    LedgerRoots,
    Records,
    SerialNumbers,
    Transactions,
    Transitions,
    Shares,
    #[cfg(test)]
    Test,
}

#[cfg(feature = "test")]
impl From<u16> for DataID {
    fn from(id: u16) -> Self {
        match id {
            0 => Self::BlockHeaders,
            1 => Self::BlockHeights,
            2 => Self::BlockTransactions,
            3 => Self::Commitments,
            4 => Self::LedgerRoots,
            5 => Self::Records,
            6 => Self::SerialNumbers,
            7 => Self::Transactions,
            8 => Self::Transitions,
            9 => Self::Shares,
            x => panic!("Unexpected map id: {}", x),
        }
    }
}
