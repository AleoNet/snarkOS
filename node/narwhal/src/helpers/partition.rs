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

use crate::helpers::TransmissionID;
use snarkvm::prelude::{Network, ToBytes};

use anyhow::Result;
use sha2::{Digest, Sha256};

fn double_sha256(data: &[u8]) -> [u8; 32] {
    let digest = Sha256::digest(Sha256::digest(data));
    let mut ret = [0u8; 32];
    ret.copy_from_slice(&digest);
    ret
}

pub fn sha256d_to_u128(data: &[u8]) -> u128 {
    let hash_slice = double_sha256(data);
    let mut hash = [0u8; 16];
    hash[..].copy_from_slice(&hash_slice[..16]);
    u128::from_le_bytes(hash)
}

/// Returns the worker ID for the given transmission ID.
pub fn assign_to_worker<N: Network>(transmission_id: impl Into<TransmissionID<N>>, num_workers: u8) -> Result<u8> {
    // Hash the transmission ID to a u128.
    let hash = sha256d_to_u128(&transmission_id.into().to_bytes_le()?);
    // Convert the hash to a worker ID.
    let worker_id = (hash % num_workers as u128) as u8;
    // Return the worker ID.
    Ok(worker_id)
}
