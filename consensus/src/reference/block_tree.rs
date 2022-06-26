use crate::{
    reference::{
        hash,
        ledger::{Block, BlockHash, Ledger},
        message::{QuorumCertificate, Vote},
        Round,
        F,
    },
    Signature,
};

#[cfg(feature = "test")]
use crate::reference::message::TestMessage;

use std::{collections::HashMap, hash::Hash};

#[cfg(feature = "test")]
use tokio::sync::mpsc;

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
