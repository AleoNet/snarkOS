use crate::block_tree::{Block, BlockId};

// TODO: integrate with snarkOS ledger
pub struct Ledger;

// TODO: these methods are not implemented in the whitepaper

impl Ledger {
    pub fn new() -> Self {
        todo!()
    }

    // apply txns speculatively
    pub fn speculate(&mut self, prev_block_id: BlockId, block_id: BlockId, txns: Vec<()>) -> () {
        todo!()
    }

    // find the pending state for the given block id or ⊥ if not present
    pub fn pending_state(&self, block_id: BlockId) -> Option<()> {
        todo!()
    }

    // commit the pending prefix of the given block id and prune other branches
    pub fn commit(&mut self, block_id: BlockId) {
        todo!()
    }

    // returns a committed block given its id
    pub fn committed_block(&self, block_id: BlockId) -> Block {
        todo!()
    }
}
