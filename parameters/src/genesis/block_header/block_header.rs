// Copyright (C) 2019-2020 Aleo Systems Inc.
// This file is part of the snarkVM library.

// The snarkVM library is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// The snarkVM library is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with the snarkVM library. If not, see <https://www.gnu.org/licenses/>.

use snarkvm_models::genesis::Genesis;

pub struct GenesisBlockHeader;

impl Genesis for GenesisBlockHeader {
    const CHECKSUM: &'static str = "";
    const SIZE: u64 = 1088;

    fn load_bytes() -> Vec<u8> {
        let buffer = include_bytes!("block_header.genesis");
        buffer.to_vec()
    }
}
