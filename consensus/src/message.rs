use crate::{
    block_tree::{Block, QuorumCertificate, VoteMsg},
    Round,
};

#[derive(Clone, Debug)]
pub enum Message {
    LocalTimeout,
    Proposal(ProposalMsg),
    Vote(VoteMsg),
    Timeout(TimeoutMsg),
}

#[derive(Clone, Debug)]
pub struct TimeoutInfo {
    pub round: Round,
    pub high_qc: QuorumCertificate,
    // Added automatically when constructed
    pub sender: (),
    // Signed automatically when constructed
    signature: (),
}

impl TimeoutInfo {
    pub fn new(round: Round, high_qc: QuorumCertificate, author: ()) -> Self {
        Self {
            round,
            high_qc,
            sender: (),
            signature: (),
        }
    }
}

#[derive(Clone, Debug)]
pub struct TimeoutCertificate {
    // All timeout messages that form TC have the same round
    pub round: Round,
    // A vector of 2f + 1 high qc round numbers of timeout messages that form TC
    pub tmo_high_qc_rounds: Vec<Round>,
    // A vector of 2f + 1 validator signatures on (round, respective high qc round)
    pub signatures: Vec<()>,
}

#[derive(Clone, Debug)]
pub struct TimeoutMsg {
    // TimeoutInfo for some round with a high qc
    pub tmo_info: TimeoutInfo,
    // TC for tmo info.round − 1 if tmo info.high_qc.round != tmo info.round − 1, else ⊥
    pub last_round_tc: Option<TimeoutCertificate>,
    // QC to synchronize on committed blocks
    pub high_commit_qc: QuorumCertificate,
}

#[derive(Clone, Debug)]
pub struct ProposalMsg {
    pub block: Block,
    // TC for block.round − 1 if block.qc.vote info.round != block.round − 1, else ⊥
    pub last_round_tc: Option<TimeoutCertificate>,
    // QC to synchronize on committed blocks
    pub high_commit_qc: QuorumCertificate,
    pub signature: (),
}

impl ProposalMsg {
    pub fn new(block: Block, last_round_tc: Option<TimeoutCertificate>, high_commit_qc: QuorumCertificate) -> Self {
        Self {
            block,
            last_round_tc,
            high_commit_qc,
            signature: (),
        }
    }
}

// A message dedicated to tests.
#[cfg(feature = "test")]
#[derive(Debug)]
pub struct TestMessage {
    // The actual message.
    pub message: Message,
    // The index of the manager to relay the message to; None is a broadcast.
    pub target: Option<usize>,
    // note: it should be possible to identify the source based on the message signature.
}

#[cfg(feature = "test")]
impl TestMessage {
    pub fn new(message: Message, target: Option<usize>) -> Self {
        Self { message, target }
    }
}
