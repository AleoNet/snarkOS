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

use super::*;

impl<N: Network> MemoryPool<N> {
    /// Returns `true` if the given unconfirmed solution exists in the memory pool.
    pub fn contains_unconfirmed_solution(&self, puzzle_commitment: PuzzleCommitment<N>) -> bool {
        self.unconfirmed_solutions.contains_key(&puzzle_commitment)
    }

    /// Returns the number of unconfirmed solutions in the memory pool.
    pub fn num_unconfirmed_solutions(&self) -> usize {
        self.unconfirmed_solutions.len()
    }

    /// Returns the unconfirmed solutions in the memory pool.
    pub fn unconfirmed_solutions(&self) -> impl '_ + Iterator<Item = &(ProverSolution<N>, u64)> {
        self.unconfirmed_solutions.values()
    }

    /// Returns a candidate set of unconfirmed solutions for inclusion in a block.
    pub fn candidate_solutions(
        &self,
        latest_height: u32,
        latest_proof_target: u64,
        latest_coinbase_target: u64,
    ) -> Result<Option<Vec<ProverSolution<N>>>> {
        // If the latest height is greater than or equal to the anchor height at year 10, then return 'None'.
        if latest_height >= anchor_block_height(N::ANCHOR_TIME, 10) {
            return Ok(None);
        }

        // Add the solutions from the memory pool that do not have collisions.
        let mut solutions = Vec::new();
        let mut commitments = Vec::new();
        let mut cumulative_prover_target: u128 = 0u128;

        for (commitment, (solution, proof_target)) in
            self.unconfirmed_solutions.iter().sorted_by(|a, b| b.1.1.cmp(&a.1.1))
        {
            // Ensure the commitments are unique.
            if commitments.contains(commitment) {
                continue;
            }
            // Ensure the proof target is sufficient.
            if *proof_target < latest_proof_target {
                continue;
            }

            // Append the solution.
            solutions.push(solution.clone());
            // Append the commitment.
            commitments.push(*commitment);
            // Compute the cumulative prover target of the prover solutions as a u128.
            cumulative_prover_target = cumulative_prover_target
                .checked_add(*proof_target as u128)
                .ok_or_else(|| anyhow!("Cumulative target overflowed"))?;

            // If the maximum number of solutions has been reached, then break.
            if solutions.len() >= N::MAX_PROVER_SOLUTIONS {
                break;
            }
        }

        // Return the prover solutions if the cumulative target is greater than or equal to the coinbase target.
        match cumulative_prover_target >= latest_coinbase_target as u128 {
            true => Ok(Some(solutions)),
            false => Ok(None),
        }
    }

    /// Adds the given unconfirmed solution to the memory pool.
    pub fn add_unconfirmed_solution(&mut self, solution: &ProverSolution<N>) -> Result<bool> {
        // Ensure the solution does not already exist in the memory pool.
        match !self.contains_unconfirmed_solution(solution.commitment()) {
            true => {
                // Compute the proof target.
                let proof_target = solution.to_target()?;
                self.unconfirmed_solutions.insert(solution.commitment(), (solution.clone(), proof_target));
                trace!(
                    "✉️  Added a prover solution with target '{proof_target}' ('{}') to the memory pool",
                    solution.commitment().0
                );
                Ok(true)
            }
            false => {
                trace!("Prover solution '{}' already exists in memory pool", solution.commitment().0);
                Ok(false)
            }
        }
    }

    /// Clears an unconfirmed solution from the memory pool.
    pub fn remove_unconfirmed_solution(&mut self, puzzle_commitment: &PuzzleCommitment<N>) {
        self.unconfirmed_solutions.remove(puzzle_commitment);
    }

    /// Clears a list of unconfirmed solutions from the memory pool.
    pub fn remove_unconfirmed_solutions(&mut self, puzzle_commitments: &[PuzzleCommitment<N>]) {
        // This code section executes atomically.

        let mut memory_pool = self.clone();

        for puzzle_commitment in puzzle_commitments {
            memory_pool.unconfirmed_solutions.remove(puzzle_commitment);
        }

        *self = memory_pool;
    }

    /// Clears all unconfirmed solutions from the memory pool.
    pub fn clear_unconfirmed_solutions(&mut self) {
        self.unconfirmed_solutions.clear();
    }
}
