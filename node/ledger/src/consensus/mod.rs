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

mod helpers;
pub use helpers::*;

mod contains;
mod find;
mod get;
mod iterators;
mod latest;

use snarkvm::{
    console::{
        account::{Address, GraphKey, PrivateKey, Signature, ViewKey},
        network::prelude::*,
        program::{Ciphertext, Identifier, Plaintext, ProgramID, Record},
        types::{Field, Group},
    },
    synthesizer::{
        block::{Block, BlockTree, Header, Metadata, Origin, Transaction, Transactions},
        coinbase_puzzle::{CoinbasePuzzle, CoinbaseSolution, EpochChallenge, ProverSolution},
        program::Program,
        state_path::StatePath,
        store::{BlockStore, ConsensusMemory, ConsensusStorage, ConsensusStore, TransactionStore, TransitionStore},
        vm::VM,
    },
};

use anyhow::Result;
use indexmap::{IndexMap, IndexSet};
use std::borrow::Cow;
use time::OffsetDateTime;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

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
pub struct Consensus<N: Network, C: ConsensusStorage<N>> {
    /// The VM state.
    vm: VM<N, C>,
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
    /// The validators.
    // TODO (howardwu): Update this to retrieve from a validators store.
    validators: IndexMap<Address<N>, ()>,
    /// The memory pool of unconfirmed transactions.
    memory_pool: IndexMap<N::TransactionID, Transaction<N>>,

    /// The coinbase puzzle.
    coinbase_puzzle: CoinbasePuzzle<N>,
    /// The memory pool of proposed coinbase puzzle solutions for the current epoch.
    coinbase_memory_pool: IndexSet<ProverSolution<N>>,
    // /// The mapping of program IDs to their global state.
    // states: MemoryMap<ProgramID<N>, IndexMap<Identifier<N>, Plaintext<N>>>,
}

impl<N: Network> Consensus<N, ConsensusMemory<N>> {
    /// Initializes a new instance of `Ledger` with the genesis block.
    pub fn new(dev: Option<u16>) -> Result<Self> {
        // Load the genesis block.
        let genesis = Block::<N>::from_bytes_le(N::genesis_bytes())?;
        // Initialize the ledger.
        Self::new_with_genesis(&genesis, genesis.signature().to_address(), dev)
    }
}

impl<N: Network, C: ConsensusStorage<N>> Consensus<N, C> {
    /// Initializes a new instance of `Consensus` with the given genesis block.
    pub fn new_with_genesis(genesis: &Block<N>, address: Address<N>, dev: Option<u16>) -> Result<Self> {
        // Initialize the consensus store.
        let store = ConsensusStore::<N, C>::open(dev)?;
        // Initialize a new VM.
        let vm = VM::from(store.clone())?;

        // Ensure that a genesis block doesn't already exist in the block store.
        if vm.block_store().contains_block_height(0)? {
            bail!("Genesis block already exists in the ledger.");
        }

        // Load the coinbase puzzle.
        let coinbase_puzzle = CoinbasePuzzle::<N>::load()?;

        // Initialize the consensus module.
        let mut consensus = Self {
            vm,
            current_hash: Default::default(),
            current_height: 0,
            current_round: 0,
            block_tree: N::merkle_tree_bhp(&[])?,
            blocks: store.block_store().clone(),
            transactions: store.transaction_store().clone(),
            transitions: store.transition_store().clone(),
            // TODO (howardwu): Update this to retrieve from a validators store.
            validators: [(address, ())].into_iter().collect(),
            memory_pool: Default::default(),
            coinbase_puzzle,
            coinbase_memory_pool: Default::default(),
        };

        // Add the genesis block.
        consensus.add_next_block(genesis)?;

        // Return the consensus.
        Ok(consensus)
    }

    /// Initializes the consensus module from storage.
    pub fn open(dev: Option<u16>) -> Result<Self> {
        // Initialize the consensus store.
        let store = ConsensusStore::<N, C>::open(dev)?;
        // Return the consensus.
        Self::from(store)
    }

    /// Initializes the consensus module from storage.
    pub fn from(store: ConsensusStore<N, C>) -> Result<Self> {
        // Initialize a new VM.
        let vm = VM::from(store.clone())?;

        // Load the coinbase puzzle.
        let coinbase_puzzle = CoinbasePuzzle::<N>::load()?;

        // Initialize the consensus.
        let mut consensus = Self {
            vm,
            current_hash: Default::default(),
            current_height: 0,
            current_round: 0,
            block_tree: N::merkle_tree_bhp(&[])?,
            blocks: store.block_store().clone(),
            transactions: store.transaction_store().clone(),
            transitions: store.transition_store().clone(),
            // TODO (howardwu): Update this to retrieve from a validators store.
            validators: Default::default(),
            memory_pool: Default::default(),
            coinbase_puzzle,
            coinbase_memory_pool: Default::default(),
        };

        // Fetch the latest height.
        let latest_height = match consensus.blocks.heights().max() {
            Some(height) => *height,
            // If there are no previous hashes, add the genesis block.
            None => {
                // Load the genesis block.
                let genesis = Block::<N>::from_bytes_le(N::genesis_bytes())?;
                // Add the genesis block.
                consensus.add_next_block(&genesis)?;
                // Return the genesis height.
                genesis.height()
            }
        };

        // Add the initial validator.
        let genesis_block = consensus.get_block(0)?;
        consensus.add_validator(genesis_block.signature().to_address())?;

        // Fetch the latest block.
        let block = consensus.get_block(latest_height)?;

        // Set the current hash, height, and round.
        consensus.current_hash = block.hash();
        consensus.current_height = block.height();
        consensus.current_round = block.round();

        // TODO (howardwu): Improve the performance here by using iterators.
        // Generate the block tree.
        let hashes: Vec<_> =
            (1..=latest_height).map(|height| consensus.get_hash(height).map(|hash| hash.to_bits_le())).try_collect()?;
        consensus.block_tree.append(&hashes)?;

        // Safety check the existence of every block.
        #[cfg(feature = "parallel")]
        let heights_iter = (0..=latest_height).into_par_iter();
        #[cfg(not(feature = "parallel"))]
        let mut heights_iter = (0..=latest_height).into_iter();
        heights_iter.try_for_each(|height| {
            consensus.get_block(height)?;
            Ok::<_, Error>(())
        })?;

        Ok(consensus)
    }

