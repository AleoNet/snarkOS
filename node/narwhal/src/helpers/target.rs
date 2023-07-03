// Copyright (C) 2019-2023 Aleo Systems Inc.
// This file is part of the snarkOS library.

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at:
// http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use snarkvm::prelude::{coinbase::PuzzleCommitment, Network, ToBytes};

use anyhow::Result;
use sha2::{Digest, Sha256};

fn double_sha256(data: &[u8]) -> [u8; 32] {
    let digest = Sha256::digest(Sha256::digest(data));
    let mut ret = [0u8; 32];
    ret.copy_from_slice(&digest);
    ret
}

pub fn sha256d_to_u64(data: &[u8]) -> u64 {
    let hash_slice = double_sha256(data);
    let mut hash = [0u8; 8];
    hash[..].copy_from_slice(&hash_slice[..8]);
    u64::from_le_bytes(hash)
}

/// TODO (howardwu): In snarkVM, move the `to_target` method from PartialSolution to PuzzleCommitment.
/// Returns the target of the solution.
pub fn to_target<N: Network>(commitment: &PuzzleCommitment<N>) -> Result<u64> {
    let hash_to_u64 = sha256d_to_u64(&commitment.to_bytes_le()?);
    if hash_to_u64 == 0 { Ok(u64::MAX) } else { Ok(u64::MAX / hash_to_u64) }
}
