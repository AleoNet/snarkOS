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
        program::{Ciphertext, Identifier, Plaintext, ProgramID, Record, StatePath, Value},
        types::{Field, Group},
    },
    synthesizer::{
        block::{Block, Header, Transaction, Transactions},
        coinbase_puzzle::{CoinbaseSolution, EpochChallenge, PuzzleCommitment},
        program::Program,
        store::{ConsensusStorage, ConsensusStore},
        vm::VM,
    },
};

use aleo_std::prelude::{finish, lap, timer};
use anyhow::Result;
use core::ops::Range;
use indexmap::IndexMap;
use parking_lot::RwLock;
use rand::{prelude::IteratorRandom, rngs::OsRng};
use std::{borrow::Cow, sync::Arc};

#[cfg(feature = "parallel")]
use rayon::prelude::*;

pub type RecordMap<N> = IndexMap<Field<N>, Record<N, Plaintext<N>>>;

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
    vm: VM<N, C>,
    /// The genesis block.
    genesis: Block<N>,
    /// The current block.
    current_block: Arc<RwLock<Block<N>>>,
    /// The current epoch challenge.
    current_epoch_challenge: Arc<RwLock<Option<EpochChallenge<N>>>>,
}

impl<N: Network, C: ConsensusStorage<N>> Ledger<N, C> {
    /// Loads the ledger from storage.
    pub fn load(genesis: Block<N>, dev: Option<u16>) -> Result<Self> {
        let timer = timer!("Ledger::load");

        // Retrieve the genesis hash.
        let genesis_hash = genesis.hash();
        // Initialize the ledger.
        let ledger = Self::load_unchecked(genesis, dev)?;

        // Ensure the ledger contains the correct genesis block.
        if !ledger.contains_block_hash(&genesis_hash)? {
            bail!("Incorrect genesis block (run 'snarkos clean' and try again)")
        }

        // Retrieve the latest height.
        let latest_height =
            *ledger.vm.block_store().heights().max().ok_or_else(|| anyhow!("Failed to load blocks from the ledger"))?;

        // Safety check the existence of `NUM_BLOCKS` random blocks.
        const NUM_BLOCKS: usize = 1000;
        let block_heights: Vec<u32> = (0..=latest_height)
            .choose_multiple(&mut OsRng::default(), core::cmp::min(NUM_BLOCKS, latest_height as usize));
        cfg_into_iter!(block_heights).try_for_each(|height| {
            ledger.get_block(height)?;
            Ok::<_, Error>(())
        })?;
        lap!(timer, "Check existence of {NUM_BLOCKS} random blocks");

        finish!(timer);
        Ok(ledger)
    }

    /// Loads the ledger from storage, without performing integrity checks.
    pub fn load_unchecked(genesis: Block<N>, dev: Option<u16>) -> Result<Self> {
        let timer = timer!("Ledger::load_unchecked");

        // Initialize the consensus store.
        let store = match ConsensusStore::<N, C>::open(dev) {
            Ok(store) => store,
            _ => bail!("Failed to load ledger (run 'snarkos clean' and try again)"),
        };
        lap!(timer, "Load consensus store");

        // Initialize a new VM.
        let vm = VM::from(store)?;
        lap!(timer, "Initialize a new VM");

        // Initialize the ledger.
        let mut ledger = Self {
            vm,
            genesis: genesis.clone(),
            current_block: Arc::new(RwLock::new(genesis.clone())),
            current_epoch_challenge: Default::default(),
        };

        // If the block store is empty, initialize the genesis block.
        if ledger.vm.block_store().heights().max().is_none() {
            // Add the genesis block.
            ledger.add_next_block(&genesis)?;
        }
        lap!(timer, "Initialize genesis");

        // Retrieve the latest height.
        let latest_height =
            *ledger.vm.block_store().heights().max().ok_or_else(|| anyhow!("Failed to load blocks from the ledger"))?;
        // Fetch the latest block.
        let block = ledger
            .get_block(latest_height)
            .map_err(|_| anyhow!("Failed to load block {latest_height} from the ledger"))?;

        // Set the current block.
        ledger.current_block = Arc::new(RwLock::new(block));
        // Set the current epoch challenge.
        ledger.current_epoch_challenge = Arc::new(RwLock::new(Some(ledger.get_epoch_challenge(latest_height)?)));
        lap!(timer, "Initialize ledger");

        finish!(timer);
        Ok(ledger)
    }