    /// Returns the VM.
    pub fn vm(&self) -> &VM<N, C> {
        &self.vm
    }

    /// Appends the given transaction to the memory pool.
    pub fn add_to_memory_pool(&mut self, transaction: Transaction<N>) -> Result<()> {
        // Ensure the transaction does not already exist.
        if self.memory_pool.contains_key(&transaction.id()) {
            bail!("Transaction '{}' already exists in the memory pool.", transaction.id());
        }

        // Check that the transaction is well formed and unique.
        self.check_transaction(&transaction)?;

        // Insert the transaction to the memory pool.
        self.memory_pool.insert(transaction.id(), transaction);
        Ok(())
    }

    /// Appends the given prover solution to the coinbase memory pool.
    pub fn add_to_coinbase_memory_pool(&mut self, prover_solution: ProverSolution<N>) -> Result<()> {
        // Ensure that prover solutions are not accepted after 10 years.
        if self.latest_height() > anchor_block_height(N::ANCHOR_TIME, 10) {
            bail!("Coinbase proofs are no longer accepted after year 10.");
        }

        // Compute the current epoch challenge.
        let epoch_challenge = self.latest_epoch_challenge()?;
        // Retrieve the current proof target.
        let proof_target = self.latest_proof_target()?;

        // Ensure that the prover solution is valid for the given epoch.
        if !prover_solution.verify(self.coinbase_puzzle.coinbase_verifying_key()?, &epoch_challenge, proof_target)? {
            bail!("Prover puzzle '{}' is invalid for the given epoch.", prover_solution.commitment().0);
        }

        // Insert the prover solution to the memory pool.
        if !self.coinbase_memory_pool.insert(prover_solution) {
            bail!("Prover puzzle '{}' already exists in the memory pool.", prover_solution.commitment().0);
        }

        Ok(())
    }

    /// Returns a candidate for the next block in the ledger.
    pub fn propose_next_block<R: Rng + CryptoRng>(&self, private_key: &PrivateKey<N>, rng: &mut R) -> Result<Block<N>> {
        // Construct the transactions for the block.
        let transactions = {
            // TODO (raychu86): Add more sophisticated logic for transaction selection.

            // Add the transactions from the memory pool that do not have input collisions.
            let mut transcations = Vec::new();
            let mut input_ids = Vec::new();

            'outer: for transaction in self.memory_pool.values() {
                for input_id in transaction.input_ids() {
                    if input_ids.contains(&input_id) {
                        continue 'outer;
                    }
                }

                transcations.push(transaction);
                input_ids.extend(transaction.input_ids());
            }

            transcations.into_iter().collect::<Transactions<N>>()
        };

        // Select the prover solutions from the memory pool.
        let prover_solutions =
            self.coinbase_memory_pool.iter().take(N::MAX_PROVER_SOLUTIONS).cloned().collect::<Vec<_>>();

        // Compute the total cumulative target of the prover puzzle solutions as a u128.
        let cumulative_prover_target: u128 = prover_solutions.iter().try_fold(0u128, |cumulative, solution| {
            cumulative.checked_add(solution.to_target()? as u128).ok_or_else(|| anyhow!("Cumulative target overflowed"))
        })?;

        // TODO (howardwu): Add `has_coinbase` to function arguments.
        // Construct the coinbase proof.
        let anchor_height_at_year_10 = anchor_block_height(N::ANCHOR_TIME, 10);
        let (coinbase_proof, coinbase_accumulator_point) = if self.latest_height() > anchor_height_at_year_10
            || cumulative_prover_target < self.latest_coinbase_target()? as u128
        {
            (None, Field::<N>::zero())
        } else {
            let epoch_challenge = self.latest_epoch_challenge()?;
            let coinbase_proof = self.coinbase_puzzle.accumulate_unchecked(&epoch_challenge, &prover_solutions)?;
            let coinbase_accumulator_point = coinbase_proof.to_accumulator_point()?;

            (Some(coinbase_proof), coinbase_accumulator_point)
        };

        // Fetch the latest block and state root.
        let block = self.latest_block()?;
        let state_root = self.latest_state_root();

        // Fetch the new round state.
        let timestamp = OffsetDateTime::now_utc().unix_timestamp();
        let next_height = self.latest_height().saturating_add(1);
        let round = block.round().saturating_add(1);

