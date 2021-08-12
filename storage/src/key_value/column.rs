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

use num_enum::TryFromPrimitive;

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, TryFromPrimitive)]
pub enum KeyValueColumn {
    Meta = 0,          // MISC Values
    BlockHeader,       // Block hash -> block header
    BlockTransactions, // Block hash -> block transactions
    BlockIndex,        // Block num -> block hash && block hash -> block num
    TransactionLookup, // Transaction Hash -> (block hash and index)
    Commitment,        // Commitment -> index
    SerialNumber,      // SN -> index
    Memo,              // Memo -> index
    DigestIndex,       // Ledger digest -> index, index -> ledger digest
    Records,           // commitment -> record bytes
    ChildHashes,       // block hash -> vector of potential child hashes
    End,               // psuedo-column to signify count of columns
}

pub const KEY_BEST_BLOCK_NUMBER: &str = "BEST_BLOCK_NUMBER";
pub const KEY_MEMORY_POOL: &str = "MEMORY_POOL";
pub const KEY_CURR_CM_INDEX: &str = "CURRENT_CM_INDEX";
pub const KEY_CURR_SN_INDEX: &str = "CURRENT_SN_INDEX";
pub const KEY_CURR_MEMO_INDEX: &str = "CURRENT_MEMO_INDEX";
