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

#![forbid(unsafe_code)]

#[macro_use]
extern crate tracing;

mod contains;
mod find;
mod get;
mod iterators;

#[cfg(test)]
mod tests;

use snarkvm::{
    console::{
        account::{Address, GraphKey, PrivateKey, Signature, ViewKey},
        network::prelude::*,
        program::{Ciphertext, Identifier, Plaintext, ProgramID, Record, Value},
        types::{Field, Group},
    },
    synthesizer::{
        block::{Block, BlockTree, Header, Transaction, Transactions},
        coinbase_puzzle::{CoinbasePuzzle, CoinbaseSolution, EpochChallenge, PuzzleCommitment},
        program::Program,
        state_path::StatePath,
        store::{BlockStore, ConsensusStorage, ConsensusStore, TransactionStore, TransitionStore},
        vm::VM,
    },
};

use anyhow::Result;
use indexmap::IndexMap;
use std::borrow::Cow;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

type RecordMap<N> = IndexMap<Field<N>, Record<N, Plaintext<N>>>;

#[derive(Copy, Clone, Debug)]
pub enum RecordsFilter<N: Network> {
    /// Returns all records associated with the account.
    All,
    /// Returns only records associated with the account that are **spent** with the graph key.
    Spent,
    /// Returns only records associated with the account that are **not spent** with the graph key.
    Unspent,
    /// Returns all records associated with the account that are **spent** with the given private key.
    SlowSpent(PrivateKey<N>),
    /// Returns all records associated with the account that are **not spent** with the given private key.
    SlowUnspent(PrivateKey<N>),
}

#[derive(Clone)]
pub struct Ledger<N: Network, C: ConsensusStorage<N>> {
    /// The VM state.
    pub vm: VM<N, C>,
    /// The coinbase puzzle.
    coinbase_puzzle: CoinbasePuzzle<N>,
    /// The current block hash.
    current_hash: N::BlockHash,
    /// The current block height.
    current_height: u32,
    /// The current round number.
    current_round: u64,
    /// The current block tree.
    block_tree: BlockTree<N>,
    /// The block store.
    blocks: BlockStore<N, C::BlockStorage>,
    /// The transaction store.
    transactions: TransactionStore<N, C::TransactionStorage>,
    /// The transition store.
    transitions: TransitionStore<N, C::TransitionStorage>,
    // /// The mapping of program IDs to their global state.
    // states: MemoryMap<ProgramID<N>, IndexMap<Identifier<N>, Plaintext<N>>>,
}

impl<N: Network, C: ConsensusStorage<N>> Ledger<N, C> {
    /// Loads the ledger from storage.
    pub fn load(genesis: Option<Block<N>>, dev: Option<u16>) -> Result<Self> {
        // Retrieve the genesis hash.
        let genesis_hash = match genesis {
            Some(ref genesis) => genesis.hash(),
            None => Block::<N>::from_bytes_le(N::genesis_bytes())?.hash(),
        };

        // Initialize the consensus store.
        let store = ConsensusStore::<N, C>::open(dev)?;
        // Initialize a new VM.
        let vm = VM::from(store)?;
        // Initialize the ledger.
        let ledger = Self::from(vm, genesis)?;

        // Ensure the ledger contains the correct genesis block.
        match ledger.contains_block_hash(&genesis_hash)? {
            true => Ok(ledger),
            false => bail!("Incorrect genesis block (run 'snarkos clean' and try again)"),
        }
    }

    /// Initializes the ledger from storage, with an optional genesis block.
    pub fn from(vm: VM<N, C>, genesis: Option<Block<N>>) -> Result<Self> {
        // Load the coinbase puzzle.
        let coinbase_puzzle = CoinbasePuzzle::<N>::load()?;

        // Initialize the ledger.
        let mut ledger = Self {
            coinbase_puzzle,
            current_hash: Default::default(),
            current_height: 0,
            current_round: 0,
            block_tree: N::merkle_tree_bhp(&[])?,
            blocks: vm.block_store().clone(),
            transactions: vm.transaction_store().clone(),
            transitions: vm.transition_store().clone(),
            vm,
        };

        // If the block store is empty, initialize the genesis block.
        if ledger.blocks.heights().max().is_none() {
            // Load the genesis block.
            let genesis = match genesis {
                Some(genesis) => genesis,
                None => Block::<N>::from_bytes_le(N::genesis_bytes())?,
            };
            // Add the genesis block.
            ledger.add_next_block(&genesis)?;
        }

        // Retrieve the latest height.
        let latest_height =
            *ledger.blocks.heights().max().ok_or_else(|| anyhow!("Failed to load blocks from the ledger"))?;
        // Fetch the latest block.
        let block = ledger
            .get_block(latest_height)
            .map_err(|_| anyhow!("Failed to load block {latest_height} from the ledger"))?;

        // Set the current hash, height, and round.
        ledger.current_hash = block.hash();
        ledger.current_height = block.height();
        ledger.current_round = block.round();

        // TODO (howardwu): Improve the performance here by using iterators.
        // Generate the block tree.
        let hashes: Vec<_> =
            (0..=latest_height).map(|height| ledger.get_hash(height).map(|hash| hash.to_bits_le())).try_collect()?;
        ledger.block_tree = N::merkle_tree_bhp(&hashes)?;

        // Safety check the existence of every block.
        cfg_into_iter!((0..=latest_height)).try_for_each(|height| {
            ledger.get_block(height)?;
            Ok::<_, Error>(())
        })?;

        Ok(ledger)
    }