        // TODO (raychu86): Pay the provers. Currently we do not pay the provers with the `credits.aleo` program
        //  and instead, will track prover leaderboards via the `coinbase_proof` in each block.
        {
            // Calculate the coinbase reward.
            let coinbase_reward =
                coinbase_reward(block.timestamp(), timestamp, next_height, N::STARTING_SUPPLY, N::ANCHOR_TIME)?;

            // Calculate the rewards for the individual provers.
            let mut prover_rewards: Vec<(Address<N>, u64)> = Vec::new();
            for prover_solution in prover_solutions {
                // Prover compensation is defined as:
                //   1/2 * coinbase_reward * (prover_target / cumulative_prover_target)
                //   = (coinbase_reward * prover_target) / (2 * cumulative_prover_target)

                // Compute the numerator.
                let numerator = (coinbase_reward as u128)
                    .checked_mul(prover_solution.to_target()? as u128)
                    .ok_or_else(|| anyhow!("Prover reward numerator overflowed"))?;

                // Compute the denominator.
                let denominator = (cumulative_prover_target as u128)
                    .checked_mul(2)
                    .ok_or_else(|| anyhow!("Prover reward denominator overflowed"))?;

                // Compute the prover reward.
                let prover_reward = u64::try_from(
                    numerator.checked_div(denominator).ok_or_else(|| anyhow!("Prover reward overflowed"))?,
                )?;

                prover_rewards.push((prover_solution.address(), prover_reward));
            }
        }

        // Construct the new coinbase target.
        let coinbase_target = coinbase_target(
            self.latest_coinbase_target()?,
            block.timestamp(),
            timestamp,
            N::ANCHOR_TIME,
            N::NUM_BLOCKS_PER_EPOCH,
        )?;

        // Construct the new proof target.
        let proof_target = proof_target(coinbase_target);

        // Construct the metadata.
        let metadata = Metadata::new(N::ID, round, next_height, coinbase_target, proof_target, timestamp)?;

        // Construct the header.
        let header = Header::from(*state_root, transactions.to_root()?, coinbase_accumulator_point, metadata)?;

