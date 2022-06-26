#[cfg(feature = "test")]
use crate::message::TestMessage;
use crate::{
    bft::Round,
    block::Block,
    block_tree::{BlockTree, LedgerCommitInfo, QuorumCertificate, VoteInfo},
    hash,
    ledger::Ledger,
    message::{Message, Propose, Timeout, TimeoutCertificate, TimeoutInfo, Vote},
    Address,
    Signature,
    F,
};

use anyhow::Result;
use std::{cmp, collections::HashMap};

#[cfg(feature = "test")]
use tokio::sync::mpsc;

// TODO: integrate with snarkVM's mempool
pub struct Mempool;

impl Mempool {
    pub fn new() -> Self {
        Self
    }

    pub fn get_transactions(&self) -> Vec<()> {
        todo!() // not implemented in the whitepaper
    }
}

/// The central object responsible for the consensus process.
// TODO: once the initial implementation is finalized, this
// should likely be made into a finite state machine.
pub struct Validator {
    block_tree: BlockTree,
    ledger: Ledger,
    mempool: Mempool,

    // Leader Selection //

    // The list of current validators
    validators: Vec<Address>,
    // A parameter for the leader reputation algorithm
    window_size: usize,
    // Between f and 2f , number of excluded authors of last committed blocks
    exclude_size: usize,
    // Map from round numbers to leaders elected due to the reputation scheme
    reputation_leaders: HashMap<Round, Address>,

    // Pacemaker //

    // Initially zero
    pub current_round: Round,
    // Initially ⊥
    last_round_tc: Option<TimeoutCertificate>,
    // Timeouts per round
    pending_timeouts: HashMap<Round, HashMap<(), TimeoutInfo>>,

    // Safety //

    // Own private key
    private_key: (),
    // Public keys of all validators
    public_keys: Vec<()>,
    // initially 0
    highest_vote_round: Round,
    highest_qc_round: Round,

    // Testing //

    // Used to send messages to other validators in tests.
    #[cfg(feature = "test")]
    outbound_sender: mpsc::Sender<TestMessage>,
}

impl Validator {
    #[cfg(not(feature = "test"))]
    pub fn new(/* TODO: pass the ledger here */) -> Self {
        // do `highest_vote_round` and `highest_qc_round` persist?
        Self {
            block_tree: BlockTree::new(),
            ledger: Ledger::new(),
            mempool: Mempool::new(),

            validators: vec![],
            window_size: 0,
            exclude_size: 0,
            reputation_leaders: HashMap::new(),

            current_round: 0,
            last_round_tc: None,
            pending_timeouts: HashMap::new(),

            private_key: (),
            public_keys: vec![],
            highest_vote_round: 0,
            highest_qc_round: 0,
        }
    }

    #[cfg(feature = "test")]
    pub fn new(
        // TODO: include the same arguments as the non-test version
        outbound_sender: mpsc::Sender<TestMessage>, // a clone of `common_msg_sender`
    ) -> Self {
        // do `highest_vote_round` and `highest_qc_round` persist?
        Self {
            block_tree: BlockTree::new(),
            ledger: Ledger::new(),
            mempool: Mempool::new(),

            validators: vec![],
            window_size: 0,
            exclude_size: 0,
            reputation_leaders: HashMap::new(),

            current_round: 0,
            last_round_tc: None,
            pending_timeouts: HashMap::new(),

            private_key: (),
            public_keys: vec![],
            highest_vote_round: 0,
            highest_qc_round: 0,

            outbound_sender,
        }
    }

    pub fn start_event_processing(&mut self, msg: Message) -> Result<()> {
        match msg {
            Message::LocalTimeout => self.local_timeout_round(),
            Message::Propose(msg) => self.process_propose(msg),
            Message::Timeout(msg) => self.process_timeout(msg),
            Message::Vote(msg) => self.process_vote(msg),
        }
    }

