#[cfg(feature = "test")]
use tokio::sync::mpsc;

#[cfg(feature = "test")]
use crate::message::TestMessage;
use crate::{
    block_tree::{BlockTree, QuorumCertificate},
    ledger::Ledger,
    message::{Message, Propose, Timeout, TimeoutCertificate, Vote},
    pacemaker::Pacemaker,
    safety::Safety,
};

use anyhow::Result;

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
pub struct Manager {
    block_tree: BlockTree,
    ledger: Ledger,
    mempool: Mempool,
    pacemaker: Pacemaker,
    safety: Safety,

    // Leader Selection //

    // The list of current validators
    validators: Vec<Address>,
    // A parameter for the leader reputation algorithm
    window_size: usize,
    // Between f and 2f , number of excluded authors of last committed blocks
    exclude_size: usize,
    // Map from round numbers to leaders elected due to the reputation scheme
    reputation_leaders: HashMap<Round, Address>,

    // Testing //

    // Used to send messages to other managers in tests.
    #[cfg(feature = "test")]
    outbound_sender: mpsc::Sender<TestMessage>,
}

impl Manager {
    #[cfg(not(feature = "test"))]
    pub fn new(/* TODO: pass the ledger here */) -> Self {
        Self {
            block_tree: BlockTree::new(),
            ledger: Ledger::new(),
            mempool: Mempool::new(),
            pacemaker: Pacemaker::new(),
            safety: Safety::new(),

            validators: vec![],
            window_size: 0,
            exclude_size: 0,
            reputation_leaders: HashMap::new(),
        }
    }

    #[cfg(feature = "test")]
    pub fn new(
        // TODO: include the same arguments as the non-test version
        outbound_sender: mpsc::Sender<TestMessage>, // a clone of `common_msg_sender`
    ) -> Self {
        Self {
            block_tree: BlockTree::new(),
            ledger: Ledger::new(),
            mempool: Mempool::new(),
            pacemaker: Pacemaker::new(outbound_sender.clone()),
            safety: Safety::new(),

            validators: vec![],
            window_size: 0,
            exclude_size: 0,
            reputation_leaders: HashMap::new(),

            outbound_sender,
        }
    }

    pub fn start_event_processing(&mut self, msg: Message) -> Result<()> {
        match msg {
            Message::LocalTimeout => self.pacemaker.local_timeout_round(&self.block_tree, &mut self.safety),
            Message::Propose(msg) => self.process_propose(msg),
            Message::Timeout(msg) => self.process_timeout(msg),
            Message::Vote(msg) => self.process_vote(msg),
        }
    }

    fn process_propose(&mut self, propose: Propose) -> Result<()> {
        self.process_qc(propose.block.qc.clone())?;
        self.process_qc(propose.high_commit_qc.clone())?;

        self.pacemaker.advance_round_tc(propose.last_round_tc.clone());

        // note: the whitepaper assigns to 'round' here
        let current_round = self.pacemaker.current_round;
        let leader = self.get_leader(current_round);

        // note: the whitepaper uses 'round' instead of 'current_round' here
        // note: the whitepaper uses 'p.sender' instead of 'p.signature' here
        if propose.block.round() != current_round || propose.leader()? != *leader || propose.block.leader() != *leader {
            return Ok(());
        }

        // note: the whitepaper passes the entire 'p' here instead of 'p.block'
        self.block_tree.execute_and_insert(propose.block.clone(), &mut self.ledger); // Adds a new speculative state to the Ledger

        // FIXME: the whitepaper doesn't consider when 'p.last_round_tc' is ⊥
        if let Some(vote_msg) = self
            .safety
            .make_vote(propose.block, propose.last_round_tc.unwrap(), &self.ledger, &self.block_tree)
        {
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

        self.pacemaker.advance_round_tc(timeout.last_round_tc.clone());

        if let Some(tc) = self.pacemaker.process_remote_timeout(timeout, &self.block_tree, &mut self.safety) {
            // FIXME: method not specified in the whitepaper again, and uses a different type now
            // self.pacemaker.advance_round(tc);
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
        // self.pacemaker.advance_round(qc.vote_info.round);
    }

    fn process_new_round_event(&self, last_tc: Option<TimeoutCertificate>) {
        // TODO: if <US> == self.election.get_leader(self.pacemaker.current_round) {
        // Leader code: generate proposal.
        let block = self
            .block_tree
            .generate_block(self.mempool.get_transactions(), self.pacemaker.current_round);
        let proposal_msg = Propose::new(block, last_tc, self.block_tree.high_commit_qc.clone(), todo!());

        // TODO: broadcast proposal_msg ProposalMsg〈b, last tc, Block-Tree.high commit qc〉

        #[cfg(feature = "test")]
        self.outbound_sender.blocking_send(TestMessage::new(todo!(), None)).unwrap();
        // }
    }
}

use crate::{bft::Round, Address};

use std::collections::HashMap;

// Leader selection
impl Manager {
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
        let current_round = self.pacemaker.current_round;

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
