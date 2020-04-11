use crate::{BlockStorage, SideChainPath};
use snarkos_errors::storage::StorageError;
use snarkos_objects::{
    dpc::{Block, Transaction},
    BlockHeaderHash,
};

//use std::collections::HashMap;

impl<T: Transaction> BlockStorage<T> {
    pub fn insert_only(&self, _block: Block<T>) -> Result<(), StorageError> {
        unimplemented!()
    }

    /// Commit/canonize a particular block.
    pub fn commit(&self, _block_header_hash: BlockHeaderHash) -> Result<(), StorageError> {
        unimplemented!()
    }

    /// Insert a block into the storage and commit as part of the longest chain.
    pub fn insert_and_commit(&self, _block: Block<T>) -> Result<(), StorageError> {
        unimplemented!()
    }

    /// Returns true if the block exists in the canon chain.
    pub fn is_canon(&self, _block_hash: &BlockHeaderHash) -> bool {
        unimplemented!()
    }

    /// Returns true if the block corresponding to this block's previous_block_h.is_canon(&block_haash is in the canon chain.
    pub fn is_previous_block_canon(&self, _block: Block<T>) -> bool {
        unimplemented!()
    }

    /// Revert the chain to the state before the fork.
    pub fn revert_for_fork(&self, _side_chain_path: &SideChainPath) -> Result<(), StorageError> {
        unimplemented!()
    }
}
