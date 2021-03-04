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

use crate::{error::StorageError, DatabaseTransaction, Ledger, Op, COL_META, KEY_MEMORY_POOL};
use snarkvm_algorithms::traits::LoadableMerkleParameters;
use snarkvm_objects::Transaction;

impl<T: Transaction, P: LoadableMerkleParameters> Ledger<T, P> {
    /// Get the stored memory pool transactions.
    pub fn get_memory_pool(&self) -> Result<Vec<u8>, StorageError> {
        self.get(COL_META, &KEY_MEMORY_POOL.as_bytes().to_vec())
    }

    /// Store the memory pool transactions.
    pub fn store_to_memory_pool(&self, transactions_serialized: Vec<u8>) -> Result<(), StorageError> {
        let op = Op::Insert {
            col: COL_META,
            key: KEY_MEMORY_POOL.as_bytes().to_vec(),
            value: transactions_serialized,
        };
        self.storage.write(DatabaseTransaction(vec![op]))
    }
}