        // Construct the new block.
        Block::new(private_key, block.hash(), header, transactions, coinbase_proof, rng)
    }

    /// Checks the given block is valid next block.
    pub fn check_next_block(&self, block: &Block<N>) -> Result<()> {
        // Ensure the previous block hash is correct.
        if self.current_hash != block.previous_hash() {
            bail!("The given block has an incorrect previous block hash")
        }

        // Ensure the block hash does not already exist.
        if self.contains_block_hash(&block.hash())? {
            bail!("Block hash '{}' already exists in the ledger", block.hash())
        }

        // Ensure the next block height is correct.
        if self.latest_height() > 0 && self.latest_height() + 1 != block.height() {
            bail!("The given block has an incorrect block height")
        }

        // Ensure the block height does not already exist.
        if self.contains_block_height(block.height())? {
            bail!("Block height '{}' already exists in the ledger", block.height())
        }

        // TODO (raychu86): Ensure the next round number includes timeouts.
        // Ensure the next round is correct.
        if self.latest_round() > 0 && self.latest_round() + 1 /*+ block.number_of_timeouts()*/ != block.round() {
            bail!("The given block has an incorrect round number")
        }

        // TODO (raychu86): Ensure the next block timestamp is the median of proposed blocks.
        // Ensure the next block timestamp is after the current block timestamp.
        if block.height() > 0 && block.header().timestamp() <= self.latest_block()?.header().timestamp() {
            bail!("The given block timestamp is before the current timestamp")
        }

        for transaction_id in block.transaction_ids() {
            // Ensure the transaction in the block do not already exist.
            if self.contains_transaction_id(transaction_id)? {
                bail!("Transaction '{transaction_id}' already exists in the ledger")
            }
        }

        /* Input */

        // Ensure that the origin are valid.
        for origin in block.origins() {
            match origin {
                // Check that the commitment exists in the ledger.
                Origin::Commitment(commitment) => {
                    if !self.contains_commitment(commitment)? {
                        bail!("The given transaction references a non-existent commitment {}", &commitment)
                    }
                }
                // TODO (raychu86): Ensure that the state root exists in the ledger.
                // Check that the state root is an existing state root.
                Origin::StateRoot(_state_root) => {
                    bail!("State roots are currently not supported (yet)")
                }
            }
        }

        // Ensure the ledger does not already contain a given serial numbers.
        for serial_number in block.serial_numbers() {
            if self.contains_serial_number(serial_number)? {
                bail!("Serial number '{serial_number}' already exists in the ledger")
            }
        }

        /* Output */

        // Ensure the ledger does not already contain a given commitments.
        for commitment in block.commitments() {
            if self.contains_commitment(commitment)? {
                bail!("Commitment '{commitment}' already exists in the ledger")
            }
        }

        // Ensure the ledger does not already contain a given nonces.
        for nonce in block.nonces() {
            if self.contains_nonce(nonce)? {
                bail!("Nonce '{nonce}' already exists in the ledger")
            }
        }

        /* Metadata */

        // Ensure the ledger does not already contain a given transition public keys.
        for tpk in block.transition_public_keys() {
            if self.contains_tpk(tpk)? {
                bail!("Transition public key '{tpk}' already exists in the ledger")
            }
        }

        /* Block Header */

        // If the block is the genesis block, check that it is valid.
        if block.height() == 0 && !block.is_genesis() {
            bail!("Invalid genesis block");
        }

        // Ensure the block header is valid.
        if !block.header().is_valid() {
            bail!("Invalid block header: {:?}", block.header());
        }

        /* Block Hash */

        // Compute the Merkle root of the block header.
        let header_root = match block.header().to_root() {
            Ok(root) => root,
            Err(error) => bail!("Failed to compute the Merkle root of the block header: {error}"),
        };

        // Check the block hash.
        match N::hash_bhp1024(&[block.previous_hash().to_bits_le(), header_root.to_bits_le()].concat()) {
            Ok(candidate_hash) => {
                // Ensure the block hash matches the one in the block.
                if candidate_hash != *block.hash() {
                    bail!("Block {} ({}) has an incorrect block hash.", block.height(), block.hash());
                }
            }
            Err(error) => {
                bail!("Unable to compute block hash for block {} ({}): {error}", block.height(), block.hash())
            }
        };

        /* Signature */

        // Ensure the block is signed by an authorized validator.
        let signer = block.signature().to_address();
        if !self.validators.contains_key(&signer) {
            let validator = self.validators.iter().next().unwrap().0;
            eprintln!("{} {} {} {}", *validator, signer, *validator == signer, self.validators.contains_key(&signer));
            bail!("Block {} ({}) is signed by an unauthorized validator ({})", block.height(), block.hash(), signer);
        }

        // Check the signature.
        if !block.signature().verify(&signer, &[*block.hash()]) {
            bail!("Invalid signature for block {} ({})", block.height(), block.hash());
        }

        /* Transactions */

        // Compute the transactions root.
        match block.transactions().to_root() {
            // Ensure the transactions root matches the one in the block header.
            Ok(root) => {
                if root != block.header().transactions_root() {
                    bail!(
                        "Block {} ({}) has an incorrect transactions root: expected {}",
                        block.height(),
                        block.hash(),
                        block.header().transactions_root()
                    );
                }
            }
            Err(error) => bail!("Failed to compute the Merkle root of the block transactions: {error}"),
        };

        // Ensure the transactions list is not empty.
        if block.transactions().is_empty() {
            bail!("Cannot validate an empty transactions list");
        }

        // Ensure the number of transactions is within the allowed range.
        if block.transactions().len() > Transactions::<N>::MAX_TRANSACTIONS {
            bail!("Cannot validate a block with more than {} transactions", Transactions::<N>::MAX_TRANSACTIONS);
        }

        // Ensure each transaction is well-formed and unique.
        #[cfg(feature = "parallel")]
        let transactions_iter = block.transactions().par_iter();
        #[cfg(not(feature = "parallel"))]
        let mut transactions_iter = block.transactions().iter();
        transactions_iter.try_for_each(|(_, transaction)| {
            self.check_transaction(transaction)
                .map_err(|e| anyhow!("Invalid transaction found in the transactions list: {e}"))
        })?;

        /* Coinbase Proof */

        // Ensure the coinbase proof is valid, if it exists.
        if let Some(coinbase_proof) = block.coinbase_proof() {
            // Ensure coinbase proofs are not accepted after the anchor block height at year 10.
            if block.height() > anchor_block_height(N::ANCHOR_TIME, 10) {
                bail!("Coinbase proofs are no longer accepted after the anchor block height at year 10.");
            }
            // Ensure the coinbase accumulator point matches in the block header.
            if block.header().coinbase_accumulator_point() != coinbase_proof.to_accumulator_point()? {
                bail!("Coinbase accumulator point does not match the coinbase proof.");
            }
            // Ensure the coinbase proof is valid.
            if !self.coinbase_puzzle.verify(
                coinbase_proof,
                &self.latest_epoch_challenge()?,
                self.latest_coinbase_target()?,
                self.latest_proof_target()?,
            )? {
                bail!("Invalid coinbase proof: {:?}", coinbase_proof);
            }
        } else {
            // Ensure that the block header does not contain a coinbase accumulator point.
            if block.header().coinbase_accumulator_point() != Field::<N>::zero() {
                bail!("Coinbase accumulator point should be zero as there is no coinbase proof in the block.");
            }
        }

        /* Fees */

        // Prepare the block height, credits program ID, and genesis function name.
        let height = block.height();
        let credits_program_id = ProgramID::from_str("credits.aleo")?;
        let credits_genesis = Identifier::from_str("genesis")?;

        // Ensure the fee is correct for each transition.
        for transition in block.transitions() {
            if height > 0 {
                // Ensure the genesis function is not called.
                if *transition.program_id() == credits_program_id && *transition.function_name() == credits_genesis {
                    bail!("The genesis function cannot be called.");
                }
                // Ensure the transition fee is not negative.
                if transition.fee().is_negative() {
                    bail!("The transition fee cannot be negative.");
                }
            }
        }

        Ok(())
    }

    /// Adds the given block as the next block in the chain.
    pub fn add_next_block(&mut self, block: &Block<N>) -> Result<()> {
        // Ensure the given block is a valid next block.
        self.check_next_block(block)?;

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

            // Clear the memory pool of the transactions that are now invalid.
            for (transaction_id, transaction) in self.memory_pool() {
                if ledger.check_transaction(transaction).is_err() {
                    ledger.memory_pool.remove(transaction_id);
                }
            }

            // Clear the coinbase memory pool of the coinbase proofs if a new epoch has started.
            if block.epoch_number() > self.latest_epoch_number() {
                ledger.coinbase_memory_pool.clear();
            }

            *self = Self {
                current_hash: ledger.current_hash,
                current_height: ledger.current_height,
                current_round: ledger.current_round,
                block_tree: ledger.block_tree,
                blocks: ledger.blocks,
                transactions: ledger.transactions,
                transitions: ledger.transitions,
                validators: ledger.validators,
                vm: ledger.vm,
                memory_pool: ledger.memory_pool,
                coinbase_puzzle: ledger.coinbase_puzzle,
                coinbase_memory_pool: ledger.coinbase_memory_pool,
            };
        }

        Ok(())
    }

    /// Adds a given address to the validator set.
    pub fn add_validator(&mut self, address: Address<N>) -> Result<()> {
        if self.validators.insert(address, ()).is_some() {
            bail!("'{address}' is already in the validator set.")
        } else {
            Ok(())
        }
    }

    /// Removes a given address from the validator set.
    pub fn remove_validator(&mut self, address: Address<N>) -> Result<()> {
        if self.validators.remove(&address).is_none() {
            bail!("'{address}' is not in the validator set.")
        } else {
            Ok(())
        }
    }

    /// Returns the block tree.
    pub const fn block_tree(&self) -> &BlockTree<N> {
        &self.block_tree
    }

    /// Returns the validator set.
    pub const fn validators(&self) -> &IndexMap<Address<N>, ()> {
        &self.validators
    }

    /// Returns the memory pool.
    pub const fn memory_pool(&self) -> &IndexMap<N::TransactionID, Transaction<N>> {
        &self.memory_pool
    }

    /// Returns the coinbase puzzle.
    pub const fn coinbase_puzzle(&self) -> &CoinbasePuzzle<N> {
        &self.coinbase_puzzle
    }

    /// Returns the coinbase memory pool.
    pub const fn coinbase_memory_pool(&self) -> &IndexSet<ProverSolution<N>> {
        &self.coinbase_memory_pool
    }

    /// Returns a state path for the given commitment.
    pub fn to_state_path(&self, commitment: &Field<N>) -> Result<StatePath<N>> {
        StatePath::new_commitment(&self.block_tree, &self.blocks, commitment)
    }

    /// Checks the given transaction is well formed and unique.
    pub fn check_transaction(&self, transaction: &Transaction<N>) -> Result<()> {
        let transaction_id = transaction.id();

        // Ensure the transaction is valid.
        if !self.vm.verify(transaction) {
            bail!("Transaction '{transaction_id}' is invalid")
        }

        // Ensure the ledger does not already contain the given transaction ID.
        if self.contains_transaction_id(&transaction_id)? {
            bail!("Transaction '{transaction_id}' already exists in the ledger")
        }

        /* Input */

        // Ensure the ledger does not already contain the given input ID.
        for input_id in transaction.input_ids() {
            if self.contains_input_id(input_id)? {
                bail!("Input ID '{input_id}' already exists in the ledger")
            }
        }

        // Ensure the ledger does not already contain a given serial numbers.
        for serial_number in transaction.serial_numbers() {
            if self.contains_serial_number(serial_number)? {
                bail!("Serial number '{serial_number}' already exists in the ledger")
            }
        }

        // Ensure the ledger does not already contain a given tag.
        for tag in transaction.tags() {
            if self.contains_tag(tag)? {
                bail!("Tag '{tag}' already exists in the ledger")
            }
        }

        // Ensure that the origin are valid.
        for origin in transaction.origins() {
            match origin {
                // Check that the commitment exists in the ledger.
                Origin::Commitment(commitment) => {
                    if !self.contains_commitment(commitment)? {
                        bail!("The given transaction references a non-existent commitment {}", &commitment)
                    }
                }
                // TODO (raychu86): Ensure that the state root exists in the ledger.
                // Check that the state root is an existing state root.
                Origin::StateRoot(_state_root) => {
                    bail!("State roots are currently not supported (yet)")
                }
            }
        }

        /* Output */

        // Ensure the ledger does not already contain the given output ID.
        for output_id in transaction.output_ids() {
            if self.contains_output_id(output_id)? {
                bail!("Output ID '{output_id}' already exists in the ledger")
            }
        }

        // Ensure the ledger does not already contain a given commitments.
        for commitment in transaction.commitments() {
            if self.contains_commitment(commitment)? {
                bail!("Commitment '{commitment}' already exists in the ledger")
            }
        }

        // Ensure the ledger does not already contain a given nonces.
        for nonce in transaction.nonces() {
            if self.contains_nonce(nonce)? {
                bail!("Nonce '{nonce}' already exists in the ledger")
            }
        }

        /* Program */

        // Ensure that the ledger does not already contain the given program ID.
        if let Transaction::Deploy(_, deployment, _) = &transaction {
            let program_id = deployment.program_id();
            if self.contains_program_id(program_id)? {
                bail!("Program ID '{program_id}' already exists in the ledger")
            }
        }

        /* Metadata */

        // Ensure the ledger does not already contain a given transition public keys.
        for tpk in transaction.transition_public_keys() {
            if self.contains_tpk(tpk)? {
                bail!("Transition public key '{tpk}' already exists in the ledger")
            }
        }

        // Ensure the ledger does not already contain a given transition commitment.
        for tcm in transaction.transition_commitments() {
            if self.contains_tcm(tcm)? {
                bail!("Transition commitment '{tcm}' already exists in the ledger")
            }
        }

        Ok(())
    }
}

