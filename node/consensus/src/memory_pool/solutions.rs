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

use super::*;

use std::collections::hash_map::Entry;

impl<N: Network> MemoryPool<N> {
    /// Returns `true` if the given unconfirmed solution exists in the memory pool.
    pub fn contains_unconfirmed_solution(&self, puzzle_commitment: PuzzleCommitment<N>) -> bool {
        self.unconfirmed_solutions.read().contains_key(&puzzle_commitment)
    }

    /// Returns the number of unconfirmed solutions in the memory pool.
    pub fn num_unconfirmed_solutions(&self) -> usize {
        self.unconfirmed_solutions.read().len()
    }

    /// Returns the unconfirmed solutions in the memory pool.
    pub fn unconfirmed_solutions(&self) -> Vec<(ProverSolution<N>, u64)> {
        self.unconfirmed_solutions.read().values().cloned().collect()
    }

    /// Returns the candidate coinbase target of the valid unconfirmed solutions in the memory pool.
    pub fn candidate_coinbase_target(&self, latest_proof_target: u64) -> Result<u128> {
        // Filter the solutions by the latest proof target, ensure they are unique, and rank in descending order of proof target.
        let mut candidate_proof_targets = self
            .unconfirmed_solutions
            .read()
            .iter()
            .filter(|(_, (_, proof_target))| *proof_target >= latest_proof_target)
            .unique_by(|(k, _)| *k)
            .map(|(_, v)| v.1)
            .sorted_by(|a, b| b.cmp(a))
            .take(256);

        // Compute the cumulative proof target of the prover solutions as a u128.
        candidate_proof_targets.try_fold(0u128, |cumulative, proof_target| {
            cumulative.checked_add(proof_target as u128).ok_or_else(|| anyhow!("Candidate coinbase target overflowed"))
        })
    }

    /// Returns a candidate set of unconfirmed solutions for inclusion in a block.
    pub fn candidate_solutions<C: ConsensusStorage<N>>(
        &self,
        consensus: &Consensus<N, C>,
        latest_height: u32,
        latest_proof_target: u64,
        latest_coinbase_target: u64,
    ) -> Result<Option<Vec<ProverSolution<N>>>> {
        // If the latest height is greater than or equal to the anchor height at year 10, then return 'None'.
        if latest_height >= anchor_block_height(N::ANCHOR_TIME, 10) {
            return Ok(None);
        }

        // Filter the solutions by the latest proof target, ensure they are unique, and rank in descending order of proof target.
        let candidate_solutions: Vec<_> = self
            .unconfirmed_solutions
            .read()
            .iter()
            .filter(|(_, (_, proof_target))| *proof_target >= latest_proof_target)
            .filter(|(_, (solution, _))| {
                // Ensure the prover solution is not already in the ledger.
                match consensus.ledger.contains_puzzle_commitment(&solution.commitment()) {
                    Ok(true) => false,
                    Ok(false) => true,
                    Err(error) => {
                        error!("Failed to check if prover solution {error} is in the ledger");
                        false
                    }
                }
            })
            .sorted_by(|a, b| b.1.1.cmp(&a.1.1))
            .map(|(_, v)| v.0)
            .unique_by(|s| s.commitment())
            .take(256)
            .collect();

        // Compute the cumulative proof target of the prover solutions as a u128.
        let cumulative_proof_target: u128 = candidate_solutions.iter().try_fold(0u128, |cumulative, solution| {
            cumulative
                .checked_add(solution.to_target()? as u128)
                .ok_or_else(|| anyhow!("Cumulative proof target overflowed"))
        })?;

        // Return the prover solutions if the cumulative target is greater than or equal to the coinbase target.
        match cumulative_proof_target >= latest_coinbase_target as u128 {
            true => Ok(Some(candidate_solutions)),
            false => Ok(None),
        }
    }

    /// Adds the given unconfirmed solution to the memory pool.
    pub fn add_unconfirmed_solution(&self, solution: &ProverSolution<N>) -> Result<bool> {
        // Acquire the write lock on the unconfirmed solutions.
        let mut unconfirmed_solutions = self.unconfirmed_solutions.write();

        // Ensure the solution does not already exist in the memory pool.
        match unconfirmed_solutions.entry(solution.commitment()) {
            Entry::Vacant(entry) => {
                // Compute the proof target.
                let proof_target = solution.to_target()?;
                // Add the solution to the memory pool.
                entry.insert((*solution, proof_target));
                debug!("✉️  Added a prover solution with target '{proof_target}' to the memory pool");
                Ok(true)
            }
            Entry::Occupied(_) => {
                trace!("Prover solution '{}' already exists in memory pool", solution.commitment());
                Ok(false)
            }
        }
    }

    /// Clears the memory pool of unconfirmed transactions that are now invalid.
    pub fn clear_invalid_solutions<C: ConsensusStorage<N>>(&self, consensus: &Consensus<N, C>) {
        self.unconfirmed_solutions.write().retain(|puzzle_commitment, _solution| {
            // Ensure the prover solution is still valid.
            match consensus.ledger.contains_puzzle_commitment(puzzle_commitment) {
                Ok(true) | Err(_) => {
                    trace!("Removed prover solution '{puzzle_commitment}' from the memory pool");
                    false
                }
                Ok(false) => true,
            }
        });
    }

    /// Clears all unconfirmed solutions from the memory pool.
    pub fn clear_all_unconfirmed_solutions(&self) {
        self.unconfirmed_solutions.write().clear();
    }
}
