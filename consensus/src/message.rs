use crate::{
    block_tree::{Block, QuorumCertificate, VoteMsg},
    QcRound,
    Round,
};

pub enum Message {
    LocalTimeout,
    Proposal(ProposalMsg),
    Vote(VoteMsg),
    Timeout(TimeoutMsg),
}

#[derive(Clone)]
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

#[derive(Clone)]
pub struct TimeoutCertificate {
    // All timeout messages that form TC have the same round
    pub round: Round,
    // A vector of 2f + 1 high qc round numbers of timeout messages that form TC
    pub tmo_high_qc_rounds: Vec<QcRound>,
    // A vector of 2f + 1 validator signatures on (round, respective high qc round)
    pub signatures: Vec<()>,
}

pub struct TimeoutMsg {
    // TimeoutInfo for some round with a high qc
    pub tmo_info: TimeoutInfo,
    // TC for tmo info.round − 1 if tmo info.high_qc.round != tmo info.round − 1, else ⊥
    pub last_round_tc: Option<TimeoutCertificate>,
    // QC to synchronize on committed blocks
    pub high_commit_qc: QuorumCertificate,
}

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
