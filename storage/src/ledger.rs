// Copyright (C) 2019-2020 Aleo Systems Inc.
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
use snarkos_algorithms::merkle_tree::MerkleTree;
use snarkos_errors::storage::StorageError;
use snarkos_models::{
    algorithms::LoadableMerkleParameters,
    genesis::Genesis,
    objects::{LedgerScheme, Transaction},
    parameters::Parameters,
};
use snarkos_objects::Block;
use snarkos_parameters::{GenesisBlock, LedgerMerkleTreeParameters};
use snarkos_utilities::bytes::FromBytes;

use parking_lot::RwLock;
use std::{
    fs,
    marker::PhantomData,
    path::{Path, PathBuf},
    sync::Arc,
};

pub struct Ledger<T: Transaction, P: LoadableMerkleParameters> {
    pub latest_block_height: RwLock<u32>,
    pub ledger_parameters: P,
    pub cm_merkle_tree: RwLock<MerkleTree<P>>,
    pub storage: Arc<Storage>,
    pub _transaction: PhantomData<T>,
}

impl<T: Transaction, P: LoadableMerkleParameters> Ledger<T, P> {
    /// Open the blockchain storage at a particular path.
    pub fn open_at_path<PATH: AsRef<Path>>(path: PATH) -> Result<Self, StorageError> {
        fs::create_dir_all(path.as_ref()).map_err(|err| StorageError::Message(err.to_string()))?;

        Self::load_ledger_state(path, true)
    }

    /// Open the blockchain storage at a particular path as a secondary read-only instance.
    pub fn open_secondary_at_path<PATH: AsRef<Path>>(path: PATH) -> Result<Self, StorageError> {
        fs::create_dir_all(path.as_ref()).map_err(|err| StorageError::Message(err.to_string()))?;

        Self::load_ledger_state(path, false)
    }

    /// Returns true if there are no blocks in the ledger.
    pub fn is_empty(&self) -> bool {
        self.get_latest_block().is_err()
    }

    /// Get the latest block height of the chain.
    pub fn get_latest_block_height(&self) -> u32 {
        *self.latest_block_height.read()
    }

    /// Get the latest number of blocks in the chain.
    pub fn get_block_count(&self) -> u32 {
        *self.latest_block_height.read() + 1
    }

    /// Get the stored old connected peers.
    pub fn get_peer_book(&self) -> Result<Vec<u8>, StorageError> {
        Ok(self.get(COL_META, &KEY_PEER_BOOK.as_bytes().to_vec())?)
    }

    /// Store the connected peers.
    pub fn store_to_peer_book(&self, peers_serialized: Vec<u8>) -> Result<(), StorageError> {
        let op = Op::Insert {
            col: COL_META,
            key: KEY_PEER_BOOK.as_bytes().to_vec(),
            value: peers_serialized,
        };
        self.storage.write(DatabaseTransaction(vec![op]))
    }

    /// Destroy the storage given a path.
    pub fn destroy_storage(path: PathBuf) -> Result<(), StorageError> {
        Storage::destroy_storage(path)
    }

    /// Returns a `Ledger` with the latest state loaded from storage at a given path as
    /// a primary or secondary ledger. A secondary ledger runs as a read-only instance.
    fn load_ledger_state<PATH: AsRef<Path>>(path: PATH, primary: bool) -> Result<Self, StorageError> {
        let mut secondary_path_os_string = path.as_ref().to_path_buf().into_os_string();
        secondary_path_os_string.push("_secondary");

        let secondary_path = PathBuf::from(secondary_path_os_string);

        let latest_block_number = {
            let storage = match primary {
                true => Storage::open_cf(path.as_ref(), NUM_COLS)?,
                false => Storage::open_secondary_cf(path.as_ref(), &secondary_path, NUM_COLS)?,
            };
            storage.get(COL_META, KEY_BEST_BLOCK_NUMBER.as_bytes())?
        };

        let crh = P::H::from(FromBytes::read(&LedgerMerkleTreeParameters::load_bytes()?[..])?);
        let ledger_parameters = P::from(crh);

        match latest_block_number {
            Some(val) => {
                let storage = match primary {
                    true => Storage::open_cf(path.as_ref(), NUM_COLS)?,
                    false => Storage::open_secondary_cf(path.as_ref(), &secondary_path, NUM_COLS)?,
                };

                // Build commitment merkle tree

                let mut cm_and_indices = vec![];

                for (commitment_key, index_value) in storage.get_iter(COL_COMMITMENT)? {
                    let commitment: T::Commitment = FromBytes::read(&commitment_key[..])?;
                    let index = bytes_to_u32(index_value.to_vec()) as usize;

                    cm_and_indices.push((commitment, index));
                }

                cm_and_indices.sort_by(|&(_, i), &(_, j)| i.cmp(&j));
                let commitments = cm_and_indices.into_iter().map(|(cm, _)| cm).collect::<Vec<_>>();

                let merkle_tree = MerkleTree::new(ledger_parameters.clone(), &commitments)?;

                Ok(Self {
                    latest_block_height: RwLock::new(bytes_to_u32(val)),
                    storage: Arc::new(storage),
                    cm_merkle_tree: RwLock::new(merkle_tree),
                    ledger_parameters,
                    _transaction: PhantomData,
                })
            }
            None => {
                // Add genesis block to database

                let genesis_block: Block<T> = FromBytes::read(GenesisBlock::load_bytes().as_slice())?;

                let ledger_storage = Self::new(&path.as_ref().to_path_buf(), ledger_parameters, genesis_block)
                    .expect("Ledger could not be instantiated");

                // If there did not exist a primary ledger at the path,
                // then create one and then open the secondary instance.
                if !primary {
                    return Self::load_ledger_state(path, primary);
                }

                Ok(ledger_storage)
            }
        }
    }

    /// Attempt to catch the secondary read-only storage instance with the primary instance.
    pub fn catch_up_secondary(&self, update_merkle_tree: bool) -> Result<(), StorageError> {
        // Sync the secondary and primary instances
        if self.storage.db.try_catch_up_with_primary().is_ok() {
            let latest_block_height_bytes = self.get(COL_META, &KEY_BEST_BLOCK_NUMBER.as_bytes().to_vec())?;
            let new_latest_block_height = bytes_to_u32(latest_block_height_bytes);
            let mut latest_block_height = self.latest_block_height.write();

            // If the new block height is greater than the stored block height,
            // update the block height and merkle tree.
            if new_latest_block_height > *latest_block_height {
                // Update the latest block height of the secondary instance.
                *latest_block_height = new_latest_block_height;

                // Optional `cm_merkle_tree` regeneration because not all usages of
                // the secondary instance requires it.
                if update_merkle_tree {
                    // Update the Merkle tree of the secondary instance.
                    let mut merkle_tree = self.cm_merkle_tree.write();
                    *merkle_tree = self.build_merkle_tree(vec![])?;
                }
            }
        }

        Ok(())
    }

    /// Retrieve a value given a key.
    pub(crate) fn get(&self, col: u32, key: &[u8]) -> Result<Vec<u8>, StorageError> {
        match self.storage.get(col, key)? {
            Some(data) => Ok(data),
            None => Err(StorageError::MissingValue(hex::encode(key))),
        }
    }
}
