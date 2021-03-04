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

use snarkvm_parameters::::Genesis;

pub struct Transaction1;

impl Genesis for Transaction1 {
    const CHECKSUM: &'static str = "";
    const SIZE: u64 = 1538;

    fn load_bytes() -> Vec<u8> {
        let buffer = include_bytes!("transaction_1.genesis");
        buffer.to_vec()
    }
}
