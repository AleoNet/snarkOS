use crate::{reference::Round, Address};

/// This value defines the height of a block, which is always less than or equal to the round number.
pub type Height = u32;

// FIXME: integrate with the snarkVM BlockHash OR height
pub type BlockHash = u64;

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct Header {}

#[derive(Clone, Debug)]
pub struct Block {
    // A unique digest of author, round, payload, qc.vote info.id and qc.signatures
    pub hash: BlockHash,

    // The leader of the round, may not be the same as qc.author after view-change
    pub leader: Address,
    // The round that generated this proposal
    pub round: Round,
    // Proposed transaction(s)
    pub payload: Vec<()>,
    // QC for parent block
    pub qc: crate::reference::message::QuorumCertificate,
}

impl Block {
    /// Returns the round number of the block.
    pub const fn round(&self) -> Round {
        self.round
    }

    /// Returns the leader of the round.
    pub const fn leader(&self) -> Address {
        self.leader
    }
}

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
