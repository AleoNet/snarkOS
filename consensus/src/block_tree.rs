use crate::{
    bft::Round,
    block::{Block, BlockHash},
    hash,
    ledger::Ledger,
    message::Vote,
    Signature,
    F,
};

#[cfg(feature = "test")]
use crate::message::TestMessage;

use std::{cmp, collections::HashMap, hash::Hash};

#[cfg(feature = "test")]
use tokio::sync::mpsc;

#[derive(Clone, Debug, Hash)]
pub struct VoteInfo {
    // Id of block
    pub id: BlockHash,
    // round of block
    pub round: Round,
    // Id of parent
    pub parent_id: BlockHash,
    // round of parent
    pub parent_round: Round,
    // Speculated execution state
    pub exec_state_id: Option<()>,
}

// speculated new committed state to vote directly on
#[derive(Clone, Debug, Hash)]
pub struct LedgerCommitInfo {
    // ⊥ if no commit happens when this vote is aggregated to QC
    pub commit_state_id: Option<()>,
    // Hash of VoteMsg.vote info
    pub vote_info_hash: u64,
}

// QC is a VoteMsg with multiple signatures
#[derive(Clone, Debug)]
pub struct QuorumCertificate {
    pub vote_info: VoteInfo,
    ledger_commit_info: LedgerCommitInfo,
    // A quorum of signatures
    pub signatures: Vec<Signature>,
    // The validator that produced the qc
    author: (),
    author_signature: (),
}

impl QuorumCertificate {
    /// Returns the signatures.
    pub fn signatures(&self) -> &[Signature] {
        &self.signatures
    }
}

impl PartialEq for QuorumCertificate {
    fn eq(&self, other: &Self) -> bool {
        todo!()
    }
}

impl Eq for QuorumCertificate {}

impl Ord for QuorumCertificate {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        todo!()
    }
}

impl PartialOrd for QuorumCertificate {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

pub struct BlockTree {
    // tree of blocks pending commitment
    pending_block_tree: HashMap<BlockHash, Block>,
    // collected votes per block indexed by their LedgerInfo hash
    pending_votes: HashMap<u64, Vec<Vote>>,
    // highest known QC
    pub high_qc: QuorumCertificate,
    // highest QC that serves as a commit certificate
    pub high_commit_qc: QuorumCertificate,
}

impl BlockTree {
    pub fn new() -> Self {
        // are `high_qc` and `high_commit_qc` persistent? are they always available?
        // otherwise they should be `Option`s

        todo!()
    }

    pub fn process_qc(&mut self, qc: QuorumCertificate, ledger: &mut Ledger) {
        if qc.ledger_commit_info.commit_state_id.is_none() {
            ledger.commit(qc.vote_info.parent_id.clone());
            self.pending_block_tree.remove(&qc.vote_info.parent_id);
            if qc > self.high_commit_qc {
                self.high_commit_qc = qc.clone();
            }
        }

        if qc > self.high_qc {
            self.high_qc = qc;
        }
    }

    pub fn execute_and_insert(&mut self, b: Block, ledger: &mut Ledger) {
        ledger.speculate(b.qc.vote_info.parent_id.clone(), b.hash.clone(), b.payload.clone());

        self.pending_block_tree.insert(b.hash.clone(), b);
    }

    pub fn process_vote(&mut self, v: Vote, ledger: &mut Ledger) -> Option<QuorumCertificate> {
        self.process_qc(v.high_commit_qc, ledger);

        let vote_idx = hash(&v.ledger_commit_info);
        let mut pending_votes = self.pending_votes.entry(vote_idx).or_default();

        /*

        FIXME: so does this collection contain votes or signatures?
               are they the same thing here?

        pending_votes[vote_idx] ← pending_votes[vote_idx] ∪ v.signature

        pending_votes.push(v.signature);

        */

        if pending_votes.len() == 2 * F + 1 {
            /*

            FIXME: this QC is different than the one defined in page 10
                   is it just a broad way of creating a QC from VoteInfo and LedgerCommitInfo?

            QC〈
                vote_info ← v.vote_info,
                state_id ← v.state_id,
                votes ← self.pending_votes[vote idx]
            〉

            return Some(QuorumCertificate {

            })

            */
        }

        None
    }

    pub fn generate_block(&self, txns: Vec<()>, current_round: Round) -> Block {
        /*

        TODO: roll the values below into the desired hash

        let id = hash(&[AUTHOR, &current_round, &txns, &self.high_qc.vote_info.id, &self.high_qc.signatures]);

        */
        let id = 0;

        Block {
            hash: id,
            leader: todo!(), // TODO: it's the own validator ID
            round: current_round,
            payload: txns,
            qc: self.high_qc.clone(),
        }
    }
}
