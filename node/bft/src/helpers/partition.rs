// Copyright 2024 Aleo Network Foundation
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

use crate::Worker;
use snarkvm::{
    ledger::narwhal::{Transmission, TransmissionID},
    prelude::{Network, ToBytes},
};

use anyhow::{bail, Result};
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
    // If there is only one worker, return it.
    if num_workers == 1 {
        return Ok(0);
    }
    // Hash the transmission ID to a u128.
    let hash = sha256d_to_u128(&transmission_id.into().to_bytes_le()?);
    // Convert the hash to a worker ID.
    let worker_id = (hash % num_workers as u128) as u8;
    // Return the worker ID.
    Ok(worker_id)
}

/// Assigns the given `(transmission ID, transmission)` entries into the `workers` using the given `op`.
pub fn assign_to_workers<N: Network>(
    workers: &[Worker<N>],
    transmissions: impl Iterator<Item = (TransmissionID<N>, Transmission<N>)>,
    op: impl Fn(&Worker<N>, TransmissionID<N>, Transmission<N>),
) -> Result<()> {
    // Set the number of workers.
    let num_workers = u8::try_from(workers.len()).expect("Too many workers");
    // Re-insert the transmissions into the workers.
    for (transmission_id, transmission) in transmissions.into_iter() {
        // Determine the worker ID.
        let Ok(worker_id) = assign_to_worker(transmission_id, num_workers) else {
            bail!("Unable to assign transmission ID '{transmission_id}' to a worker")
        };
        // Retrieve the worker.
        match workers.get(worker_id as usize) {
            // Use the provided closure to operate on the worker.
            Some(worker) => op(worker, transmission_id, transmission),
            None => bail!("Unable to find worker {worker_id}"),
        };
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use snarkvm::prelude::puzzle::SolutionID;

    type CurrentNetwork = snarkvm::prelude::MainnetV0;

    #[test]
    fn test_assign_to_worker() {
        let data = "Hello Aleo".as_bytes();
        let sha = double_sha256(data);
        assert_eq!(sha, [
            113, 157, 210, 34, 60, 51, 220, 8, 63, 213, 79, 8, 117, 190, 134, 206, 127, 197, 21, 180, 116, 49, 218,
            150, 49, 116, 116, 38, 244, 135, 215, 14
        ]);
        let hash = sha256d_to_u128(data);
        assert_eq!(hash, 274520597840828436951879875061540363633u128);
        let transmission_id: TransmissionID<CurrentNetwork> =
            TransmissionID::Solution(SolutionID::from(123456789), 12345);
        let worker_id = assign_to_worker(transmission_id, 5).unwrap();
        assert_eq!(worker_id, 4);
    }
}
