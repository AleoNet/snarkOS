use crate::{
    bft::Round,
    block::Block,
    block_tree::{LedgerCommitInfo, QuorumCertificate, VoteInfo},
};

#[derive(Clone, Debug)]
pub enum Message {
    LocalTimeout,
    Propose(Propose),
    Timeout(Timeout),
    Vote(Vote),
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
pub struct Timeout {
    // TimeoutInfo for some round with a high qc
    pub tmo_info: TimeoutInfo,
    // TC for tmo info.round − 1 if tmo info.high_qc.round != tmo info.round − 1, else ⊥
    pub last_round_tc: Option<TimeoutCertificate>,
    // QC to synchronize on committed blocks
    pub high_commit_qc: QuorumCertificate,
}

#[derive(Clone, Debug)]
pub struct Propose {
    pub block: Block,
    // TC for block.round − 1 if block.qc.vote info.round != block.round − 1, else ⊥
    pub last_round_tc: Option<TimeoutCertificate>,
    // QC to synchronize on committed blocks
    pub high_commit_qc: QuorumCertificate,
    pub signature: (),
}

impl Propose {
    pub fn new(block: Block, last_round_tc: Option<TimeoutCertificate>, high_commit_qc: QuorumCertificate) -> Self {
        Self {
            block,
            last_round_tc,
            high_commit_qc,
            signature: (),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Vote {
    // A VoteInfo record
    vote_info: VoteInfo,
    // Speculated ledger info
    pub ledger_commit_info: LedgerCommitInfo,
    // QC to synchronize on committed blocks
    pub high_commit_qc: QuorumCertificate,
    // Added automatically when constructed
    sender: (),
    // Signed automatically when constructed
    signature: (),
}

impl Vote {
    pub fn new(vote_info: VoteInfo, ledger_commit_info: LedgerCommitInfo, high_commit_qc: QuorumCertificate, author: ()) -> Self {
        Self {
            vote_info,
            ledger_commit_info,
            high_commit_qc,
            sender: (),
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
