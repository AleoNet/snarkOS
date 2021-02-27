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

use crate::{block_header::GenesisBlockHeader, transaction_1::Transaction1};
use snarkvm_models::genesis::Genesis;
use snarkvm_utilities::variable_length_integer::variable_length_integer;

pub struct GenesisBlock;

impl Genesis for GenesisBlock {
    const CHECKSUM: &'static str = "";
    const SIZE: u64 = 2627;

    fn load_bytes() -> Vec<u8> {
        let mut buffer = vec![];

        let block_header_bytes = GenesisBlockHeader::load_bytes();

        let num_transactions: u64 = 1;
        let transaction_1_bytes = Transaction1::load_bytes();

        buffer.extend(block_header_bytes);
        buffer.extend(variable_length_integer(num_transactions));
        buffer.extend(transaction_1_bytes);

        buffer
    }
}
