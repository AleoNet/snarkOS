use crate::{
    block_tree::{BlockTree, QuorumCertificate, VoteMsg},
    election::Election,
    ledger::Ledger,
    mempool::Mempool,
    message::{Message, ProposalMsg, TimeoutCertificate, TimeoutMsg},
    pacemaker::Pacemaker,
    safety::Safety,
};

struct Manager {
    block_tree: BlockTree,
    election: Election,
    ledger: Ledger,
    mempool: Mempool,
    pacemaker: Pacemaker,
    safety: Safety,
}

impl Manager {
    fn start_event_processing(&mut self, msg: Message) {
        match msg {
            Message::LocalTimeout => self.pacemaker.local_timeout_round(&self.block_tree, &mut self.safety),
            Message::Proposal(msg) => self.process_proposal_msg(msg),
            Message::Vote(msg) => self.process_vote_msg(msg),
            Message::Timeout(msg) => self.process_timeout_msg(msg),
        }
    }

    fn process_certificate_qc(&mut self, qc: QuorumCertificate) {
        self.block_tree.process_qc(qc.clone(), &mut self.ledger);
        self.election.update_leaders(qc, &self.pacemaker, &self.ledger);
        // FIXME: method not specified in the whitepaper
        // self.pacemaker.advance_round(qc.vote_info.round);
    }

    fn process_proposal_msg(&mut self, p: ProposalMsg) {
        self.process_certificate_qc(p.block.qc.clone());
        self.process_certificate_qc(p.high_commit_qc);

        self.pacemaker.advance_round_tc(p.last_round_tc.clone());

        // note: the whitepaper assigns to 'round' here
        let current_round = self.pacemaker.current_round;
        let leader = self.election.get_leader(current_round);

        // note: the whitepaper uses 'round' instead of 'current_round' here
        // note: the whitepaper uses 'p.sender' instead of 'p.signature' here
        if p.block.round != current_round || p.signature != leader || p.block.author != leader {
            return;
        }

        // note: the whitepaper passes the entire 'p' here instead of 'p.block'
        self.block_tree.execute_and_insert(p.block.clone(), &mut self.ledger); // Adds a new speculative state to the Ledger

        // FIXME: the whitepaper doesn't consider when 'p.last_round_tc' is ⊥
        if let Some(vote_msg) = self
            .safety
            .make_vote(p.block, p.last_round_tc.unwrap(), &self.ledger, &self.block_tree)
        {
            let leader = self.election.get_leader(current_round + 1);
            // TODO: send vote msg to the leader
        }
    }

    fn process_timeout_msg(&mut self, m: TimeoutMsg) {
        self.process_certificate_qc(m.tmo_info.high_qc.clone());
        self.process_certificate_qc(m.high_commit_qc.clone());

        self.pacemaker.advance_round_tc(m.last_round_tc.clone());

        if let Some(tc) = self.pacemaker.process_remote_timeout(m, &self.block_tree, &mut self.safety) {
            // FIXME: method not specified in the whitepaper again, and uses a different type now
            // self.pacemaker.advance_round(tc);
            self.process_new_round_event(Some(tc));
        }
    }

    fn process_vote_msg(&mut self, m: VoteMsg) {
        if let Some(qc) = self.block_tree.process_vote(m, &mut self.ledger) {
            self.process_certificate_qc(qc);
            self.process_new_round_event(None)
        }
    }

    fn process_new_round_event(&self, last_tc: Option<TimeoutCertificate>) {
        // TODO: if <US> == self.election.get_leader(self.pacemaker.current_round) {
        // Leader code: generate proposal.
        let block = self
            .block_tree
            .generate_block(self.mempool.get_transactions(), self.pacemaker.current_round);
        let proposal_msg = ProposalMsg::new(block, last_tc, self.block_tree.high_commit_qc.clone());
        // TODO: broadcast proposal_msg ProposalMsg〈b, last tc, Block-Tree.high commit qc〉
        // }
    }
}
