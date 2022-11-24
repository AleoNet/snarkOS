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

impl<N: Network, C: ConsensusStorage<N>> Ledger<N, C> {
    /// Returns the state root that contains the given `block height`.
    pub fn get_state_root(&self, block_height: u32) -> Result<Option<N::StateRoot>> {
        self.vm.block_store().get_state_root(block_height)
    }

    /// Returns a state path for the given commitment.
    pub fn get_state_path_for_commitment(&self, commitment: &Field<N>) -> Result<StatePath<N>> {
        self.vm.block_store().get_state_path_for_commitment(commitment)
    }

    /// Returns the epoch challenge for the given block height.
    pub fn get_epoch_challenge(&self, block_height: u32) -> Result<EpochChallenge<N>> {
        // Compute the epoch number from the current block height.
        let epoch_number = block_height / N::NUM_BLOCKS_PER_EPOCH;
        // Compute the epoch starting height (a multiple of `NUM_BLOCKS_PER_EPOCH`).
        let epoch_starting_height = epoch_number * N::NUM_BLOCKS_PER_EPOCH;
        // Retrieve the epoch block hash, defined as the 'previous block hash' from the epoch starting height.
        let epoch_block_hash = self.get_previous_hash(epoch_starting_height)?;
        // Construct the epoch challenge.
        EpochChallenge::new(epoch_number, epoch_block_hash, N::COINBASE_PUZZLE_DEGREE)
    }

    /// Returns the block for the given block height.
    pub fn get_block(&self, height: u32) -> Result<Block<N>> {
        // If the height is 0, return the genesis block.
        if height == 0 {
            return Ok(self.genesis.clone());
        }
        // Retrieve the block hash.
        let block_hash = match self.vm.block_store().get_block_hash(height)? {
            Some(block_hash) => block_hash,
            None => bail!("Block {height} does not exist in storage"),
        };
        // Retrieve the block.
        match self.vm.block_store().get_block(&block_hash)? {
            Some(block) => Ok(block),
            None => bail!("Block {height} ('{block_hash}') does not exist in storage"),
        }
    }

    /// Returns the blocks in the given block range.
    /// The range is inclusive of the start and exclusive of the end.
    pub fn get_blocks(&self, heights: Range<u32>) -> Result<Vec<Block<N>>> {
        cfg_into_iter!(heights).map(|height| self.get_block(height)).collect()
    }

    /// Returns the block for the given block hash.
    pub fn get_block_by_hash(&self, block_hash: &N::BlockHash) -> Result<Block<N>> {
        // Retrieve the block.
        match self.vm.block_store().get_block(block_hash)? {
            Some(block) => Ok(block),
            None => bail!("Block '{block_hash}' does not exist in storage"),
        }
    }

    /// Returns the block height for the given block hash.
    pub fn get_height(&self, block_hash: &N::BlockHash) -> Result<u32> {
        match self.vm.block_store().get_block_height(block_hash)? {
            Some(height) => Ok(height),
            None => bail!("Missing block height for block '{block_hash}'"),
        }
    }

    /// Returns the block hash for the given block height.
    pub fn get_hash(&self, height: u32) -> Result<N::BlockHash> {
        // If the height is 0, return the genesis block hash.
        if height == 0 {
            return Ok(self.genesis.hash());
        }
        match self.vm.block_store().get_block_hash(height)? {
            Some(block_hash) => Ok(block_hash),
            None => bail!("Missing block hash for block {height}"),
        }
    }

    /// Returns the previous block hash for the given block height.
    pub fn get_previous_hash(&self, height: u32) -> Result<N::BlockHash> {
        // If the height is 0, return the default block hash.
        if height == 0 {
            return Ok(N::BlockHash::default());
        }
        match self.vm.block_store().get_previous_block_hash(height)? {
            Some(previous_hash) => Ok(previous_hash),
            None => bail!("Missing previous block hash for block {height}"),
        }
    }

