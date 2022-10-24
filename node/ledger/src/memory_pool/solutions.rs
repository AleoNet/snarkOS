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
    pub fn unconfirmed_solutions(&self) -> impl '_ + Iterator<Item = &ProverSolution<N>> {
        self.unconfirmed_solutions.values()
    }

    /// Returns a candidate set of unconfirmed solutions for inclusion in a block.
    pub fn candidate_solutions(&self) -> Vec<ProverSolution<N>> {
        // Add the solutions from the memory pool that do not have collisions.
        let mut solutions = Vec::new();
        let mut commitments = Vec::new();

        for (commitment, solution) in self.unconfirmed_solutions.iter().take(N::MAX_PROVER_SOLUTIONS) {
            // Ensure the commitments are unique.
            if commitments.contains(commitment) {
                continue;
            }

            solutions.push(solution.clone());
            commitments.push(*commitment);
        }

        solutions
    }

    /// Adds the given unconfirmed solution to the memory pool.
    pub fn add_unconfirmed_solution(&mut self, solution: &ProverSolution<N>) -> bool {
        // Ensure the solution does not already exist in the memory pool.
        match !self.contains_unconfirmed_solution(solution.commitment()) {
            true => {
                self.unconfirmed_solutions.insert(solution.commitment(), solution.clone());
                true
            }
            false => {
                trace!("Solution '{}' already exists in memory pool", solution.commitment().0);
                false
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
