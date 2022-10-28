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

use crate::memory_pool::MemoryPool;
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
        store::{BlockStore, ConsensusStorage, ConsensusStore, TransactionStore, TransitionStore},
        vm::VM,
    },
};

use anyhow::Result;
use indexmap::IndexMap;
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
    /// The beacons.
    // TODO (howardwu): Update this to retrieve from a beacons store.
    beacons: IndexMap<Address<N>, ()>,
    /// The memory pool.
    memory_pool: MemoryPool<N>,
    // /// The mapping of program IDs to their global state.
    // states: MemoryMap<ProgramID<N>, IndexMap<Identifier<N>, Plaintext<N>>>,
}

impl<N: Network, C: ConsensusStorage<N>> Consensus<N, C> {
    /// Loads the consensus module from storage.
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
        // Initialize consensus.
        let consensus = Self::from(vm, genesis)?;

        // Ensure the ledger contains the correct genesis block.
        match consensus.contains_block_hash(&genesis_hash)? {
            true => Ok(consensus),
            false => bail!("Incorrect genesis block (run 'snarkos clean' and try again)"),
        }
    }

    /// Initializes the consensus module from storage, with an optional genesis block.
    pub fn from(vm: VM<N, C>, genesis: Option<Block<N>>) -> Result<Self> {
        // Load the coinbase puzzle.
        let coinbase_puzzle = CoinbasePuzzle::<N>::load()?;

        // Initialize the consensus.
        let mut consensus = Self {
            current_hash: Default::default(),
            current_height: 0,
            current_round: 0,
            block_tree: N::merkle_tree_bhp(&[])?,
            blocks: vm.block_store().clone(),
            transactions: vm.transaction_store().clone(),
            transitions: vm.transition_store().clone(),
            // TODO (howardwu): Update this to retrieve from a validators store.
            beacons: Default::default(),
            memory_pool: Default::default(),
            coinbase_puzzle,
            vm,
        };

        // If the block store is empty, initialize the genesis block.
        if consensus.blocks.heights().max().is_none() {
            // Load the genesis block.
            let genesis = match genesis {
                Some(genesis) => genesis,
                None => Block::<N>::from_bytes_le(N::genesis_bytes())?,
            };
            // Add the initial beacon.
            consensus.add_beacon(genesis.signature().to_address())?;
            // Add the genesis block.
            consensus.add_next_block(&genesis)?;
        }

        // Retrieve the latest height.
        let latest_height =
            *consensus.blocks.heights().max().ok_or_else(|| anyhow!("Failed to load blocks in consensus"))?;
        // Fetch the latest block.
        let block = consensus
            .get_block(latest_height)
            .map_err(|_| anyhow!("Failed to load block {latest_height} in consensus"))?;

        // Set the current hash, height, and round.
        consensus.current_hash = block.hash();
        consensus.current_height = block.height();
        consensus.current_round = block.round();

        // TODO (howardwu): Improve the performance here by using iterators.
        // Generate the block tree.
        let hashes: Vec<_> =
            (1..=latest_height).map(|height| consensus.get_hash(height).map(|hash| hash.to_bits_le())).try_collect()?;
        consensus.block_tree.append(&hashes)?;

        // Add the genesis beacon.
        let genesis_beacon = consensus.get_block(0)?.signature().to_address();
        if !consensus.beacons.contains_key(&genesis_beacon) {
            consensus.add_beacon(genesis_beacon)?;
        }

        // Safety check the existence of every block.
        cfg_into_iter!((0..=latest_height)).try_for_each(|height| {
            consensus.get_block(height)?;
            Ok::<_, Error>(())
        })?;

        Ok(consensus)
    }

    /// Adds the given unconfirmed transaction to the memory pool.
    pub fn add_unconfirmed_transaction(&mut self, transaction: Transaction<N>) -> Result<()> {
        // Check that the transaction is well-formed and unique.
        self.check_transaction_basic(&transaction)?;

        // Insert the transaction to the memory pool.
        self.memory_pool.add_unconfirmed_transaction(&transaction);

        Ok(())
    }

    /// Adds the given unconfirmed solution to the memory pool.
    pub fn add_unconfirmed_solution(&mut self, solution: &ProverSolution<N>) -> Result<()> {
        // Ensure that prover solutions are not accepted after 10 years.
        if self.latest_height() > anchor_block_height(N::ANCHOR_TIME, 10) {
            bail!("Coinbase proofs are no longer accepted after year 10.");
        }
        // Ensure the prover solution is not already in the ledger.
        if self.contains_puzzle_commitment(&solution.commitment())? {
            bail!("Prover solution is already in the ledger.");
        }
        // Ensure the prover solution is not already in the memory pool.
        if self.memory_pool.contains_unconfirmed_solution(solution.commitment()) {
            bail!("Prover solution is already in the memory pool.");
        }

        // Compute the current epoch challenge.
        let epoch_challenge = self.latest_epoch_challenge()?;
        // Retrieve the current proof target.
        let proof_target = self.latest_proof_target()?;

        // Ensure that the prover solution is valid for the given epoch.
        if !solution.verify(self.coinbase_puzzle.coinbase_verifying_key()?, &epoch_challenge, proof_target)? {
            bail!("Invalid prover solution '{}' for the current epoch.", solution.commitment());
        }

        // Insert the solution to the memory pool.
        self.memory_pool.add_unconfirmed_solution(solution)?;

        Ok(())
    }

    /// Returns the candidate coinbase target of the valid unconfirmed solutions in the memory pool.
    pub fn candidate_coinbase_target(&self) -> Result<u128> {
        // Retrieve the latest proof target.
        let latest_proof_target = self.latest_proof_target()?;
        // Compute the candidate coinbase target.
        self.memory_pool.candidate_coinbase_target(latest_proof_target)
    }

    /// Returns `true` if the coinbase target is met.
    pub fn is_coinbase_target_met(&self) -> Result<bool> {
        // Retrieve the latest block header.
        let header = self.latest_header()?;
        // Compute the candidate coinbase target.
        let cumuluative_proof_target = self.memory_pool.candidate_coinbase_target(header.proof_target())?;
        // Check if the coinbase target is met.
        Ok(cumuluative_proof_target >= header.coinbase_target() as u128)
    }

    /// Returns a candidate for the next block in the ledger.
    pub fn propose_next_block<R: Rng + CryptoRng>(&self, private_key: &PrivateKey<N>, rng: &mut R) -> Result<Block<N>> {
        // Retrieve the latest state root.
        let latest_state_root = self.latest_state_root();
        // Retrieve the latest block.
        let latest_block = self.latest_block()?;
        // Retrieve the latest proof target.
        let latest_proof_target = latest_block.proof_target();
        // Retrieve the latest coinbase target.
        let latest_coinbase_target = latest_block.coinbase_target();

        // Select the transactions from the memory pool.
        let transactions = self.memory_pool.candidate_transactions(self).into_iter().collect::<Transactions<N>>();
        // Select the prover solutions from the memory pool.
        let prover_solutions =
            self.memory_pool.candidate_solutions(self.latest_height(), latest_proof_target, latest_coinbase_target)?;

        // Construct the coinbase solution.
        let (coinbase, coinbase_accumulator_point) = match &prover_solutions {
            Some(prover_solutions) => {
                let epoch_challenge = self.latest_epoch_challenge()?;
                let coinbase_solution =
                    self.coinbase_puzzle.accumulate_unchecked(&epoch_challenge, prover_solutions)?;
                let coinbase_accumulator_point = coinbase_solution.to_accumulator_point()?;

                (Some(coinbase_solution), coinbase_accumulator_point)
            }
            None => (None, Field::<N>::zero()),
        };

        // Fetch the next round state.
        let next_timestamp = OffsetDateTime::now_utc().unix_timestamp();
        let next_height = self.latest_height().saturating_add(1);
        let next_round = latest_block.round().saturating_add(1);

        // TODO (raychu86): Pay the provers. Currently we do not pay the provers with the `credits.aleo` program
        //  and instead, will track prover leaderboards via the `coinbase_solution` in each block.
        if let Some(prover_solutions) = prover_solutions {
            // Calculate the coinbase reward.
            let coinbase_reward = coinbase_reward(
                latest_block.last_coinbase_timestamp(),
                next_timestamp,
                next_height,
                N::STARTING_SUPPLY,
                N::ANCHOR_TIME,
            )?;

            // Compute the cumulative proof target of the prover solutions as a u128.
            let cumulative_proof_target: u128 = prover_solutions.iter().try_fold(0u128, |cumulative, solution| {
                cumulative
                    .checked_add(solution.to_target()? as u128)
                    .ok_or_else(|| anyhow!("Cumulative proof target overflowed"))
            })?;

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
                let denominator = cumulative_proof_target
                    .checked_mul(2)
                    .ok_or_else(|| anyhow!("Prover reward denominator overflowed"))?;

                // Compute the prover reward.
                let prover_reward = u64::try_from(
                    numerator.checked_div(denominator).ok_or_else(|| anyhow!("Prover reward overflowed"))?,
                )?;

                prover_rewards.push((prover_solution.address(), prover_reward));
            }
        }

        // Construct the next coinbase target.
        let next_coinbase_target = coinbase_target(
            latest_coinbase_target,
            latest_block.last_coinbase_timestamp(),
            next_timestamp,
            N::ANCHOR_TIME,
            N::NUM_BLOCKS_PER_EPOCH,
        )?;

        // Construct the next proof target.
        let next_proof_target = proof_target(next_coinbase_target);

        // Construct the next coinbase timestamp.
        let next_coinbase_timestamp = match coinbase {
            Some(_) => next_timestamp,
            None => latest_block.last_coinbase_timestamp(),
        };

        // Construct the metadata.
        let metadata = Metadata::new(
            N::ID,
            next_round,
            next_height,
            next_coinbase_target,
            next_proof_target,
            next_coinbase_timestamp,
            next_timestamp,
        )?;

        // Construct the header.
        let header = Header::from(*latest_state_root, transactions.to_root()?, coinbase_accumulator_point, metadata)?;

        // Construct the new block.
        Block::new(private_key, latest_block.hash(), header, transactions, coinbase, rng)
    }

    /// Checks the given block is valid next block.
    pub fn check_next_block(&self, block: &Block<N>) -> Result<()> {
        // Ensure the previous block hash is correct.
        if self.current_hash != block.previous_hash() {
            bail!("The next block has an incorrect previous block hash")
        }

        // Ensure the block hash does not already exist.
        if self.contains_block_hash(&block.hash())? {
            bail!("Block hash '{}' already exists in the ledger", block.hash())
        }

        // Ensure the next block height is correct.
        if self.latest_height() > 0 && self.latest_height() + 1 != block.height() {
            bail!("The next block has an incorrect block height")
        }

        // Ensure the block height does not already exist.
        if self.contains_block_height(block.height())? {
            bail!("Block height '{}' already exists in the ledger", block.height())
        }

        // TODO (raychu86): Ensure the next round number includes timeouts.
        // Ensure the next round is correct.
        if self.latest_round() > 0 && self.latest_round() + 1 /*+ block.number_of_timeouts()*/ != block.round() {
            bail!("The next block has an incorrect round number")
        }

        // TODO (raychu86): Ensure the next block timestamp is the median of proposed blocks.
        // Ensure the next block timestamp is after the current block timestamp.
        if block.height() > 0 {
            let next_timestamp = block.header().timestamp();
            let latest_timestamp = self.latest_block()?.header().timestamp();
            if next_timestamp <= latest_timestamp {
                bail!("The next block timestamp {next_timestamp} is before the current timestamp {latest_timestamp}")
            }
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

        // Ensure the block is signed by an authorized beacon.
        let signer = block.signature().to_address();
        if !self.beacons.contains_key(&signer) {
            let beacon = self.beacons.iter().next().unwrap().0;
            eprintln!("{} {signer} {} {}", *beacon, *beacon == signer, self.beacons.contains_key(&signer));
            bail!("Block {} ({}) is signed by an unauthorized beacon ({})", block.height(), block.hash(), signer);
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
        cfg_iter!(block.transactions()).try_for_each(|(_, transaction)| {
            self.check_transaction_basic(transaction)
                .map_err(|e| anyhow!("Invalid transaction found in the transactions list: {e}"))
        })?;

        /* Coinbase Proof */

        // Ensure the coinbase solution is valid, if it exists.
        if let Some(coinbase) = block.coinbase() {
            // Ensure coinbase solutions are not accepted after the anchor block height at year 10.
            if block.height() > anchor_block_height(N::ANCHOR_TIME, 10) {
                bail!("Coinbase proofs are no longer accepted after the anchor block height at year 10.");
            }
            // Ensure the coinbase accumulator point matches in the block header.
            if block.header().coinbase_accumulator_point() != coinbase.to_accumulator_point()? {
                bail!("Coinbase accumulator point does not match the coinbase solution.");
            }
            // Ensure the puzzle commitments are new.
            for puzzle_commitment in coinbase.puzzle_commitments() {
                if self.contains_puzzle_commitment(&puzzle_commitment)? {
                    bail!("Puzzle commitment {puzzle_commitment} already exists in the ledger");
                }
            }
            // Ensure the last coinbase timestamp matches the *next block timestamp*.
            if block.last_coinbase_timestamp() != block.timestamp() {
                bail!("The last coinbase timestamp does not match the next block timestamp.");
            }
            // Ensure the coinbase solution is valid.
            if !self.coinbase_puzzle.verify(
                coinbase,
                &self.latest_epoch_challenge()?,
                self.latest_coinbase_target()?,
                self.latest_proof_target()?,
            )? {
                bail!("Invalid coinbase solution: {:?}", coinbase);
            }
        } else {
            // Ensure that the block header does not contain a coinbase accumulator point.
            if block.header().coinbase_accumulator_point() != Field::<N>::zero() {
                bail!("Coinbase accumulator point should be zero as there is no coinbase solution in the block.");
            }
            // Ensure the last coinbase timestamp matches the *latest coinbase timestamp*.
            if block.height() > 0 && block.last_coinbase_timestamp() != self.latest_coinbase_timestamp()? {
                bail!("The last coinbase timestamp does not match the latest coinbase timestamp.");
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
            let mut consensus = self.clone();

            // Update the blocks.
            consensus.current_hash = block.hash();
            consensus.current_height = block.height();
            consensus.current_round = block.round();
            consensus.block_tree.append(&[block.hash().to_bits_le()])?;
            consensus.blocks.insert(*consensus.block_tree.root(), block)?;

            // Update the VM.
            for transaction in block.transactions().values() {
                consensus.vm.finalize(transaction)?;
            }

            // Clear the memory pool of unconfirmed transactions that are now invalid.
            consensus.memory_pool.clear_invalid_transactions(&consensus.clone());

            // Clear the memory pool of the unconfirmed solutions if a new epoch has started.
            if block.epoch_number() > self.latest_epoch_number() {
                consensus.memory_pool.clear_unconfirmed_solutions();
            } else if let Some(coinbase_solution) = block.coinbase() {
                // Clear the memory pool of unconfirmed solutions that are now invalid.
                coinbase_solution.partial_solutions().iter().map(|s| s.commitment()).for_each(|commitment| {
                    consensus.memory_pool.remove_unconfirmed_solution(&commitment);
                });
            }

            *self = Self {
                current_hash: consensus.current_hash,
                current_height: consensus.current_height,
                current_round: consensus.current_round,
                block_tree: consensus.block_tree,
                blocks: consensus.blocks,
                transactions: consensus.transactions,
                transitions: consensus.transitions,
                beacons: consensus.beacons,
                vm: consensus.vm,
                memory_pool: consensus.memory_pool,
                coinbase_puzzle: consensus.coinbase_puzzle,
            };
        }

        Ok(())
    }

    /// Adds a given address to the beacon set.
    pub fn add_beacon(&mut self, address: Address<N>) -> Result<()> {
        if self.beacons.insert(address, ()).is_some() {
            bail!("'{address}' is already in the beacon set.")
        } else {
            Ok(())
        }
    }

    /// Removes a given address from the beacon set.
    pub fn remove_beacon(&mut self, address: Address<N>) -> Result<()> {
        if self.beacons.remove(&address).is_none() { bail!("'{address}' is not in the beacon set.") } else { Ok(()) }
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

    /// Returns the beacon set.
    pub const fn beacons(&self) -> &IndexMap<Address<N>, ()> {
        &self.beacons
    }

    /// Returns the memory pool.
    pub const fn memory_pool(&self) -> &MemoryPool<N> {
        &self.memory_pool
    }

    /// Returns a state path for the given commitment.
    pub fn to_state_path(&self, commitment: &Field<N>) -> Result<StatePath<N>> {
        StatePath::new_commitment(&self.block_tree, &self.blocks, commitment)
    }

    /// Checks the given transaction is well formed and unique.
    pub fn check_transaction_basic(&self, transaction: &Transaction<N>) -> Result<()> {
        let transaction_id = transaction.id();

        // Ensure the ledger does not already contain the given transaction ID.
        if self.contains_transaction_id(&transaction_id)? {
            bail!("Transaction '{transaction_id}' already exists in the ledger")
        }

        // Ensure the transaction is valid.
        if !self.vm.verify(transaction) {
            bail!("Transaction '{transaction_id}' is invalid")
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
        synthesizer::{Block, ConsensusMemory},
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
        let ledger = CurrentConsensus::load(Some(genesis.clone()), None).unwrap();
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
        synthesizer::ConsensusMemory,
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
    fn test_load() {
        let rng = &mut TestRng::default();

        // Sample the genesis private key.
        let private_key = crate::consensus::test_helpers::sample_genesis_private_key(rng);
        // Initialize the store.
        let store = ConsensusStore::<_, ConsensusMemory<_>>::open(None).unwrap();
        // Create a genesis block.
        let genesis = Block::genesis(&VM::from(store).unwrap(), &private_key, rng).unwrap();

        // Initialize consensus with the genesis block.
        let ledger = CurrentConsensus::load(Some(genesis.clone()), None).unwrap();
        assert_eq!(ledger.latest_hash(), genesis.hash());
        assert_eq!(ledger.latest_height(), genesis.height());
        assert_eq!(ledger.latest_round(), genesis.round());
        assert_eq!(ledger.latest_block().unwrap(), genesis);
    }

    #[test]
    fn test_from() {
        // Load the genesis block.
        let genesis = Block::<CurrentNetwork>::from_bytes_le(CurrentNetwork::genesis_bytes()).unwrap();

        // Initialize the VM.
        let vm = VM::from(ConsensusStore::<_, ConsensusMemory<_>>::open(None).unwrap()).unwrap();
        // Initialize consensus without the genesis block.
        let ledger = CurrentConsensus::from(vm, None).unwrap();
        assert_eq!(ledger.latest_hash(), genesis.hash());
        assert_eq!(ledger.latest_height(), genesis.height());
        assert_eq!(ledger.latest_round(), genesis.round());
        assert_eq!(ledger.latest_block().unwrap(), genesis);

        // Initialize the ledger with the genesis block.
        let ledger = CurrentConsensus::load(Some(genesis.clone()), None).unwrap();
        assert_eq!(ledger.latest_hash(), genesis.hash());
        assert_eq!(ledger.latest_height(), genesis.height());
        assert_eq!(ledger.latest_round(), genesis.round());
        assert_eq!(ledger.latest_block().unwrap(), genesis);
    }

    #[test]
    fn test_state_path() {
        // Load the genesis block.
        let genesis = Block::<CurrentNetwork>::from_bytes_le(CurrentNetwork::genesis_bytes()).unwrap();
        // Initialize the ledger with the genesis block.
        let ledger = CurrentConsensus::load(Some(genesis), None).unwrap();
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
        ledger.add_unconfirmed_transaction(transaction.clone()).unwrap();

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
        assert!(ledger.check_transaction_basic(&transaction).is_err());
        // Ensure that the ledger cannot add the same transaction.
        assert!(ledger.add_unconfirmed_transaction(transaction).is_err());
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
        ledger.add_unconfirmed_transaction(transaction.clone()).unwrap();

        // Propose the next block.
        let next_block = ledger.propose_next_block(&private_key, rng).unwrap();

        // Construct a next block.
        ledger.add_next_block(&next_block).unwrap();
        assert_eq!(ledger.latest_height(), 1);
        assert_eq!(ledger.latest_hash(), next_block.hash());

        // Ensure that the ledger deems the same transaction invalid.
        assert!(ledger.check_transaction_basic(&transaction).is_err());
        // Ensure that the ledger cannot add the same transaction.
        assert!(ledger.add_unconfirmed_transaction(transaction).is_err());
    }

    #[test]
    #[traced_test]
    fn test_ledger_execute_many() {
        let rng = &mut TestRng::default();

        // Sample the genesis private key, view key, and address.
        let private_key = crate::consensus::test_helpers::sample_genesis_private_key(rng);
        let view_key = ViewKey::try_from(private_key).unwrap();
        let _address = Address::try_from(&view_key).unwrap();

        // Initialize the store.
        let store = ConsensusStore::<_, ConsensusMemory<_>>::open(None).unwrap();
        // Create a genesis block.
        let genesis = Block::genesis(&VM::from(store).unwrap(), &private_key, rng).unwrap();
        // Initialize the ledger.
        let mut ledger = CurrentConsensus::load(Some(genesis), None).unwrap();

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
                ledger.add_unconfirmed_transaction(transaction).unwrap();
            }
            assert_eq!(ledger.memory_pool().num_unconfirmed_transactions(), 1 << (height - 1));

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
                assert!(ledger.add_unconfirmed_solution(&prover_solution).is_ok())
            } else {
                assert!(ledger.add_unconfirmed_solution(&prover_solution).is_err())
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
        ledger.add_unconfirmed_transaction(transaction).unwrap();

        // Ensure that the ledger can't create a block that satisfies the coinbase target.
        let proposed_block = ledger.propose_next_block(&private_key, rng).unwrap();
        // Ensure the block does not contain a coinbase solution.
        assert!(proposed_block.coinbase().is_none());

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
            if ledger.add_unconfirmed_solution(&prover_solution).is_ok() {
                // Add to the cumulative target if the prover solution is valid.
                cumulative_target += prover_solution.to_target().unwrap() as u128;
            }
        }

        // Ensure that the ledger can create a block that satisfies the coinbase target.
        let proposed_block = ledger.propose_next_block(&private_key, rng).unwrap();
        // Ensure the block contains a coinbase solution.
        assert!(proposed_block.coinbase().is_some());
    }
}
