#[cfg(feature = "test")]
use tokio::sync::mpsc;

#[cfg(feature = "test")]
use crate::message::TestMessage;
use crate::{
    block_tree::{BlockTree, QuorumCertificate},
    election::Election,
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
    election: Election,
    ledger: Ledger,
    mempool: Mempool,
    pacemaker: Pacemaker,
    safety: Safety,

    // Used to send messages to other managers in tests.
    #[cfg(feature = "test")]
    outbound_sender: mpsc::Sender<TestMessage>,
}

impl Manager {
    #[cfg(not(feature = "test"))]
    pub fn new(/* TODO: pass the ledger here */) -> Self {
        Self {
            block_tree: BlockTree::new(),
            election: Election::new(),
            ledger: Ledger::new(),
            mempool: Mempool::new(),
            pacemaker: Pacemaker::new(),
            safety: Safety::new(),
        }
    }

    #[cfg(feature = "test")]
    pub fn new(
        // TODO: include the same arguments as the non-test version
        outbound_sender: mpsc::Sender<TestMessage>, // a clone of `common_msg_sender`
    ) -> Self {
        Self {
            block_tree: BlockTree::new(),
            election: Election::new(),
            ledger: Ledger::new(),
            mempool: Mempool::new(),
            pacemaker: Pacemaker::new(outbound_sender.clone()),
            safety: Safety::new(),
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
        let leader = self.election.get_leader(current_round);

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
            let leader = self.election.get_leader(current_round + 1);

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
        self.election.update_leaders(qc, &self.pacemaker, &self.ledger)
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
