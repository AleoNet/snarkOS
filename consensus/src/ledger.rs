use crate::block::{Block, BlockHash};

/// The ledger contains blocks that have been committed by consensus.
pub struct Ledger;

// TODO: these methods are not implemented in the whitepaper

impl Ledger {
    pub fn new() -> Self {
        todo!()
    }

    // apply txns speculatively
    pub fn speculate(&mut self, prev_block_hash: BlockHash, block_hash: BlockHash, txns: Vec<()>) -> () {
        todo!()
    }

    // find the pending state for the given block id or ⊥ if not present
    pub fn pending_state(&self, block_hash: BlockHash) -> Option<()> {
        todo!()
    }

    // commit the pending prefix of the given block id and prune other branches
    pub fn commit(&mut self, block_hash: BlockHash) {
        todo!()
    }

    /// Returns the block given the block hash.
    pub fn get_block(&self, block_hash: BlockHash) -> Block {
        todo!()
    }
}
