use crate::{
    reference::{
        ledger::{Block, BlockHash},
        Round,
        Address,
        Signature,
    },
};

use anyhow::Result;
use std::{cmp, collections::HashMap, hash::Hash};

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
    pub signatures: Vec<Signature>,
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
    pub signature: Signature,
}

impl Propose {
    pub fn new(block: Block, last_round_tc: Option<TimeoutCertificate>, high_commit_qc: QuorumCertificate, signature: Signature) -> Self {
        Self {
            block,
            last_round_tc,
            high_commit_qc,
            signature,
        }
    }

    /// Returns the address of the leader who proposed this block.
    pub fn leader(&self) -> Result<Address> {
        self.signature.signer()
    }
}

#[derive(Clone, Debug, Hash)]
pub struct VoteInfo {
    // Id of block
    pub id: BlockHash,
    // round of block
    pub round: Round,
    // Id of parent
    pub parent_id: BlockHash,
    // round of parent
    pub parent_round: Round,
    // Speculated execution state
    pub exec_state_id: Option<()>,
}

// speculated new committed state to vote directly on
#[derive(Clone, Debug, Hash)]
pub struct LedgerCommitInfo {
    // ⊥ if no commit happens when this vote is aggregated to QC
    pub commit_state_id: Option<()>,
    // Hash of VoteMsg.vote info
    pub vote_info_hash: u64,
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

// QC is a VoteMsg with multiple signatures
#[derive(Clone, Debug)]
pub struct QuorumCertificate {
    pub vote_info: VoteInfo,
    pub ledger_commit_info: LedgerCommitInfo,
    // A quorum of signatures
    pub signatures: Vec<Signature>,
    // The validator that produced the qc
    author: (),
    author_signature: (),
}

impl QuorumCertificate {
    /// Returns the signatures.
    pub fn signatures(&self) -> &[Signature] {
        &self.signatures
    }
}

impl PartialEq for QuorumCertificate {
    fn eq(&self, other: &Self) -> bool {
        todo!()
    }
}

impl Eq for QuorumCertificate {}

impl Ord for QuorumCertificate {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        todo!()
    }
}

impl PartialOrd for QuorumCertificate {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

// A message dedicated to tests.
#[cfg(feature = "test")]
#[derive(Debug)]
pub struct TestMessage {
    // The actual message.
    pub message: Message,
    // The index of the validator to relay the message to; None is a broadcast.
    pub target: Option<usize>,
    // note: it should be possible to identify the source based on the message signature.
}

#[cfg(feature = "test")]
impl TestMessage {
    pub fn new(message: Message, target: Option<usize>) -> Self {
        Self { message, target }
    }
}
