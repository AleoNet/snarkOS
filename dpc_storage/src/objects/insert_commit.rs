use crate::{BlockStorage, SideChainPath, KEY_BEST_BLOCK_NUMBER};
use snarkos_errors::{
    objects::{BlockError, TransactionError},
    storage::StorageError,
};
use snarkos_objects::{Block, BlockHeaderHash};

use std::collections::HashMap;

impl BlockStorage {
    pub fn insert_only(&self, block: Block) -> Result<(), StorageError> {
        unimplemented!()
    }

    /// Commit/canonize a particular block.
    pub fn commit(&self, block_header_hash: BlockHeaderHash) -> Result<(), StorageError> {
        unimplemented!()
    }

    /// Insert a block into the storage and commit as part of the longest chain.
    pub fn insert_and_commit(&self, block: Block) -> Result<(), StorageError> {
        unimplemented!()
    }

    /// Returns true if the block exists in the canon chain.
    pub fn is_canon(&self, block_hash: &BlockHeaderHash) -> bool {
        unimplemented!()
    }

    /// Returns true if the block corresponding to this block's previous_block_hash is in the canon chain.
    pub fn is_previous_block_canon(&self, block: &Block) -> bool {
        unimplemented!()
    }

    /// Revert the chain to the state before the fork.
    pub fn revert_for_fork(&self, side_chain_path: &SideChainPath) -> Result<(), StorageError> {
        unimplemented!()
    }
}