    /// Returns the VM.
    pub fn vm(&self) -> &VM<N, C> {
        &self.vm
    }

    /// Returns the coinbase puzzle.
    pub const fn coinbase_puzzle(&self) -> &CoinbasePuzzle<N> {
        &self.coinbase_puzzle
    }

    /// Returns the block tree.
    pub const fn block_tree(&self) -> &BlockTree<N> {
        &self.block_tree
    }

    /// Returns a state path for the given commitment.
    pub fn to_state_path(&self, commitment: &Field<N>) -> Result<StatePath<N>> {
        StatePath::new_commitment(&self.block_tree, &self.blocks, commitment)
    }

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
        // Get the epoch starting height (a multiple of `NUM_BLOCKS_PER_EPOCH`).
        let epoch_starting_height = self.current_height - self.current_height % N::NUM_BLOCKS_PER_EPOCH;
        ensure!(epoch_starting_height % N::NUM_BLOCKS_PER_EPOCH == 0, "Invalid epoch starting height");
        // Retrieve the epoch block hash, defined as the 'previous block hash' from the epoch starting height.
        let epoch_block_hash = self.get_previous_hash(epoch_starting_height)?;
        // Construct the epoch challenge.
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

    /// Adds the given block as the next block in the chain.
    pub fn add_next_block(&mut self, block: &Block<N>) -> Result<()> {
        /* ATOMIC CODE SECTION */

        // Add the block to the ledger. This code section executes atomically.
        {
            let mut ledger = self.clone();

            // Update the blocks.
            ledger.current_hash = block.hash();
            ledger.current_height = block.height();
            ledger.current_round = block.round();
            ledger.block_tree.append(&[block.hash().to_bits_le()])?;
            ledger.blocks.insert(*ledger.block_tree.root(), block)?;

            // Update the VM.
            for transaction in block.transactions().values() {
                ledger.vm.finalize(transaction)?;
            }

            *self = Self {
                vm: ledger.vm,
                coinbase_puzzle: ledger.coinbase_puzzle,
                current_hash: ledger.current_hash,
                current_height: ledger.current_height,
                current_round: ledger.current_round,
                block_tree: ledger.block_tree,
                blocks: ledger.blocks,
                transactions: ledger.transactions,
                transitions: ledger.transitions,
            };
        }

        Ok(())
    }

    /// Returns the unspent records.
    pub fn find_unspent_records(&self, view_key: &ViewKey<N>) -> Result<RecordMap<N>> {
        Ok(self
            .find_records(view_key, RecordsFilter::Unspent)?
            .filter(|(_, record)| !record.gates().is_zero())
            .collect::<IndexMap<_, _>>())
    }

    /// Creates a transfer transaction.
    pub fn create_transfer(&self, private_key: &PrivateKey<N>, to: &Address<N>, amount: u64) -> Result<Transaction<N>> {
        // Fetch the unspent records.
        let records = self.find_unspent_records(&ViewKey::try_from(private_key)?)?;
        ensure!(!records.len().is_zero(), "The Aleo account has no records to spend.");

        // Initialize an RNG.
        let rng = &mut rand::thread_rng();

        // Create a new transaction.
        Transaction::execute(
            &self.vm,
            private_key,
            &ProgramID::from_str("credits.aleo")?,
            Identifier::from_str("transfer")?,
            &[
                Value::Record(records.values().next().unwrap().clone()),
                Value::from_str(&format!("{to}"))?,
                Value::from_str(&format!("{amount}u64"))?,
            ],
            None,
            rng,
        )
    }
}