    fn process_propose(&mut self, propose: Propose) -> Result<()> {
        self.process_qc(propose.block.qc.clone())?;
        self.process_qc(propose.high_commit_qc.clone())?;

        self.advance_round_tc(propose.last_round_tc.clone());

        // note: the whitepaper assigns to 'round' here
        let current_round = self.current_round;
        let leader = self.get_leader(current_round);

        // note: the whitepaper uses 'round' instead of 'current_round' here
        // note: the whitepaper uses 'p.sender' instead of 'p.signature' here
        if propose.block.round() != current_round || propose.leader()? != *leader || propose.block.leader() != *leader {
            return Ok(());
        }

        // note: the whitepaper passes the entire 'p' here instead of 'p.block'
        self.block_tree.execute_and_insert(propose.block.clone(), &mut self.ledger); // Adds a new speculative state to the Ledger

        // FIXME: the whitepaper doesn't consider when 'p.last_round_tc' is ⊥
        if let Some(vote_msg) = self.make_vote(propose.block, propose.last_round_tc.unwrap()) {
            let leader = self.get_leader(current_round + 1);

            // TODO: send vote msg to the leader

            #[cfg(feature = "test")]
            self.outbound_sender
                .blocking_send(TestMessage::new(todo!(), Some(todo!())))
                .unwrap();
        }

        Ok(())
    }

    fn process_timeout(&mut self, timeout: Timeout) -> Result<()> {
        self.process_qc(timeout.tmo_info.high_qc.clone())?;
        self.process_qc(timeout.high_commit_qc.clone())?;

        self.advance_round_tc(timeout.last_round_tc.clone());

        if let Some(tc) = self.process_remote_timeout(timeout) {
            // FIXME: method not specified in the whitepaper again, and uses a different type now
            // self.advance_round(tc);
            self.process_new_round_event(Some(tc));
        }

        Ok(())
    }

    fn process_vote(&mut self, vote: Vote) -> Result<()> {
        if let Some(qc) = self.block_tree.process_vote(vote, &mut self.ledger) {
            self.process_qc(qc)?;
            self.process_new_round_event(None);
        }

        Ok(())
    }

    fn process_qc(&mut self, qc: QuorumCertificate) -> Result<()> {
        self.block_tree.process_qc(qc.clone(), &mut self.ledger);
        self.update_leaders(qc)
        // FIXME: method not specified in the whitepaper
        // self.advance_round(qc.vote_info.round);
    }

    fn process_new_round_event(&self, last_tc: Option<TimeoutCertificate>) {
        // TODO: if <US> == self.election.get_leader(self.current_round) {
        // Leader code: generate proposal.
        let block = self.block_tree.generate_block(self.mempool.get_transactions(), self.current_round);
        let proposal_msg = Propose::new(block, last_tc, self.block_tree.high_commit_qc.clone(), todo!());

        // TODO: broadcast proposal_msg ProposalMsg〈b, last tc, Block-Tree.high commit qc〉

        #[cfg(feature = "test")]
        self.outbound_sender.blocking_send(TestMessage::new(todo!(), None)).unwrap();
        // }
    }
}

// Leader selection
impl Validator {
    pub fn get_leader(&self, round: Round) -> &Address {
        if let Some(leader) = self.reputation_leaders.get(&round) {
            leader
        } else {
            &self.validators[(round as f32 / 2.0).floor() as usize % self.validators.len()] // Round-robin leader (two rounds per leader)
        }
    }

    pub fn update_leaders(&mut self, qc: QuorumCertificate) -> Result<()> {
        let extended_round = qc.vote_info.parent_round;
        let qc_round = qc.vote_info.round;
        let current_round = self.current_round;

        if extended_round + 1 == qc_round && qc_round + 1 == current_round {
            self.reputation_leaders.insert(current_round + 1, self.elect_reputation_leader(qc)?);
        }

        Ok(())
    }

