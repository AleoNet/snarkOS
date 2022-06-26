use std::collections::HashMap;

#[cfg(feature = "test")]
use tokio::sync::mpsc;

#[cfg(feature = "test")]
use crate::message::TestMessage;
use crate::{
    bft::Round,
    block_tree::{BlockTree, QuorumCertificate},
    message::{Timeout, TimeoutCertificate, TimeoutInfo},
    safety::Safety,
    F,
};

pub struct Pacemaker {
    // Initially zero
    pub current_round: Round,
    // Initially ⊥
    last_round_tc: Option<TimeoutCertificate>,
    // Timeouts per round
    pending_timeouts: HashMap<Round, HashMap<(), TimeoutInfo>>,

    // Used to send messages to other managers in tests.
    #[cfg(feature = "test")]
    outbound_sender: mpsc::Sender<TestMessage>,
}

impl Pacemaker {
    #[cfg(not(feature = "test"))]
    pub fn new() -> Self {
        // does `current_round` persist?

        todo!()
    }

    #[cfg(feature = "test")]
    pub fn new(
        // TODO: include the same arguments as the non-test version
        outbound_sender: mpsc::Sender<TestMessage>, // a clone of `common_msg_sender`
    ) -> Self {
        todo!()
    }

    pub fn get_round_timer(&self, round: Round) -> () {
        // FIXME: timer
        // round timer formula // For example, use 4 × ∆ or α + βcommit gap(r) if ∆ is unknown.

        todo!()
    }

    pub fn start_timer(&mut self, new_round: Round) {
        // FIXME: timer
        // stop_timer(current round)

        self.current_round = new_round;

        // start local timer for round current round for duration get round timer(current round)

        todo!()
    }

    pub fn local_timeout_round(&self, block_tree: &BlockTree, safety: &mut Safety) {
        // FIXME: what should this do
        // save_consensus_state()

        let high_qc = block_tree.high_qc.clone();
        // TODO: are the unwraps safe?
        let timeout_info = safety
            .make_timeout(self.current_round, high_qc, self.last_round_tc.clone().unwrap())
            .unwrap();
        let timeout_msg = Timeout {
            tmo_info: timeout_info,
            last_round_tc: self.last_round_tc.clone(),
            high_commit_qc: block_tree.high_commit_qc.clone(),
        };

        // TODO: broadcast timeout_msg

        #[cfg(feature = "test")]
        self.outbound_sender.blocking_send(TestMessage::new(todo!(), None)).unwrap();
    }

    pub fn process_remote_timeout(&mut self, tmo: Timeout, block_tree: &BlockTree, safety: &mut Safety) -> Option<TimeoutCertificate> {
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

            self.local_timeout_round(block_tree, safety) // Bracha timeout
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

    pub fn advance_round_qc(&mut self, qc: QuorumCertificate) -> bool {
        if qc.vote_info.round < self.current_round {
            return false;
        }

        self.last_round_tc = None;

        // FIXME: timer
        // start timer(qc.vote_info.round + 1)

        true
    }
}