#[cfg(test)]
pub(crate) mod test_helpers {
    use super::*;
    use snarkvm::{
        console::{account::PrivateKey, network::Testnet3, program::Value},
        prelude::TestRng,
        synthesizer::Block,
    };

    use once_cell::sync::OnceCell;

    type CurrentNetwork = Testnet3;
    pub(crate) type CurrentConsensus = Consensus<CurrentNetwork, ConsensusMemory<CurrentNetwork>>;

    pub(crate) fn sample_vm() -> VM<CurrentNetwork, ConsensusMemory<CurrentNetwork>> {
        VM::from(ConsensusStore::open(None).unwrap()).unwrap()
    }

    pub(crate) fn sample_genesis_private_key(rng: &mut TestRng) -> PrivateKey<CurrentNetwork> {
        static INSTANCE: OnceCell<PrivateKey<CurrentNetwork>> = OnceCell::new();
        *INSTANCE.get_or_init(|| {
            // Initialize a new caller.
            PrivateKey::<CurrentNetwork>::new(rng).unwrap()
        })
    }

    #[allow(dead_code)]
    pub(crate) fn sample_genesis_block(rng: &mut TestRng) -> Block<CurrentNetwork> {
        static INSTANCE: OnceCell<Block<CurrentNetwork>> = OnceCell::new();
        INSTANCE
            .get_or_init(|| {
                // Initialize the VM.
                let vm = crate::consensus::test_helpers::sample_vm();
                // Initialize a new caller.
                let caller_private_key = PrivateKey::<CurrentNetwork>::new(rng).unwrap();
                // Return the block.
                Block::genesis(&vm, &caller_private_key, rng).unwrap()
            })
            .clone()
    }

