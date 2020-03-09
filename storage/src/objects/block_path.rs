use crate::BlockStorage;
use snarkos_errors::storage::StorageError;
use snarkos_objects::{BlockHeader, BlockHeaderHash};

//TODO: MOVE ALL OF THIS
#[derive(Clone, Debug)]
pub enum BlockPath {
    ExistingBlock,
    CanonChain(u32),
    SideChain(SideChainPath),
}

#[derive(Clone, Debug)]
pub struct SideChainPath {
    /// Latest block number before diverging from the canon chain.
    pub shared_block_number: u32,

    /// New block number
    pub new_block_number: u32,

    /// Path of block hashes from the shared block to the latest diverging block (oldest first).
    pub path: Vec<BlockHeaderHash>,
}

impl BlockStorage {
    /// Get the block's path/origin.
    pub fn get_block_path(&self, block_header: &BlockHeader) -> Result<BlockPath, StorageError> {
        let block_hash = block_header.get_hash();
        if self.block_hash_exists(&block_hash) {
            return Ok(BlockPath::ExistingBlock);
        }

        if &self.get_latest_block()?.header.get_hash() == &block_header.previous_block_hash {
            return Ok(BlockPath::CanonChain(self.get_latest_block_height() + 1));
        }

        const OLDEST_FORK_THRESHOLD: u32 = 1024;
        let mut side_chain_path = vec![];
        let mut parent_hash = block_header.previous_block_hash.clone();

        for _ in 0..=OLDEST_FORK_THRESHOLD {
            // check if the part is part of the canon chain
            match &self.get_block_num(&parent_hash) {
                // This is a canon parent
                Ok(block_num) => {
                    return Ok(BlockPath::SideChain(SideChainPath {
                        shared_block_number: *block_num,
                        new_block_number: block_num + side_chain_path.len() as u32 + 1,
                        path: side_chain_path,
                    }));
                }
                // Add to the side_chain_path
                Err(_) => {
                    side_chain_path.insert(0, parent_hash.clone());
                    parent_hash = self.get_block_header(&parent_hash)?.previous_block_hash;
                }
            }
        }

        Err(StorageError::IrrelevantBlock)
    }
}
