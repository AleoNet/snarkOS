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

use snarkvm_algorithms::{merkle_tree::*, traits::LoadableMerkleParameters};
use snarkvm_dpc::{Block, LedgerScheme, Parameters, TransactionScheme};

use std::{marker::PhantomData, path::Path, sync::Arc};

pub struct EmptyLedger<C: Parameters, T: TransactionScheme> {
    _parameters: PhantomData<C>,
    _transaction: PhantomData<T>,
}

impl<C: Parameters, T: TransactionScheme> LedgerScheme<C> for EmptyLedger<C, T> {
    type Block = Block<Self::Transaction>;
    type Transaction = T;

    /// Instantiates a new ledger with a genesis block.
    fn new(_path: Option<&Path>, _genesis_block: Self::Block) -> anyhow::Result<Self> {
        Ok(Self {
            _parameters: PhantomData,
            _transaction: PhantomData,
        })
    }

    /// Returns the number of blocks including the genesis block.
    fn block_height(&self) -> usize {
        0
    }

    /// Return a digest of the latest ledger Merkle tree.
    fn latest_digest(&self) -> Option<MerkleTreeDigest<C::RecordCommitmentTreeParameters>> {
        None
    }

    /// Check that st_{ts} is a valid digest for some (past) ledger state.
    fn validate_digest(&self, digest: &MerkleTreeDigest<C::RecordCommitmentTreeParameters>) -> bool {
        true
    }

    /// Returns true if the given commitment exists in the ledger.
    fn contains_commitment(&self, commitment: &C::RecordCommitment) -> bool {
        false
    }

    /// Returns true if the given serial number exists in the ledger.
    fn contains_serial_number(&self, serial_number: &C::AccountSignaturePublicKey) -> bool {
        false
    }

    /// Returns the Merkle path to the latest ledger digest
    /// for a given commitment, if it exists in the ledger.
    fn prove_cm(&self, cm: &C::RecordCommitment) -> anyhow::Result<MerklePath<C::RecordCommitmentTreeParameters>> {
        unimplemented!()
    }
}