    pub(crate) fn sample_genesis_block_with_private_key(
        rng: &mut TestRng,
        private_key: PrivateKey<CurrentNetwork>,
    ) -> Block<CurrentNetwork> {
        static INSTANCE: OnceCell<Block<CurrentNetwork>> = OnceCell::new();
        INSTANCE
            .get_or_init(|| {
                // Initialize the VM.
                let vm = crate::consensus::test_helpers::sample_vm();
                // Return the block.
                Block::genesis(&vm, &private_key, rng).unwrap()
            })
            .clone()
    }

    pub(crate) fn sample_genesis_ledger(rng: &mut TestRng) -> CurrentConsensus {
        // Sample the genesis private key.
        let private_key = sample_genesis_private_key(rng);
        // Sample the genesis block.
        let genesis = sample_genesis_block_with_private_key(rng, private_key);

        // Initialize the ledger with the genesis block and the associated private key.
        let address = Address::try_from(&private_key).unwrap();
        let ledger = CurrentConsensus::new_with_genesis(&genesis, address, None).unwrap();
        assert_eq!(0, ledger.latest_height());
        assert_eq!(genesis.hash(), ledger.latest_hash());
        assert_eq!(genesis.round(), ledger.latest_round());
        assert_eq!(genesis, ledger.get_block(0).unwrap());

        ledger
    }

    pub(crate) fn sample_program() -> Program<CurrentNetwork> {
        static INSTANCE: OnceCell<Program<CurrentNetwork>> = OnceCell::new();
        INSTANCE
            .get_or_init(|| {
                // Initialize a new program.
                Program::<CurrentNetwork>::from_str(
                    r"
program testing.aleo;

interface message:
    amount as u128;

record token:
    owner as address.private;
    gates as u64.private;
    amount as u64.private;

function compute:
    input r0 as message.private;
    input r1 as message.public;
    input r2 as message.private;
    input r3 as token.record;
    add r0.amount r1.amount into r4;
    cast r3.owner r3.gates r3.amount into r5 as token.record;
    output r4 as u128.public;
    output r5 as token.record;",
                )
                .unwrap()
            })
            .clone()
    }

    pub(crate) fn sample_deployment_transaction(rng: &mut TestRng) -> Transaction<CurrentNetwork> {
        static INSTANCE: OnceCell<Transaction<CurrentNetwork>> = OnceCell::new();
        INSTANCE
            .get_or_init(|| {
                // Initialize the program.
                let program = sample_program();

                // Initialize a new caller.
                let caller_private_key = crate::consensus::test_helpers::sample_genesis_private_key(rng);
                let caller_view_key = ViewKey::try_from(&caller_private_key).unwrap();

                // Initialize the ledger.
                let ledger = crate::consensus::test_helpers::sample_genesis_ledger(rng);

                // Fetch the unspent records.
                let records = ledger
                    .find_records(&caller_view_key, RecordsFilter::SlowUnspent(caller_private_key))
                    .unwrap()
                    .filter(|(_, record)| !record.gates().is_zero())
                    .collect::<indexmap::IndexMap<_, _>>();
                trace!("Unspent Records:\n{:#?}", records);

                // Prepare the additional fee.
                let credits = records.values().next().unwrap().clone();
                let additional_fee = (credits, 10);

                // Initialize the VM.
                let vm = sample_vm();
                // Deploy.
                let transaction = Transaction::deploy(&vm, &caller_private_key, &program, additional_fee, rng).unwrap();
                // Verify.
                assert!(vm.verify(&transaction));
                // Return the transaction.
                transaction
            })
            .clone()
    }

