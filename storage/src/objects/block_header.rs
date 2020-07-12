use crate::{Ledger, COL_BLOCK_HEADER};
use snarkos_errors::storage::StorageError;
use snarkos_models::{algorithms::LoadableMerkleParameters, objects::Transaction};
use snarkos_objects::{Block, BlockHeader, BlockHeaderHash};
use snarkos_utilities::FromBytes;

impl<T: Transaction, P: LoadableMerkleParameters> Ledger<T, P> {
    /// Returns true if the block for the given block header hash exists.
    pub fn block_hash_exists(&self, block_hash: &BlockHeaderHash) -> bool {
        if self.is_empty() {
            return false;
        }

        match self.get_block_header(block_hash) {
            Ok(_block_header) => true,
            Err(_) => false,
        }
    }

    /// Get a block header given the block hash.
    pub fn get_block_header(&self, block_hash: &BlockHeaderHash) -> Result<BlockHeader, StorageError> {
        match self.storage.get(COL_BLOCK_HEADER, &block_hash.0)? {
            Some(block_header_bytes) => Ok(BlockHeader::read(&block_header_bytes[..])?),
            None => Err(StorageError::MissingBlockHeader(block_hash.to_string())),
        }
    }

    /// Returns true if the block corresponding to this block's previous_block_hash exists.
    pub fn previous_block_hash_exists(&self, block: &Block<T>) -> bool {
        self.block_hash_exists(&block.header.previous_block_hash)
    }

    /// Returns the latest shared block header hash.
    /// If the block locator hashes are for a side chain, returns the common point of fork.
    /// If the block locator hashes are for the canon chain, returns the latest block header hash.
    pub fn get_latest_shared_hash(
        &self,
        block_locator_hashes: Vec<BlockHeaderHash>,
    ) -> Result<BlockHeaderHash, StorageError> {
        for block_hash in block_locator_hashes {
            if self.is_canon(&block_hash) {
                return Ok(block_hash);
            }
        }

        self.get_block_hash(0)
    }

    /// Returns a list of block locator hashes. The purpose of this method is to detect
    /// wrong branches in the caller's canon chain.
    pub fn get_block_locator_hashes(&self) -> Result<Vec<BlockHeaderHash>, StorageError> {
        // Start from the latest block and work backwards
        let mut index = self.get_latest_block_height();

        // Update the step size with each iteration
        let mut step = 1;

        // The output list of block locator hashes
        let mut block_locator_hashes = vec![];

        while index > 0 {
            block_locator_hashes.push(self.get_block_hash(index)?);
            if block_locator_hashes.len() >= 10 {
                step *= 2;
            }

            // Check whether it is appropriate to terminate
            if index < step {
                // If the genesis block has not already been include, add it to the final output
                if index != 1 {
                    block_locator_hashes.push(self.get_block_hash(0)?);
                }
                break;
            }

            index -= step;
        }

        Ok(block_locator_hashes)
    }
}
