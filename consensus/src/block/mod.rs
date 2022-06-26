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

use crate::Address;

// FIXME: integrate with the snarkVM BlockHash OR height
pub type BlockHash = u128;

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct Header {
    /// The round that produced this block.
    round: u64,
    /// The height of this block.
    height: u32,
}

impl Header {
    /// Returns the round number of the block.
    pub const fn round(&self) -> u64 {
        self.round
    }

    /// Returns the height of the block.
    pub const fn height(&self) -> u32 {
        self.height
    }
}

#[derive(Clone)]
pub struct Block {
    // A unique digest of author, round, payload, qc.vote info.id and qc.signatures
    hash: BlockHash,
    /// The header of the block.
    header: Header,
    // // QC for parent block
    // pub qc: crate::message::QuorumCertificate,
}