    fn elect_reputation_leader(&self, qc: QuorumCertificate) -> Result<Address> {
        let mut active_validators = vec![]; // validators that signed the last window size committed blocks
        let mut past_leaders = vec![]; // ordered set of authors of last exclude size committed blocks
        let mut current_qc = qc.clone();

        let mut i = 0;
        while i < self.window_size || past_leaders.len() < self.exclude_size {
            if i < self.window_size {
                active_validators.extend(&current_qc.signatures().iter().map(|s| s.signer()).collect::<Result<Vec<_>>>()?);
                // whitepaper comment:
                // |current qc.signatures.signers()| ≥ 2f + 1
            }

            // Retrieve the current block.
            let current_block = self.ledger.get_block(current_qc.vote_info.parent_id);

            if past_leaders.len() < self.exclude_size {
                past_leaders.push(current_block.leader());
            }

            current_qc = current_block.qc.clone();

            i += 1;
        }

        active_validators = active_validators.into_iter().filter(|v| !past_leaders.contains(v)).collect();

        // TODO: pick an active validator
        // active validators.pick_one(seed ← qc.voteinfo.round)
        Ok(active_validators[0])
    }
}

// Pacemaker
impl Validator {
    // pub fn get_round_timer(&self, round: Round) -> () {
    //     // FIXME: timer
    //     // round timer formula // For example, use 4 × ∆ or α + βcommit gap(r) if ∆ is unknown.
    //
    //     todo!()
    // }
    //
    // pub fn start_timer(&mut self, new_round: Round) {
    //     // FIXME: timer
    //     // stop_timer(current round)
    //
    //     self.current_round = new_round;
    //
    //     // start local timer for round current round for duration get round timer(current round)
    //
    //     todo!()
    // }

    pub fn local_timeout_round(&mut self) -> Result<()> {
        // FIXME: what should this do
        // save_consensus_state()

        let high_qc = self.block_tree.high_qc.clone();
        // TODO: are the unwraps safe?
        let timeout_info = self
            .make_timeout(self.current_round, high_qc, self.last_round_tc.clone().unwrap())
            .unwrap();
        let timeout_msg = Timeout {
            tmo_info: timeout_info,
            last_round_tc: self.last_round_tc.clone(),
            high_commit_qc: self.block_tree.high_commit_qc.clone(),
        };

        // TODO: broadcast timeout_msg

        #[cfg(feature = "test")]
        self.outbound_sender.blocking_send(TestMessage::new(todo!(), None)).unwrap();

        Ok(())
    }

    pub fn process_remote_timeout(&mut self, tmo: Timeout) -> Option<TimeoutCertificate> {
        let tmo_info = &tmo.tmo_info;

        if tmo_info.round < self.current_round {
            return None;
        }

        if !self.pending_timeouts[&tmo_info.round].contains_key(&tmo_info.sender) {
            if let Some(infos) = self.pending_timeouts.get_mut(&tmo_info.round) {
                infos.insert(tmo_info.sender, tmo_info.clone());
            }
        }

        let num_round_senders = self.pending_timeouts[&tmo_info.round].len();

        if num_round_senders == F + 1 {
            // FIXME: timer
            // stop_timer(current round)

            self.local_timeout_round().unwrap() // Bracha timeout
        }

        if num_round_senders == 2 * F + 1 {
            return Some(TimeoutCertificate {
                round: tmo_info.round,
                // TODO: what's t? is it a bitwise OR? {t.high_qc.round | t ∈ pending_timeouts[tmo_info.round]}
                tmo_high_qc_rounds: todo!(),
                // TODO: what's t? is it a bitwise OR? {t.signature | t ∈ pending_timeouts[tmo_info.round]}
                signatures: todo!(),
            });
        }

        None
    }

    pub fn advance_round_tc(&mut self, tc: Option<TimeoutCertificate>) -> bool {
        if tc.is_none() || tc.as_ref().unwrap().round < self.current_round {
            return false;
        }

        self.last_round_tc = tc;

        // FIXME: timer
        // start timer(tc.round + 1)

        true
    }

    // pub fn advance_round_qc(&mut self, qc: QuorumCertificate) -> bool {
    //     if qc.vote_info.round < self.current_round {
    //         return false;
    //     }
    //
    //     self.last_round_tc = None;
    //
    //     // FIXME: timer
    //     // start timer(qc.vote_info.round + 1)
    //
    //     true
    // }
}

