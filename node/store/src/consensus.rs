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

use crate::{BlockDB, ProgramDB, TransactionDB, TransitionDB};
use snarkvm::prelude::*;

/// An RocksDB consensus storage.
#[derive(Clone)]
pub struct ConsensusDB<N: Network> {
    /// The program store.
    program_store: ProgramStore<N, ProgramDB<N>>,
    /// The block store.
    block_store: BlockStore<N, BlockDB<N>>,
}

#[rustfmt::skip]
impl<N: Network> ConsensusStorage<N> for ConsensusDB<N> {
    type ProgramStorage = ProgramDB<N>;
    type BlockStorage = BlockDB<N>;
    type TransactionStorage = TransactionDB<N>;
    type TransitionStorage = TransitionDB<N>;

    /// Initializes the consensus storage.
    fn open(dev: Option<u16>) -> Result<Self> {
        // Initialize the program store.
        let program_store = ProgramStore::<N, ProgramDB<N>>::open(dev)?;
        // Initialize the block store.
        let block_store = BlockStore::<N, BlockDB<N>>::open(dev)?;
        // Return the consensus storage.
        Ok(Self {
            program_store,
            block_store,
        })
    }

    /// Returns the program store.
    fn program_store(&self) -> &ProgramStore<N, Self::ProgramStorage> {
        &self.program_store
    }

    /// Returns the block store.
    fn block_store(&self) -> &BlockStore<N, Self::BlockStorage> {
        &self.block_store
    }
}
