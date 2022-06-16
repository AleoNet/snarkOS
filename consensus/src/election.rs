use std::collections::HashMap;

use crate::{block_tree::QuorumCertificate, ledger::Ledger, pacemaker::Pacemaker, Round};

pub struct Election {
    // The list of current validators
    validators: Vec<()>,
    // A parameter for the leader reputation algorithm
    window_size: usize,
    // Between f and 2f , number of excluded authors of last committed blocks
    exclude_size: usize,
    // Map from round numbers to leaders elected due to the reputation scheme
    reputation_leaders: HashMap<Round, ()>,
}

impl Election {
    pub fn new() -> Self {
        // is the list of validators known beforehand?

        todo!()
    }

    pub fn elect_reputation_leader(&self, qc: QuorumCertificate, ledger: &Ledger) -> () {
        let mut active_validators = vec![]; // validators that signed the last window size committed blocks
        let mut last_authors = vec![]; // ordered set of authors of last exclude size committed blocks
        let mut current_qc = qc.clone();

        let mut i = 0;
        while i < self.window_size || last_authors.len() < self.exclude_size {
            let current_block = ledger.committed_block(current_qc.vote_info.parent_id);
            let block_author = current_block.author;

            if i < self.window_size {
                active_validators.extend_from_slice(&current_qc.signatures /* .signers() FIXME */);
                // whitepaper comment:
                // |current qc.signatures.signers()| ≥ 2f + 1
            }

            if last_authors.len() < self.exclude_size {
                last_authors.push(block_author);
            }

            current_qc = current_block.qc.clone();

            i += 1;
        }

        active_validators = active_validators.into_iter().filter(|v| !last_authors.contains(v)).collect();

        // TODO: pick an active validator
        // active validators.pick_one(seed ← qc.voteinfo.round)
    }

    pub fn update_leaders(&mut self, qc: QuorumCertificate, pacemaker: &Pacemaker, ledger: &Ledger) {
        let extended_round = qc.vote_info.parent_round;
        let qc_round = qc.vote_info.round;
        let current_round = pacemaker.current_round;

        if extended_round + 1 == qc_round && qc_round + 1 == current_round {
            self.reputation_leaders
                .insert(current_round + 1, self.elect_reputation_leader(qc, ledger));
        }
    }

    pub fn get_leader(&self, round: Round) -> () {
        if let Some(leader) = self.reputation_leaders.get(&round) {
            leader.clone() // Reputation-based leader
        } else {
            self.validators[(round as f32 / 2.0).floor() as usize % self.validators.len()] // Round-robin leader (two rounds per leader)
        }
    }
}
