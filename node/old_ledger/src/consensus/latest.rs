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

use super::*;

impl<N: Network, C: ConsensusStorage<N>> Consensus<N, C> {
    /// Returns the latest state root.
    pub const fn latest_state_root(&self) -> &Field<N> {
        self.block_tree.root()
    }

    /// Returns the latest block.
    pub fn latest_block(&self) -> Result<Block<N>> {
        self.get_block(self.current_height)
    }

    /// Returns the latest block hash.
    pub const fn latest_hash(&self) -> N::BlockHash {
        self.current_hash
    }

    /// Returns the latest block height.
    pub const fn latest_height(&self) -> u32 {
        self.current_height
    }

    /// Returns the latest round number.
    pub const fn latest_round(&self) -> u64 {
        self.current_round
    }

    /// Returns the latest epoch number.
    pub fn latest_epoch_number(&self) -> u32 {
        self.current_height / N::NUM_BLOCKS_PER_EPOCH
    }

    /// Returns the latest epoch challenge.
    pub fn latest_epoch_challenge(&self) -> Result<EpochChallenge<N>> {
        // Get the epoch's starting height (multiple of `NUM_BLOCKS_PER_EPOCH`).
        let epoch_starting_height = self.current_height - self.current_height % N::NUM_BLOCKS_PER_EPOCH;
        ensure!(epoch_starting_height % N::NUM_BLOCKS_PER_EPOCH == 0, "Invalid epoch starting height");

        // Fetch the epoch block hash, defined as the 'previous block hash' from the starting block height.
        let epoch_block_hash = self.get_previous_hash(epoch_starting_height)?;

        EpochChallenge::new(self.latest_epoch_number(), epoch_block_hash, N::COINBASE_PUZZLE_DEGREE)
    }

    /// Returns the latest block header.
    pub fn latest_header(&self) -> Result<Header<N>> {
        self.get_header(self.current_height)
    }

    /// Returns the latest block coinbase target.
    pub fn latest_coinbase_target(&self) -> Result<u64> {
        Ok(self.latest_header()?.coinbase_target())
    }

    /// Returns the latest block proof target.
    pub fn latest_proof_target(&self) -> Result<u64> {
        Ok(self.latest_header()?.proof_target())
    }

    /// Returns the latest coinbase timestamp.
    pub fn latest_coinbase_timestamp(&self) -> Result<i64> {
        Ok(self.latest_header()?.last_coinbase_timestamp())
    }

    /// Returns the latest block timestamp.
    pub fn latest_timestamp(&self) -> Result<i64> {
        Ok(self.latest_header()?.timestamp())
    }

    /// Returns the latest block transactions.
    pub fn latest_transactions(&self) -> Result<Transactions<N>> {
        self.get_transactions(self.current_height)
    }
}
