use crate::BlockStorage;
use snarkos_errors::storage::StorageError;
use snarkos_objects::{Block, BlockHeader, BlockHeaderHash};

impl BlockStorage {
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

    /// Returns true if the block corresponding to this block's previous_block_hash exists.
    pub fn previous_block_hash_exists(&self, block: &Block) -> bool {
        self.block_hash_exists(&block.header.previous_block_hash)
    }

    /// Get the latest block in the chain
    pub fn get_latest_block(&self) -> Result<Block, StorageError> {
        self.get_block_from_block_num(self.get_latest_block_height())
    }

    /// Find the potential parent block given a block header
    pub fn find_parent_block(&self, block_header: &BlockHeader) -> Result<Block, StorageError> {
        self.get_block(&block_header.previous_block_hash)
    }

    /// Returns true if the block exists in the canon chain.
    pub fn is_canon(&self, block_hash: &BlockHeaderHash) -> bool {
        self.block_hash_exists(block_hash) && self.get_block_num(block_hash).is_ok()
    }

    /// Returns true if the block corresponding to this block's previous_block_hash is in the canon chain.
    pub fn is_previous_block_canon(&self, block: &Block) -> bool {
        self.is_canon(&block.header.previous_block_hash)
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

    /// Get the list of block locator hashes (Bitcoin protocol)
    pub fn get_block_locator_hashes(&self) -> Result<Vec<BlockHeaderHash>, StorageError> {
        let mut step = 1;
        let mut index = self.get_latest_block_height();
        let mut block_locator_hashes = vec![];

        while index > 0 {
            block_locator_hashes.push(self.get_block_hash(index)?);

            if block_locator_hashes.len() >= 10 {
                step *= 2;
            }

            if index < step {
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
