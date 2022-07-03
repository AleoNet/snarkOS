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

mod header;
pub use header::*;

use snarkvm::prelude::Network;

#[derive(Clone)]
pub struct Block<N: Network> {
    /// The header of the block.
    header: BlockHeader<N>,
    // // QC for parent block
    // pub qc: crate::message::QuorumCertificate,
}

impl<N: Network> Block<N> {
    /// Returns the block header.
    pub const fn header(&self) -> &BlockHeader<N> {
        &self.header
    }
}
