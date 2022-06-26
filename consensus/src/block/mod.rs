// Copyright (C) 2019-2022 Aleo Systems Inc.
// This file is part of the snarkOS library.

// The snarkOS library is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// The snarkOS library is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with the snarkOS library. If not, see <https://www.gnu.org/licenses/>.

use crate::{round::Round, Address};

/// This value defines the height of a block, which is always less than or equal to the round number.
pub type Height = u32;

// FIXME: integrate with the snarkVM BlockHash OR height
pub type BlockHash = u64;

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct Header {}

#[derive(Clone, Debug)]
pub struct Block {
    // A unique digest of author, round, payload, qc.vote info.id and qc.signatures
    pub hash: BlockHash,

    // The leader of the round, may not be the same as qc.author after view-change
    pub leader: Address,
    // The round that generated this proposal
    pub round: Round,
    // Proposed transaction(s)
    pub payload: Vec<()>,
    // // QC for parent block
    // pub qc: crate::message::QuorumCertificate,
}

impl Block {
    /// Returns the round number of the block.
    pub const fn round(&self) -> &Round {
        &self.round
    }

    /// Returns the leader of the round.
    pub const fn leader(&self) -> Address {
        self.leader
    }
}