    pub(crate) fn sample_execution_transaction(rng: &mut TestRng) -> Transaction<CurrentNetwork> {
        static INSTANCE: OnceCell<Transaction<CurrentNetwork>> = OnceCell::new();
        INSTANCE
            .get_or_init(|| {
                // Initialize a new caller.
                let caller_private_key = crate::consensus::test_helpers::sample_genesis_private_key(rng);
                let caller_view_key = ViewKey::try_from(&caller_private_key).unwrap();
                let address = Address::try_from(&caller_private_key).unwrap();

                // Initialize the ledger.
                let ledger = crate::consensus::test_helpers::sample_genesis_ledger(rng);

                // Fetch the unspent records.
                let records = ledger
                    .find_records(&caller_view_key, RecordsFilter::SlowUnspent(caller_private_key))
                    .unwrap()
                    .filter(|(_, record)| !record.gates().is_zero())
                    .collect::<indexmap::IndexMap<_, _>>();
                trace!("Unspent Records:\n{:#?}", records);
                // Select a record to spend.
                let record = records.values().next().unwrap().clone();

                // Initialize the VM.
                let vm = sample_vm();

                // Authorize.
                let authorization = vm
                    .authorize(
                        &caller_private_key,
                        &ProgramID::from_str("credits.aleo").unwrap(),
                        Identifier::from_str("transfer").unwrap(),
                        &[
                            Value::<CurrentNetwork>::Record(record),
                            Value::<CurrentNetwork>::from_str(&address.to_string()).unwrap(),
                            Value::<CurrentNetwork>::from_str("1u64").unwrap(),
                        ],
                        rng,
                    )
                    .unwrap();
                assert_eq!(authorization.len(), 1);

                // Execute.
                let transaction = Transaction::execute_authorization(&vm, authorization, rng).unwrap();
                // Verify.
                assert!(vm.verify(&transaction));
                // Return the transaction.
                transaction
            })
            .clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consensus::test_helpers::CurrentConsensus;
    use snarkvm::{
        console::{network::Testnet3, program::Value},
        prelude::TestRng,
    };

    use tracing_test::traced_test;

    type CurrentNetwork = Testnet3;

    #[test]
    fn test_validators() {
        // Initialize an RNG.
        let rng = &mut TestRng::default();

        // Sample the private key, view key, and address.
        let private_key = PrivateKey::<CurrentNetwork>::new(rng).unwrap();
        let view_key = ViewKey::try_from(private_key).unwrap();
        let address = Address::try_from(&view_key).unwrap();

        // Initialize the VM.
        let vm = crate::consensus::test_helpers::sample_vm();

        // Create a genesis block.
        let genesis = Block::genesis(&vm, &private_key, rng).unwrap();

        // Initialize the validators.
        let validators: IndexMap<Address<_>, ()> = [(address, ())].into_iter().collect();

        // Ensure the block is signed by an authorized validator.
        let signer = genesis.signature().to_address();
        if !validators.contains_key(&signer) {
            let validator = validators.iter().next().unwrap().0;
            eprintln!("{} {} {} {}", *validator, signer, *validator == signer, validators.contains_key(&signer));
            eprintln!(
                "Block {} ({}) is signed by an unauthorized validator ({})",
                genesis.height(),
                genesis.hash(),
                signer
            );
        }
        assert!(validators.contains_key(&signer));
    }

    #[test]
    fn test_new() {
        // Load the genesis block.
        let genesis = Block::<CurrentNetwork>::from_bytes_le(CurrentNetwork::genesis_bytes()).unwrap();

        // Initialize consensus with the genesis block.
        let ledger = CurrentConsensus::new(None).unwrap();
        assert_eq!(ledger.latest_hash(), genesis.hash());
        assert_eq!(ledger.latest_height(), genesis.height());
        assert_eq!(ledger.latest_round(), genesis.round());
        assert_eq!(ledger.latest_block().unwrap(), genesis);
    }

    #[test]
    fn test_from() {
        // Load the genesis block.
        let genesis = Block::<CurrentNetwork>::from_bytes_le(CurrentNetwork::genesis_bytes()).unwrap();
        // Initialize the address.
        let address =
            Address::<CurrentNetwork>::from_str("aleo1q6qstg8q8shwqf5m6q5fcenuwsdqsvp4hhsgfnx5chzjm3secyzqt9mxm8")
                .unwrap();

        // Initialize consensus without the genesis block.
        let ledger = CurrentConsensus::from(ConsensusStore::<_, ConsensusMemory<_>>::open(None).unwrap()).unwrap();
        assert_eq!(ledger.latest_hash(), genesis.hash());
        assert_eq!(ledger.latest_height(), genesis.height());
        assert_eq!(ledger.latest_round(), genesis.round());
        assert_eq!(ledger.latest_block().unwrap(), genesis);

        // Initialize the ledger with the genesis block.
        let ledger = CurrentConsensus::new_with_genesis(&genesis, address, None).unwrap();
        assert_eq!(ledger.latest_hash(), genesis.hash());
        assert_eq!(ledger.latest_height(), genesis.height());
        assert_eq!(ledger.latest_round(), genesis.round());
        assert_eq!(ledger.latest_block().unwrap(), genesis);
    }

    #[test]
    fn test_state_path() {
        // Initialize the ledger with the genesis block.
        let ledger = CurrentConsensus::new(None).unwrap();
        // Retrieve the genesis block.
        let genesis = ledger.get_block(0).unwrap();

        // Construct the state path.
        let commitments = genesis.transactions().commitments().collect::<Vec<_>>();
        let commitment = commitments[0];

        let _state_path = ledger.to_state_path(commitment).unwrap();
    }

    #[test]
    #[traced_test]
    fn test_ledger_deploy() {
        let rng = &mut TestRng::default();

        // Sample the genesis private key.
        let private_key = crate::consensus::test_helpers::sample_genesis_private_key(rng);
        // Sample the genesis ledger.
        let mut ledger = test_helpers::sample_genesis_ledger(rng);

        // Add a transaction to the memory pool.
        let transaction = crate::consensus::test_helpers::sample_deployment_transaction(rng);
        ledger.add_to_memory_pool(transaction.clone()).unwrap();

        // Propose the next block.
        let next_block = ledger.propose_next_block(&private_key, rng).unwrap();

        // Construct a next block.
        ledger.add_next_block(&next_block).unwrap();
        assert_eq!(ledger.latest_height(), 1);
        assert_eq!(ledger.latest_hash(), next_block.hash());
        assert!(ledger.contains_transaction_id(&transaction.id()).unwrap());
        assert!(transaction.input_ids().count() > 0);
        assert!(ledger.contains_input_id(transaction.input_ids().next().unwrap()).unwrap());

        // Ensure that the VM can't re-deploy the same program.
        assert!(ledger.vm.finalize(&transaction).is_err());
        // Ensure that the ledger deems the same transaction invalid.
        assert!(ledger.check_transaction(&transaction).is_err());
        // Ensure that the ledger cannot add the same transaction.
        assert!(ledger.add_to_memory_pool(transaction).is_err());
    }

    #[test]
    #[traced_test]
    fn test_ledger_execute() {
        let rng = &mut TestRng::default();

        // Sample the genesis private key.
        let private_key = crate::consensus::test_helpers::sample_genesis_private_key(rng);
        // Sample the genesis ledger.
        let mut ledger = test_helpers::sample_genesis_ledger(rng);

        // Add a transaction to the memory pool.
        let transaction = crate::consensus::test_helpers::sample_execution_transaction(rng);
        ledger.add_to_memory_pool(transaction.clone()).unwrap();

        // Propose the next block.
        let next_block = ledger.propose_next_block(&private_key, rng).unwrap();

        // Construct a next block.
        ledger.add_next_block(&next_block).unwrap();
        assert_eq!(ledger.latest_height(), 1);
        assert_eq!(ledger.latest_hash(), next_block.hash());

        // Ensure that the ledger deems the same transaction invalid.
        assert!(ledger.check_transaction(&transaction).is_err());
        // Ensure that the ledger cannot add the same transaction.
        assert!(ledger.add_to_memory_pool(transaction).is_err());
    }

    #[test]
    #[traced_test]
    fn test_ledger_execute_many() {
        let rng = &mut TestRng::default();

        // Sample the genesis private key, view key, and address.
        let private_key = crate::consensus::test_helpers::sample_genesis_private_key(rng);
        let view_key = ViewKey::try_from(private_key).unwrap();
        let address = Address::try_from(&view_key).unwrap();

        // Initialize the store.
        let store = ConsensusStore::<_, ConsensusMemory<_>>::open(None).unwrap();
        // Create a genesis block.
        let genesis = Block::genesis(&VM::from(store).unwrap(), &private_key, rng).unwrap();
        // Initialize the ledger.
        let mut ledger = CurrentConsensus::new_with_genesis(&genesis, address, None).unwrap();

        for height in 1..6 {
            // Fetch the unspent records.
            let records: Vec<_> = ledger
                .find_records(&view_key, RecordsFilter::Unspent)
                .unwrap()
                .filter(|(_, record)| !record.gates().is_zero())
                .collect();
            assert_eq!(records.len(), 1 << (height - 1));

            for (_, record) in records {
                // Create a new transaction.
                let transaction = Transaction::execute(
                    ledger.vm(),
                    &private_key,
                    &ProgramID::from_str("credits.aleo").unwrap(),
                    Identifier::from_str("split").unwrap(),
                    &[
                        Value::Record(record.clone()),
                        Value::from_str(&format!("{}u64", ***record.gates() / 2)).unwrap(),
                    ],
                    None,
                    rng,
                )
                .unwrap();
                // Add the transaction to the memory pool.
                ledger.add_to_memory_pool(transaction).unwrap();
            }
            assert_eq!(ledger.memory_pool().len(), 1 << (height - 1));

            // Propose the next block.
            let next_block = ledger.propose_next_block(&private_key, rng).unwrap();

            // Construct a next block.
            ledger.add_next_block(&next_block).unwrap();
            assert_eq!(ledger.latest_height(), height);
            assert_eq!(ledger.latest_hash(), next_block.hash());
        }
    }

    #[test]
    #[traced_test]
    fn test_proof_target() {
        let rng = &mut TestRng::default();

        // Sample the genesis private key and address.
        let private_key = crate::consensus::test_helpers::sample_genesis_private_key(rng);
        let address = Address::try_from(&private_key).unwrap();

        // Sample the genesis ledger.
        let mut ledger = crate::consensus::test_helpers::sample_genesis_ledger(rng);

        // Fetch the proof target and epoch challenge for the block.
        let proof_target = ledger.latest_proof_target().unwrap();
        let epoch_challenge = ledger.latest_epoch_challenge().unwrap();

        for _ in 0..100 {
            // Generate a prover solution.
            let prover_solution = ledger.coinbase_puzzle.prove(&epoch_challenge, address, rng.gen()).unwrap();

            // Check that the prover solution meets the proof target requirement.
            if prover_solution.to_target().unwrap() >= proof_target {
                assert!(ledger.add_to_coinbase_memory_pool(prover_solution).is_ok())
            } else {
                assert!(ledger.add_to_coinbase_memory_pool(prover_solution).is_err())
            }
        }
    }

    #[test]
    #[traced_test]
    fn test_coinbase_target() {
        let rng = &mut TestRng::default();

        // Sample the genesis private key and address.
        let private_key = crate::consensus::test_helpers::sample_genesis_private_key(rng);
        let address = Address::try_from(&private_key).unwrap();

        // Sample the genesis ledger.
        let mut ledger = test_helpers::sample_genesis_ledger(rng);

        // Add a transaction to the memory pool.
        let transaction = crate::consensus::test_helpers::sample_execution_transaction(rng);
        ledger.add_to_memory_pool(transaction).unwrap();

        // Ensure that the ledger can't create a block that satisfies the coinbase target.
        let proposed_block = ledger.propose_next_block(&private_key, rng).unwrap();
        // Ensure the block does not contain a coinbase proof.
        assert!(proposed_block.coinbase_proof().is_none());

        // Check that the ledger won't generate a block for a cumulative target that does not meet the requirements.
        let mut cumulative_target = 0u128;
        let epoch_challenge = ledger.latest_epoch_challenge().unwrap();

        while cumulative_target < ledger.latest_coinbase_target().unwrap() as u128 {
            // Generate a prover solution.
            let prover_solution = match ledger.coinbase_puzzle.prove(&epoch_challenge, address, rng.gen()) {
                Ok(prover_solution) => prover_solution,
                Err(_) => continue,
            };

            // Try to add the prover solution to the memory pool.
            if ledger.add_to_coinbase_memory_pool(prover_solution).is_ok() {
                // Add to the cumulative target if the prover solution is valid.
                cumulative_target += prover_solution.to_target().unwrap() as u128;
            }
        }

        // Ensure that the ledger can create a block that satisfies the coinbase target.
        let proposed_block = ledger.propose_next_block(&private_key, rng).unwrap();
        // Ensure the block contains a coinbase proof.
        assert!(proposed_block.coinbase_proof().is_some());
    }
}
