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

use crate::objects::Transaction;
use snarkos_utilities::bytes::{FromBytes, ToBytes};

pub trait BlockScheme: Clone + Eq + FromBytes + ToBytes {
    type BlockHeader: Clone + Eq + FromBytes + ToBytes;
    type Transaction: Transaction;

    /// Returns the header.
    fn header(&self) -> &Self::BlockHeader;

    /// Returns the transactions.
    fn transactions(&self) -> &[Self::Transaction];
}