    /// Returns the block header for the given block height.
    pub fn get_header(&self, height: u32) -> Result<Header<N>> {
        // If the height is 0, return the genesis block header.
        if height == 0 {
            return Ok(*self.genesis.header());
        }
        // Retrieve the block hash.
        let block_hash = match self.vm.block_store().get_block_hash(height)? {
            Some(block_hash) => block_hash,
            None => bail!("Block {height} does not exist in storage"),
        };
        // Retrieve the block header.
        match self.vm.block_store().get_block_header(&block_hash)? {
            Some(header) => Ok(header),
            None => bail!("Missing block header for block {height}"),
        }
    }

    /// Returns the block transactions for the given block height.
    pub fn get_transactions(&self, height: u32) -> Result<Transactions<N>> {
        // If the height is 0, return the genesis block transactions.
        if height == 0 {
            return Ok(self.genesis.transactions().clone());
        }
        // Retrieve the block hash.
        let block_hash = match self.vm.block_store().get_block_hash(height)? {
            Some(block_hash) => block_hash,
            None => bail!("Block {height} does not exist in storage"),
        };
        // Retrieve the block transaction.
        match self.vm.block_store().get_block_transactions(&block_hash)? {
            Some(transactions) => Ok(transactions),
            None => bail!("Missing block transactions for block {height}"),
        }
    }

    /// Returns the transaction for the given transaction ID.
    pub fn get_transaction(&self, transaction_id: N::TransactionID) -> Result<Transaction<N>> {
        // Retrieve the transaction.
        match self.vm.transaction_store().get_transaction(&transaction_id)? {
            Some(transaction) => Ok(transaction),
            None => bail!("Missing transaction for ID {transaction_id}"),
        }
    }

    /// Returns the program for the given program ID.
    pub fn get_program(&self, program_id: ProgramID<N>) -> Result<Program<N>> {
        match self.vm.transaction_store().get_program(&program_id)? {
            Some(program) => Ok(program),
            None => bail!("Missing program for ID {program_id}"),
        }
    }

    /// Returns the block coinbase solution for the given block height.
    pub fn get_coinbase(&self, height: u32) -> Result<Option<CoinbaseSolution<N>>> {
        // If the height is 0, return the genesis block coinbase.
        if height == 0 {
            return Ok(self.genesis.coinbase().cloned());
        }
        // Retrieve the block hash.
        let block_hash = match self.vm.block_store().get_block_hash(height)? {
            Some(block_hash) => block_hash,
            None => bail!("Block {height} does not exist in storage"),
        };
        // Retrieve the block coinbase solution.
        self.vm.block_store().get_block_coinbase(&block_hash)
    }

    /// Returns the block signature for the given block height.
    pub fn get_signature(&self, height: u32) -> Result<Signature<N>> {
        // If the height is 0, return the genesis block signature.
        if height == 0 {
            return Ok(*self.genesis.signature());
        }
        // Retrieve the block hash.
        let block_hash = match self.vm.block_store().get_block_hash(height)? {
            Some(block_hash) => block_hash,
            None => bail!("Block {height} does not exist in storage"),
        };
        // Retrieve the block signature.
        match self.vm.block_store().get_block_signature(&block_hash)? {
            Some(signature) => Ok(signature),
            None => bail!("Missing signature for block {height}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::test_helpers::CurrentLedger;
    use snarkvm::console::network::Testnet3;

    type CurrentNetwork = Testnet3;

    #[test]
    fn test_get_block() {
        // Load the genesis block.
        let genesis = Block::from_bytes_le(CurrentNetwork::genesis_bytes()).unwrap();

        // Initialize a new ledger.
        let ledger = CurrentLedger::load(genesis.clone(), None).unwrap();
        // Retrieve the genesis block.
        let candidate = ledger.get_block(0).unwrap();
        // Ensure the genesis block matches.
        assert_eq!(genesis, candidate);
    }
}
