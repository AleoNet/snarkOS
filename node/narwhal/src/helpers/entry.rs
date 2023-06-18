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

use snarkos_node_messages::Data;
use snarkvm::{
    console::prelude::*,
    prelude::{ProverSolution, PuzzleCommitment, Transaction},
};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum EntryID<N: Network> {
    /// A prover solution.
    Solution(PuzzleCommitment<N>),
    /// A transaction.
    Transaction(N::TransactionID),
}

impl<N: Network> From<PuzzleCommitment<N>> for EntryID<N> {
    /// Converts the puzzle commitment into an entry ID.
    fn from(puzzle_commitment: PuzzleCommitment<N>) -> Self {
        Self::Solution(puzzle_commitment)
    }
}

impl<N: Network> From<&N::TransactionID> for EntryID<N> {
    /// Converts the transaction ID into an entry ID.
    fn from(transaction_id: &N::TransactionID) -> Self {
        Self::Transaction(*transaction_id)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Entry<N: Network> {
    /// A prover solution.
    Solution(Data<ProverSolution<N>>),
    /// A transaction.
    Transaction(Data<Transaction<N>>),
}

impl<N: Network> From<Data<ProverSolution<N>>> for Entry<N> {
    /// Converts the prover solution into an entry.
    fn from(solution: Data<ProverSolution<N>>) -> Self {
        Self::Solution(solution)
    }
}

impl<N: Network> From<Data<Transaction<N>>> for Entry<N> {
    /// Converts the transaction into an entry.
    fn from(transaction: Data<Transaction<N>>) -> Self {
        Self::Transaction(transaction)
    }
}

impl<N: Network> FromBytes for Entry<N> {
    /// Reads the prover solution from the buffer.
    fn read_le<R: Read>(mut reader: R) -> IoResult<Self> {
        // Read the version.
        let version = u8::read_le(&mut reader)?;
        // Ensure the version is valid.
        if version != 0 {
            return Err(error("Invalid worker entry version"));
        }

        // Read the variant.
        let variant = u8::read_le(&mut reader)?;
        // Match the variant.
        match variant {
            0 => {
                // Read the prover solution.
                let solution = ProverSolution::read_le(&mut reader)?;
                // Return the prover solution.
                Ok(Entry::Solution(Data::Object(solution)))
            }
            1 => {
                // Read the transaction.
                let transaction = Transaction::read_le(&mut reader)?;
                // Return the transaction.
                Ok(Entry::Transaction(Data::Object(transaction)))
            }
            2.. => Err(error("Invalid worker entry variant")),
        }
    }
}

impl<N: Network> ToBytes for Entry<N> {
    /// Writes the prover solution to the buffer.
    fn write_le<W: Write>(&self, mut writer: W) -> IoResult<()> {
        // Write the version.
        0u8.write_le(&mut writer)?;
        // Write the entry.
        match self {
            Entry::Solution(solution) => {
                0u8.write_le(&mut writer)?;
                solution.serialize_blocking_into(&mut writer).map_err(|e| error(e.to_string()))
            }
            Entry::Transaction(transaction) => {
                1u8.write_le(&mut writer)?;
                transaction.serialize_blocking_into(&mut writer).map_err(|e| error(e.to_string()))
            }
        }
    }
}
