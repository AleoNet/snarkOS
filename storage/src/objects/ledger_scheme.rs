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

use crate::*;
use snarkvm_algorithms::merkle_tree::*;
use snarkvm_dpc::{Block, LedgerError, LedgerScheme, Parameters, Storage, TransactionScheme};
use snarkvm_utilities::{
    bytes::{FromBytes, ToBytes},
    to_bytes_le,
};

use arc_swap::ArcSwap;
use std::{fs, marker::PhantomData, path::Path, sync::Arc};

impl<C: Parameters, T: TransactionScheme, S: Storage> LedgerScheme<C> for Ledger<C, T, S> {
    type Block = Block<Self::Transaction>;
    type Transaction = T;

    /// Instantiates a new ledger with a genesis block.
    fn new(path: Option<&Path>, genesis_block: Self::Block) -> anyhow::Result<Self> {
        let storage = if let Some(path) = path {
            fs::create_dir_all(&path).map_err(|err| LedgerError::Message(err.to_string()))?;

            S::open(Some(path), None)
        } else {
            S::open(None, None) // this must mean we're using an in-memory storage
        }?;

        if let Some(block_num) = storage.get(COL_META, KEY_BEST_BLOCK_NUMBER.as_bytes())? {
            if bytes_to_u32(&block_num) != 0 {
                return Err(LedgerError::ExistingDatabase.into());
            }
        }

        let leaves: &[[u8; 32]] = &[];
        let parameters = Arc::new(C::record_commitment_tree_parameters().clone());
        let empty_cm_merkle_tree = MerkleTree::new(parameters.clone(), leaves)?;

        let ledger_storage = Self {
            current_block_height: Default::default(),
            storage,
            cm_merkle_tree: ArcSwap::new(Arc::new(empty_cm_merkle_tree)),
            _transaction: PhantomData,
        };

        ledger_storage.insert_and_commit(&genesis_block)?;

        Ok(ledger_storage)
    }

    /// Returns the number of blocks including the genesis block
    fn block_height(&self) -> usize {
        self.get_current_block_height() as usize + 1
    }

    /// Return a digest of the latest ledger Merkle tree.
    fn latest_digest(&self) -> Option<MerkleTreeDigest<C::RecordCommitmentTreeParameters>> {
        let digest = match self.storage.get(COL_META, KEY_CURR_DIGEST.as_bytes()).unwrap() {
            Some(current_digest) => current_digest,
            None => to_bytes_le![self.cm_merkle_tree.load().root()].unwrap(),
        };
        Some(FromBytes::read_le(digest.as_slice()).unwrap())
    }

    /// Check that st_{ts} is a valid digest for some (past) ledger state.
    fn validate_digest(&self, digest: &MerkleTreeDigest<C::RecordCommitmentTreeParameters>) -> bool {
        self.storage.exists(COL_DIGEST, &digest.to_bytes_le().unwrap())
    }

    /// Returns true if the given commitment exists in the ledger.
    fn contains_commitment(&self, commitment: &C::RecordCommitment) -> bool {
        self.storage.exists(COL_COMMITMENT, &commitment.to_bytes_le().unwrap())
    }

    /// Returns true if the given serial number exists in the ledger.
    fn contains_serial_number(&self, serial_number: &C::AccountSignaturePublicKey) -> bool {
        self.storage
            .exists(COL_SERIAL_NUMBER, &serial_number.to_bytes_le().unwrap())
    }

    /// Returns the Merkle path to the latest ledger digest
    /// for a given commitment, if it exists in the ledger.
    fn prove_cm(&self, cm: &C::RecordCommitment) -> anyhow::Result<MerklePath<C::RecordCommitmentTreeParameters>> {
        let cm_index = self
            .get_cm_index(&cm.to_bytes_le()?)?
            .ok_or(LedgerError::InvalidCmIndex)?;
        let result = self.cm_merkle_tree.load().generate_proof(cm_index, cm)?;

        Ok(result)
    }
}
