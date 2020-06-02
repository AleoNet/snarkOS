use crate::Ledger;
use snarkos_algorithms::merkle_tree::MerkleParameters;
use snarkos_errors::{objects::BlockError, storage::StorageError};
use snarkos_models::objects::Transaction;
use snarkos_objects::{BlockHeader, BlockHeaderHash};

const OLDEST_FORK_THRESHOLD: u32 = 1024;

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

impl<T: Transaction, P: MerkleParameters> Ledger<T, P> {
    /// Get the block's path/origin.
    pub fn get_block_path(&self, block_header: &BlockHeader) -> Result<BlockPath, StorageError> {
        let block_hash = block_header.get_hash();
        if self.block_hash_exists(&block_hash) {
            return Ok(BlockPath::ExistingBlock);
        }

        if &self.get_latest_block()?.header.get_hash() == &block_header.previous_block_hash {
            return Ok(BlockPath::CanonChain(self.get_latest_block_height() + 1));
        }

        let mut side_chain_path = vec![];
        let mut parent_hash = block_header.previous_block_hash.clone();

        for _ in 0..=OLDEST_FORK_THRESHOLD {
            // check if the part is part of the canon chain
            match &self.get_block_num(&parent_hash) {
                // This is a canon parent
                Ok(block_num) => {
                    // Add the children from the latest block

                    let longest_path = self.longest_child_path(block_hash)?;

                    side_chain_path.extend(longest_path.1);

                    return Ok(BlockPath::SideChain(SideChainPath {
                        shared_block_number: *block_num,
                        new_block_number: block_num + side_chain_path.len() as u32,
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

        Err(StorageError::BlockError(BlockError::IrrelevantBlock(
            block_hash.to_string(),
        )))
    }

    /// Find the longest path of non-canon children from the given block header
    pub fn longest_child_path(
        &self,
        block_hash: BlockHeaderHash,
    ) -> Result<(usize, Vec<BlockHeaderHash>), StorageError> {
        let children = self.get_child_hashes(&block_hash)?;

        let mut final_path = vec![block_hash];

        if children.len() == 0 {
            Ok((1, final_path))
        } else {
            let mut paths = vec![];
            for child in children {
                paths.push(self.longest_child_path(child)?);
            }

            match paths.iter().max_by_key(|x| x.0) {
                Some((longest_child_path_length, longest_child_path)) => {
                    final_path.extend(longest_child_path.clone());

                    Ok((*longest_child_path_length + 1, final_path))
                }
                None => Ok((1, final_path)),
            }
        }
    }
}