    /// Returns the VM.
    pub fn vm(&self) -> &VM<N, C> {
        &self.vm
    }

    /// Returns the latest state root.
    pub fn latest_state_root(&self) -> Field<N> {
        *self.vm.block_store().current_state_root()
    }

    /// Returns the latest block.
    pub fn latest_block(&self) -> Block<N> {
        self.current_block.read().clone()
    }

    /// Returns the latest round number.
    pub fn latest_round(&self) -> u64 {
        self.current_block.read().round()
    }

    /// Returns the latest block height.
    pub fn latest_height(&self) -> u32 {
        self.current_block.read().height()
    }

    /// Returns the latest block hash.
    pub fn latest_hash(&self) -> N::BlockHash {
        self.current_block.read().hash()
    }

    /// Returns the latest block header.
    pub fn latest_header(&self) -> Header<N> {
        *self.current_block.read().header()
    }

    /// Returns the latest block coinbase accumulator point.
    pub fn latest_coinbase_accumulator_point(&self) -> Field<N> {
        self.current_block.read().header().coinbase_accumulator_point()
    }

    /// Returns the latest block coinbase target.
    pub fn latest_coinbase_target(&self) -> u64 {
        self.current_block.read().coinbase_target()
    }

    /// Returns the latest block proof target.
    pub fn latest_proof_target(&self) -> u64 {
        self.current_block.read().proof_target()
    }

    /// Returns the last coinbase target.
    pub fn last_coinbase_target(&self) -> u64 {
        self.current_block.read().last_coinbase_target()
    }

    /// Returns the last coinbase timestamp.
    pub fn last_coinbase_timestamp(&self) -> i64 {
        self.current_block.read().last_coinbase_timestamp()
    }

    /// Returns the latest block timestamp.
    pub fn latest_timestamp(&self) -> i64 {
        self.current_block.read().timestamp()
    }

    /// Returns the latest block transactions.
    pub fn latest_transactions(&self) -> Transactions<N> {
        self.current_block.read().transactions().clone()
    }

    /// Returns the latest epoch number.
    pub fn latest_epoch_number(&self) -> u32 {
        self.current_block.read().height() / N::NUM_BLOCKS_PER_EPOCH
    }

    /// Returns the latest epoch challenge.
    pub fn latest_epoch_challenge(&self) -> Result<EpochChallenge<N>> {
        match self.current_epoch_challenge.read().as_ref() {
            Some(challenge) => Ok(challenge.clone()),
            None => self.get_epoch_challenge(self.latest_height()),
        }
    }

    /// Adds the given block as the next block in the chain.
    pub fn add_next_block(&self, block: &Block<N>) -> Result<()> {
        // Acquire the write lock on the current block.
        let mut current_block = self.current_block.write();
        // Update the VM.
        self.vm.add_next_block(block)?;
        // Update the current block.
        *current_block = block.clone();
        // Drop the write lock on the current block.
        drop(current_block);

        // If the block is the start of a new epoch, or the epoch challenge has not been set, update the current epoch challenge.
        if block.height() % N::NUM_BLOCKS_PER_EPOCH == 0 || self.current_epoch_challenge.read().is_none() {
            // Update the current epoch challenge.
            self.current_epoch_challenge.write().clone_from(&self.get_epoch_challenge(block.height()).ok());
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
    pub fn create_transfer(&self, private_key: &PrivateKey<N>, to: Address<N>, amount: u64) -> Result<Transaction<N>> {
        // Fetch the unspent records.
        let records = self.find_unspent_records(&ViewKey::try_from(private_key)?)?;
        ensure!(!records.len().is_zero(), "The Aleo account has no records to spend.");

        // Initialize an RNG.
        let rng = &mut rand::thread_rng();

        // Prepare the inputs.
        let inputs = [
            Value::Record(records.values().next().unwrap().clone()),
            Value::from_str(&format!("{to}"))?,
            Value::from_str(&format!("{amount}u64"))?,
        ];

        // Create a new transaction.
        Transaction::execute(
            &self.vm,
            private_key,
            ProgramID::from_str("credits.aleo")?,
            Identifier::from_str("transfer")?,
            inputs.iter(),
            None,
            None,
            rng,
        )
    }
}
