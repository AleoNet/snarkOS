use std::cmp;

use crate::{
    block_tree::{Block, BlockTree, LedgerCommitInfo, QuorumCertificate, VoteInfo, VoteMsg},
    hash,
    ledger::Ledger,
    message::{TimeoutCertificate, TimeoutInfo},
    BlockRound,
    QcRound,
    Round,
};

pub struct Safety {
    // Own private key
    private_key: (),
    // Public keys of all validators
    public_keys: Vec<()>,
    // initially 0
    highest_vote_round: Round,
    highest_qc_round: QcRound,
}

impl Safety {
    pub fn new() -> Self {
        // do `highest_vote_round` and `highest_qc_round` persist?

        todo!()
    }

    fn increase_highest_vote_round(&mut self, round: Round) {
        // commit not to vote in rounds lower than round
        if round > self.highest_vote_round {
            self.highest_vote_round = round;
        }
    }

    fn update_highest_qc_round(&mut self, qc_round: QcRound) {
        if qc_round > self.highest_qc_round {
            self.highest_qc_round = qc_round;
        }
    }

    fn consecutive(&self, block_round: BlockRound, round: Round) -> bool {
        round + 1 == block_round
    }

    fn safe_to_extend(&self, block_round: BlockRound, qc_round: QcRound, tc: TimeoutCertificate) -> bool {
        // TODO: is the unwrap safe here?
        self.consecutive(block_round, tc.round) && qc_round >= *tc.tmo_high_qc_rounds.iter().max().unwrap()
    }

    fn safe_to_vote(&self, block_round: BlockRound, qc_round: QcRound, tc: TimeoutCertificate) -> bool {
        if block_round <= cmp::max(self.highest_qc_round, qc_round) {
            // 1. must vote in monotonically increasing rounds
            // 2. must extend a smaller round
            false
        } else {
            // Extending qc from previous round or safe to extend due to tc
            self.consecutive(block_round, qc_round) || self.safe_to_extend(block_round, qc_round, tc)
        }
    }

    fn safe_to_timeout(&self, round: Round, qc_round: QcRound, tc: TimeoutCertificate) -> bool {
        if qc_round < self.highest_qc_round || round <= cmp::max(self.highest_vote_round - 1, qc_round) {
            // respect highest qc round and don’t timeout in a past round
            false
        } else {
            // qc or tc must allow entering the round to timeout
            self.consecutive(round, qc_round) || self.consecutive(round, tc.round)
        }
    }

    fn commit_state_id_candidate(&self, block_round: BlockRound, qc: QuorumCertificate, ledger: &Ledger) -> Option<()> {
        // find the committed id in case a qc is formed in the vote round
        if self.consecutive(block_round, qc.vote_info.round) {
            ledger.pending_state(qc.vote_info.id)
        } else {
            None
        }
    }

    pub fn make_vote(&mut self, b: Block, last_tc: TimeoutCertificate, ledger: &Ledger, block_tree: &BlockTree) -> Option<VoteMsg> {
        let qc_round = b.qc.vote_info.round;

        if valid_signatures(&b.qc.signatures) && valid_signatures(&last_tc.signatures) && self.safe_to_vote(b.round, qc_round, last_tc) {
            self.update_highest_qc_round(qc_round); // Protect qc round
            self.increase_highest_vote_round(b.round); // Don’t vote again in this (or lower) round

            // VoteInfo carries the potential QC info with ids and rounds of the parent QC
            let vote_info = VoteInfo {
                id: b.id,
                round: b.round,
                parent_id: b.qc.vote_info.id,
                parent_round: qc_round,
                exec_state_id: ledger.pending_state(b.id),
            };

            let ledger_commit_info = LedgerCommitInfo {
                commit_state_id: self.commit_state_id_candidate(b.round, b.qc, ledger),
                vote_info_hash: hash(&vote_info),
            };

            Some(VoteMsg::new(vote_info, ledger_commit_info, block_tree.high_commit_qc.clone(), ()))
        } else {
            None
        }
    }

    pub fn make_timeout(&mut self, round: Round, high_qc: QuorumCertificate, last_tc: TimeoutCertificate) -> Option<TimeoutInfo> {
        let qc_round = high_qc.vote_info.round;

        if valid_signatures(&high_qc.signatures) && valid_signatures(&last_tc.signatures) && self.safe_to_timeout(round, qc_round, last_tc)
        {
            self.increase_highest_vote_round(round); // Stop voting for round

            Some(TimeoutInfo::new(round, high_qc, ()))
        } else {
            None
        }
    }
}

fn valid_signatures(signatures: &[()]) -> bool {
    // valid signatures call in the beginning of these functions checks
    // the well-formedness and signatures on all parameters provided
    // to construct the votes (using the public keys of other validators

    true
}
