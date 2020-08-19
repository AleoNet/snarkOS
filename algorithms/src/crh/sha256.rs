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

use sha2::{Digest, Sha256};

pub fn sha256(data: &[u8]) -> Vec<u8> {
    Sha256::digest(&data).to_vec()
}

pub fn double_sha256(data: &[u8]) -> Vec<u8> {
    Sha256::digest(&Sha256::digest(&data)).to_vec()
}

pub fn sha256d_to_u64(data: &[u8]) -> u64 {
    let hash_slice = double_sha256(data);
    let mut hash = [0u8; 8];
    hash[..].copy_from_slice(&hash_slice[..8]);
    u64::from_le_bytes(hash)
}