// Safety
impl Validator {
    pub fn make_vote(&mut self, block: Block, last_tc: TimeoutCertificate) -> Option<Vote> {
        let qc_round = block.qc.vote_info.round;

        if Self::is_valid_signatures(&block.qc.signatures)
            && Self::is_valid_signatures(&last_tc.signatures)
            && self.safe_to_vote(block.round, qc_round, last_tc)
        {
            self.update_highest_qc_round(qc_round); // Protect qc round
            self.increase_highest_vote_round(block.round); // Don’t vote again in this (or lower) round

            // VoteInfo carries the potential QC info with ids and rounds of the parent QC
            let vote_info = VoteInfo {
                id: block.hash,
                round: block.round,
                parent_id: block.qc.vote_info.id,
                parent_round: qc_round,
                exec_state_id: self.ledger.pending_state(block.hash),
            };

            let ledger_commit_info = LedgerCommitInfo {
                commit_state_id: self.commit_state_id_candidate(block.round(), block.qc),
                vote_info_hash: hash(&vote_info),
            };

            Some(Vote::new(vote_info, ledger_commit_info, self.block_tree.high_commit_qc.clone(), ()))
        } else {
            None
        }
    }

    pub fn make_timeout(&mut self, round: Round, high_qc: QuorumCertificate, last_tc: TimeoutCertificate) -> Option<TimeoutInfo> {
        let qc_round = high_qc.vote_info.round;

        if Self::is_valid_signatures(&high_qc.signatures)
            && Self::is_valid_signatures(&last_tc.signatures)
            && self.safe_to_timeout(round, qc_round, last_tc)
        {
            self.increase_highest_vote_round(round); // Stop voting for round

            Some(TimeoutInfo::new(round, high_qc, ()))
        } else {
            None
        }
    }
}

// Safety
impl Validator {
    fn increase_highest_vote_round(&mut self, round: Round) {
        // commit not to vote in rounds lower than round
        if round > self.highest_vote_round {
            self.highest_vote_round = round;
        }
    }

    fn update_highest_qc_round(&mut self, qc_round: Round) {
        if qc_round > self.highest_qc_round {
            self.highest_qc_round = qc_round;
        }
    }

    fn safe_to_extend(&self, block_round: Round, qc_round: Round, tc: TimeoutCertificate) -> bool {
        // TODO: is the unwrap safe here?
        Self::is_consecutive(block_round, tc.round) && qc_round >= *tc.tmo_high_qc_rounds.iter().max().unwrap()
    }

    fn safe_to_vote(&self, block_round: Round, qc_round: Round, tc: TimeoutCertificate) -> bool {
        if block_round <= cmp::max(self.highest_qc_round, qc_round) {
            // 1. must vote in monotonically increasing rounds
            // 2. must extend a smaller round
            false
        } else {
            // Extending qc from previous round or safe to extend due to tc
            Self::is_consecutive(block_round, qc_round) || self.safe_to_extend(block_round, qc_round, tc)
        }
    }

    fn safe_to_timeout(&self, round: Round, qc_round: Round, tc: TimeoutCertificate) -> bool {
        if qc_round < self.highest_qc_round || round <= cmp::max(self.highest_vote_round - 1, qc_round) {
            // respect highest qc round and don’t timeout in a past round
            false
        } else {
            // qc or tc must allow entering the round to timeout
            Self::is_consecutive(round, qc_round) || Self::is_consecutive(round, tc.round)
        }
    }

    fn commit_state_id_candidate(&self, block_round: Round, qc: QuorumCertificate) -> Option<()> {
        // find the committed id in case a qc is formed in the vote round
        if Self::is_consecutive(block_round, qc.vote_info.round) {
            self.ledger.pending_state(qc.vote_info.id)
        } else {
            None
        }
    }

    fn is_consecutive(block_round: Round, round: Round) -> bool {
        round + 1 == block_round
    }

    fn is_valid_signatures(signatures: &[Signature]) -> bool {
        // valid signatures call in the beginning of these functions checks
        // the well-formedness and signatures on all parameters provided
        // to construct the votes (using the public keys of other validators

        true
    }
}
