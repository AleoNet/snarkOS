// Copyright (C) 2019-2020 Aleo Systems Inc.
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

use crate::{difficulty::bitcoin_retarget, memory_pool::MemoryPool, MerkleTreeLedger};
use snarkos_curves::bls12_377::Bls12_377;
use snarkos_dpc::base_dpc::{
    instantiated::*,
    parameters::PublicParameters,
    program::NoopProgram,
    record::DPCRecord,
    record_payload::RecordPayload,
    BaseDPCComponents,
};
use snarkos_errors::consensus::ConsensusError;
use snarkos_models::{
    algorithms::{CRH, SNARK},
    dpc::{DPCComponents, DPCScheme, Program},
    objects::{AccountScheme, LedgerScheme},
};
use snarkos_objects::{
    dpc::DPCTransactions,
    Account,
    AccountAddress,
    AccountPrivateKey,
    AleoAmount,
    Block,
    BlockHeader,
    BlockHeaderHash,
    MerkleRootHash,
    Network,
    PedersenMerkleRootHash,
};
use snarkos_posw::{txids_to_roots, Marlin, PoswMarlin};
use snarkos_profiler::{end_timer, start_timer};
use snarkos_storage::BlockPath;
use snarkos_utilities::{to_bytes, FromBytes, ToBytes};

use chrono::Utc;
use rand::Rng;

pub const TWO_HOURS_UNIX: i64 = 7200;

/// Parameters for a proof of work blockchain.
#[derive(Clone, Debug)]
pub struct ConsensusParameters {
    /// Maximum block size in bytes
    pub max_block_size: usize,

    /// Maximum nonce value allowed
    pub max_nonce: u32,

    /// The amount of time it should take to find a block
    pub target_block_time: i64,

    /// Network
    pub network: Network,

    /// The Proof of Succinct Work verifier (read-only mode, no proving key loaded)
    pub verifier: PoswMarlin,

    /// The authorized inner SNARK IDs
    pub authorized_inner_snark_ids: Vec<Vec<u8>>,
}

/// Calculate a block reward that halves every 4 years * 365 days * 24 hours * 100 blocks/hr = 3,504,000 blocks.
pub fn get_block_reward(block_num: u32) -> AleoAmount {
    let expected_blocks_per_hour: u32 = 100;
    let num_years = 4;
    let block_segments = num_years * 365 * 24 * expected_blocks_per_hour;

    let aleo_denonimation = AleoAmount::COIN;
    let initial_reward = 150i64 * aleo_denonimation;

    // The block reward halves at most 2 times - minimum is 37.5 ALEO after 8 years.
    let num_halves = u32::min(block_num / block_segments, 2);
    let reward = initial_reward / (2_u64.pow(num_halves)) as i64;

    AleoAmount::from_bytes(reward)
}

impl ConsensusParameters {
    /// Calculate the difficulty for the next block based off how long it took to mine the last one.
    pub fn get_block_difficulty(&self, prev_header: &BlockHeader, block_timestamp: i64) -> u64 {
        bitcoin_retarget(
            block_timestamp,
            prev_header.time,
            self.target_block_time,
            prev_header.difficulty_target,
        )
    }

    pub fn is_genesis(block_header: &BlockHeader) -> bool {
        block_header.previous_block_hash == BlockHeaderHash([0u8; 32])
    }

    /// Verify all fields in a block header.
    /// 1. The parent hash points to the tip of the chain.
    /// 2. Transactions hash to merkle root.
    /// 3. The timestamp is less than 2 hours into the future.
    /// 4. The timestamp is greater than parent timestamp.
    /// 5. The header is greater than or equal to target difficulty.
    /// 6. The nonce is within the limit.
    pub fn verify_header(
        &self,
        header: &BlockHeader,
        parent_header: &BlockHeader,
        merkle_root_hash: &MerkleRootHash,
        pedersen_merkle_root_hash: &PedersenMerkleRootHash,
    ) -> Result<(), ConsensusError> {
        let hash_result = header.to_difficulty_hash();

        let now = Utc::now().timestamp();
        let future_timelimit: i64 = now + TWO_HOURS_UNIX;
        let expected_difficulty = self.get_block_difficulty(parent_header, header.time);

        if parent_header.get_hash() != header.previous_block_hash {
            return Err(ConsensusError::NoParent(
                parent_header.get_hash().to_string(),
                header.previous_block_hash.to_string(),
            ));
        } else if header.merkle_root_hash != *merkle_root_hash {
            return Err(ConsensusError::MerkleRoot(header.merkle_root_hash.to_string()));
        } else if header.pedersen_merkle_root_hash != *pedersen_merkle_root_hash {
            return Err(ConsensusError::PedersenMerkleRoot(
                header.pedersen_merkle_root_hash.to_string(),
            ));
        } else if header.time > future_timelimit {
            return Err(ConsensusError::FuturisticTimestamp(future_timelimit, header.time));
        } else if header.time < parent_header.time {
            return Err(ConsensusError::TimestampInvalid(header.time, parent_header.time));
        } else if hash_result > header.difficulty_target {
            return Err(ConsensusError::PowInvalid(header.difficulty_target, hash_result));
        } else if header.nonce >= self.max_nonce {
            return Err(ConsensusError::NonceInvalid(header.nonce, self.max_nonce));
        } else if header.difficulty_target != expected_difficulty {
            return Err(ConsensusError::DifficultyMismatch(
                expected_difficulty,
                header.difficulty_target,
            ));
        }

        // Verify the proof
        let proof = <Marlin<Bls12_377> as SNARK>::Proof::read(&header.proof.0[..])?;
        let verification_timer = start_timer!(|| "POSW verify");
        self.verifier
            .verify(header.nonce, &proof, &header.pedersen_merkle_root_hash)?;
        end_timer!(verification_timer);

        Ok(())
    }

    /// Check if the transaction is valid.
    pub fn verify_transaction(
        &self,
        parameters: &<InstantiatedDPC as DPCScheme<MerkleTreeLedger>>::Parameters,
        transaction: &Tx,
        ledger: &MerkleTreeLedger,
    ) -> Result<bool, ConsensusError> {
        if !self
            .authorized_inner_snark_ids
            .contains(&to_bytes![transaction.inner_snark_id]?)
        {
            return Ok(false);
        }

        Ok(InstantiatedDPC::verify(parameters, transaction, ledger)?)
    }

    /// Check if the transactions are valid.
    pub fn verify_transactions(
        &self,
        parameters: &<InstantiatedDPC as DPCScheme<MerkleTreeLedger>>::Parameters,
        transactions: &[Tx],
        ledger: &MerkleTreeLedger,
    ) -> Result<bool, ConsensusError> {
        for tx in transactions {
            if !self.authorized_inner_snark_ids.contains(&to_bytes![tx.inner_snark_id]?) {
                return Ok(false);
            }
        }

        Ok(InstantiatedDPC::verify_transactions(parameters, transactions, ledger)?)
    }

    /// Check if the block is valid.
    /// Verify transactions and transaction fees.
    pub fn verify_block(
        &self,
        parameters: &<InstantiatedDPC as DPCScheme<MerkleTreeLedger>>::Parameters,
        block: &Block<Tx>,
        ledger: &MerkleTreeLedger,
    ) -> Result<bool, ConsensusError> {
        let transaction_ids: Vec<_> = block.transactions.to_transaction_ids()?;
        let (merkle_root, pedersen_merkle_root, _) = txids_to_roots(&transaction_ids);

        // Verify the block header
        if !Self::is_genesis(&block.header) {
            let parent_block = ledger.get_latest_block()?;
            if let Err(err) =
                self.verify_header(&block.header, &parent_block.header, &merkle_root, &pedersen_merkle_root)
            {
                println!("header failed to verify: {:?}", err);
                return Ok(false);
            }
        }
        // Verify block amounts and check that there is a single coinbase transaction

        let mut coinbase_transaction_count = 0;
        let mut total_value_balance = AleoAmount::ZERO;

        for transaction in block.transactions.iter() {
            let value_balance = transaction.value_balance;

            if value_balance.is_negative() {
                coinbase_transaction_count += 1;
            }

            total_value_balance = total_value_balance.add(value_balance);
        }

        // Check that there is only 1 coinbase transaction
        if coinbase_transaction_count > 1 {
            println!("error - multiple coinbase transactions");
            return Ok(false);
        }

        // Check that the block value balances are correct
        let expected_block_reward = get_block_reward(ledger.len() as u32).0;
        if total_value_balance.0 + expected_block_reward != 0 {
            println!("total_value_balance: {:?}", total_value_balance);
            println!("expected_block_reward: {:?}", expected_block_reward);

            return Ok(false);
        }

        // Check that all the transction proofs verify
        Ok(self.verify_transactions(parameters, &block.transactions.0, ledger)?)
    }

    /// Return whether or not the given block is valid and insert it.
    /// 1. Verify that the block header is valid.
    /// 2. Verify that the transactions are valid.
    /// 3. Insert/canonize block.
    pub fn process_block(
        &self,
        parameters: &PublicParameters<Components>,
        storage: &MerkleTreeLedger,
        memory_pool: &mut MemoryPool<Tx>,
        block: &Block<Tx>,
    ) -> Result<(), ConsensusError> {
        if storage.is_canon(&block.header.get_hash()) {
            return Ok(());
        }

        // 1. Verify that the block valid
        if !self.verify_block(parameters, block, storage)? {
            return Err(ConsensusError::InvalidBlock(block.header.get_hash().0.to_vec()));
        }

        // 2. Insert/canonize block
        storage.insert_and_commit(block)?;

        // 3. Remove transactions from the mempool
        for transaction_id in block.transactions.to_transaction_ids()? {
            memory_pool.remove_by_hash(&transaction_id)?;
        }

        Ok(())
    }

    /// Receive a block from an external source and process it based on ledger state.
    pub fn receive_block(
        &self,
        parameters: &PublicParameters<Components>,
        storage: &MerkleTreeLedger,
        memory_pool: &mut MemoryPool<Tx>,
        block: &Block<Tx>,
    ) -> Result<(), ConsensusError> {
        let block_size = block.serialize()?.len();
        if block_size > self.max_block_size {
            return Err(ConsensusError::BlockTooLarge(block_size, self.max_block_size));
        }

        // Block is an unknown orphan
        if !storage.previous_block_hash_exists(block) && !storage.is_previous_block_canon(&block.header) {
            debug!("Processing a block that is an unknown orphan");

            // There are two possible cases for an unknown orphan.
            // 1) The block is a genesis block, or
            // 2) The block is unknown and does not correspond with the canon chain.
            if Self::is_genesis(&block.header) && storage.is_empty() {
                self.process_block(parameters, &storage, memory_pool, &block)?;
            } else {
                storage.insert_only(block)?;
            }
        } else {
            // If the block is not an unknown orphan, find the origin of the block
            match storage.get_block_path(&block.header)? {
                BlockPath::ExistingBlock => {}
                BlockPath::CanonChain(block_height) => {
                    debug!("Processing a block that is on canon chain. Height {}", block_height);

                    self.process_block(parameters, &storage, memory_pool, block)?;

                    // Attempt to fast forward the block state if the node already stores
                    // the children of the new canon block.
                    let (_, child_path) = storage.longest_child_path(block.header.get_hash())?;
                    for child_block_hash in child_path {
                        let new_block = storage.get_block(&child_block_hash)?;
                        self.process_block(parameters, &storage, memory_pool, &new_block)?;
                    }
                }
                BlockPath::SideChain(side_chain_path) => {
                    debug!(
                        "Processing a block that is on side chain. Height {}",
                        side_chain_path.new_block_number
                    );

                    // If the side chain is now longer than the canon chain,
                    // perform a fork to the side chain.
                    if side_chain_path.new_block_number > storage.get_current_block_height() {
                        debug!(
                            "Determined side chain is longer than canon chain by {} blocks",
                            side_chain_path.new_block_number - storage.get_current_block_height()
                        );
                        warn!("A valid fork has been detected. Performing a fork to the side chain.");

                        // Fork to superior side chain
                        storage.revert_for_fork(&side_chain_path)?;

                        if !side_chain_path.path.is_empty() {
                            for block_hash in side_chain_path.path {
                                if block_hash == block.header.get_hash() {
                                    self.process_block(parameters, &storage, memory_pool, &block)?
                                } else {
                                    let new_block = storage.get_block(&block_hash)?;
                                    self.process_block(parameters, &storage, memory_pool, &new_block)?;
                                }
                            }
                        }
                    } else {
                        // If the sidechain is not longer than the main canon chain, simply store the block
                        storage.insert_only(block)?;
                    }
                }
            };
        }

        Ok(())
    }

    /// Generate a coinbase transaction given candidate block transactions
    #[allow(clippy::too_many_arguments)]
    pub fn create_coinbase_transaction<R: Rng>(
        &self,
        block_num: u32,
        transactions: &DPCTransactions<Tx>,
        parameters: &PublicParameters<Components>,
        program_vk_hash: Vec<u8>,
        new_birth_program_ids: Vec<Vec<u8>>,
        new_death_program_ids: Vec<Vec<u8>>,
        recipient: AccountAddress<Components>,
        ledger: &MerkleTreeLedger,
        rng: &mut R,
    ) -> Result<(Vec<DPCRecord<Components>>, Tx), ConsensusError> {
        let mut total_value_balance = get_block_reward(block_num);

        for transaction in transactions.iter() {
            let tx_value_balance = transaction.value_balance;

            if tx_value_balance.is_negative() {
                return Err(ConsensusError::CoinbaseTransactionAlreadyExists());
            }

            total_value_balance = total_value_balance.add(transaction.value_balance);
        }

        // Generate a new account that owns the dummy input records
        let new_account = Account::new(
            &parameters.system_parameters.account_signature,
            &parameters.system_parameters.account_commitment,
            &parameters.system_parameters.account_encryption,
            rng,
        )
        .unwrap();

        // Generate dummy input records having as address the genesis address.
        let old_account_private_keys = vec![new_account.private_key.clone(); Components::NUM_INPUT_RECORDS];
        let mut old_records = Vec::with_capacity(Components::NUM_INPUT_RECORDS);
        for _ in 0..Components::NUM_INPUT_RECORDS {
            let sn_nonce_input: [u8; 4] = rng.gen();

            let old_sn_nonce =
                SerialNumberNonce::hash(&parameters.system_parameters.serial_number_nonce, &sn_nonce_input)?;

            let old_record = InstantiatedDPC::generate_record(
                parameters.system_parameters.clone(),
                old_sn_nonce,
                new_account.address.clone(),
                true, // The input record is dummy
                0,
                RecordPayload::default(),
                // Filler program input
                program_vk_hash.clone(),
                program_vk_hash.clone(),
                rng,
            )?;

            old_records.push(old_record);
        }

        let new_record_owners = vec![recipient; Components::NUM_OUTPUT_RECORDS];
        let new_is_dummy_flags = [vec![false], vec![true; Components::NUM_OUTPUT_RECORDS - 1]].concat();
        let new_values = [vec![total_value_balance.0 as u64], vec![
            0;
            Components::NUM_OUTPUT_RECORDS
                - 1
        ]]
        .concat();
        let new_payloads = vec![RecordPayload::default(); NUM_OUTPUT_RECORDS];

        let memo: [u8; 32] = rng.gen();

        self.create_transaction(
            parameters,
            old_records,
            old_account_private_keys,
            new_record_owners,
            new_birth_program_ids,
            new_death_program_ids,
            new_is_dummy_flags,
            new_values,
            new_payloads,
            memo,
            ledger,
            rng,
        )
    }

    /// Generate a transaction by spending old records and specifying new record attributes
    #[allow(clippy::too_many_arguments)]
    pub fn create_transaction<R: Rng>(
        &self,
        parameters: &<InstantiatedDPC as DPCScheme<MerkleTreeLedger>>::Parameters,
        old_records: Vec<DPCRecord<Components>>,
        old_account_private_keys: Vec<AccountPrivateKey<Components>>,
        new_record_owners: Vec<AccountAddress<Components>>,
        new_birth_program_ids: Vec<Vec<u8>>,
        new_death_program_ids: Vec<Vec<u8>>,
        new_is_dummy_flags: Vec<bool>,
        new_values: Vec<u64>,
        new_payloads: Vec<RecordPayload>,
        memo: [u8; 32],
        ledger: &MerkleTreeLedger,
        rng: &mut R,
    ) -> Result<(Vec<DPCRecord<Components>>, Tx), ConsensusError> {
        // Offline execution to generate a DPC transaction
        let execute_context = <InstantiatedDPC as DPCScheme<MerkleTreeLedger>>::execute_offline(
            parameters.system_parameters.clone(),
            old_records,
            old_account_private_keys,
            new_record_owners,
            &new_is_dummy_flags,
            &new_values,
            new_payloads,
            new_birth_program_ids,
            new_death_program_ids,
            memo,
            self.network.id(),
            rng,
        )?;

        // Construct the program proofs

        let local_data = execute_context.into_local_data();

        let noop_program_snark_id = to_bytes![ProgramVerificationKeyCRH::hash(
            &parameters.system_parameters.program_verification_key_crh,
            &to_bytes![parameters.noop_program_snark_parameters.verification_key]?
        )?]?;

        let dpc_program =
            NoopProgram::<_, <Components as BaseDPCComponents>::NoopProgramSNARK>::new(noop_program_snark_id);

        let mut old_death_program_proofs = Vec::with_capacity(NUM_INPUT_RECORDS);
        for i in 0..NUM_INPUT_RECORDS {
            let private_input = dpc_program.execute(
                &parameters.noop_program_snark_parameters.proving_key,
                &parameters.noop_program_snark_parameters.verification_key,
                &local_data,
                i as u8,
                rng,
            )?;

            old_death_program_proofs.push(private_input);
        }

        let mut new_birth_program_proofs = Vec::with_capacity(NUM_OUTPUT_RECORDS);
        for j in 0..NUM_OUTPUT_RECORDS {
            let private_input = dpc_program.execute(
                &parameters.noop_program_snark_parameters.proving_key,
                &parameters.noop_program_snark_parameters.verification_key,
                &local_data,
                (NUM_INPUT_RECORDS + j) as u8,
                rng,
            )?;

            new_birth_program_proofs.push(private_input);
        }

        // Online execution to generate a DPC transaction
        let (new_records, transaction) = InstantiatedDPC::execute_online(
            &parameters,
            execute_context,
            old_death_program_proofs,
            new_birth_program_proofs,
            ledger,
            rng,
        )?;

        Ok((new_records, transaction))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{thread_rng, Rng};
    use snarkos_objects::PedersenMerkleRootHash;
    use snarkos_testing::consensus::DATA;

    #[test]
    fn test_block_rewards() {
        let rng = &mut thread_rng();

        let first_halfing: u32 = 4 * 365 * 24 * 100;
        let second_halfing: u32 = first_halfing * 2;

        let mut block_reward: i64 = 150 * 1_000_000;

        // Before block halving
        assert_eq!(get_block_reward(0).0, block_reward);

        for _ in 0..100 {
            let block_num: u32 = rng.gen_range(0, first_halfing);
            assert_eq!(get_block_reward(block_num).0, block_reward);
        }

        // First block halving

        block_reward /= 2;

        assert_eq!(get_block_reward(first_halfing).0, block_reward);

        for _ in 0..100 {
            let block_num: u32 = rng.gen_range(first_halfing + 1, second_halfing);
            assert_eq!(get_block_reward(block_num).0, block_reward);
        }

        // Second and final block halving

        block_reward /= 2;

        assert_eq!(get_block_reward(second_halfing).0, block_reward);
        assert_eq!(get_block_reward(u32::MAX).0, block_reward);

        for _ in 0..100 {
            let block_num: u32 = rng.gen_range(second_halfing, u32::MAX);
            assert_eq!(get_block_reward(block_num).0, block_reward);
        }
    }

    #[test]
    fn verify_header() {
        // mine a PoSW proof
        let posw = PoswMarlin::load().unwrap();

        let consensus: ConsensusParameters = ConsensusParameters {
            max_block_size: 1_000_000usize,
            max_nonce: std::u32::MAX - 1,
            target_block_time: 2i64, //unix seconds
            network: Network::Mainnet,
            verifier: posw,
            authorized_inner_snark_ids: vec![],
        };

        let b1 = DATA.block_1.clone();
        let h1 = b1.header;

        let b2 = DATA.block_2.clone();
        let h2 = b2.header;
        let merkle_root_hash = h2.merkle_root_hash.clone();
        let pedersen_merkle_root = h2.pedersen_merkle_root_hash.clone();

        // OK
        consensus
            .verify_header(&h2, &h1, &merkle_root_hash, &pedersen_merkle_root)
            .unwrap();

        // invalid parent hash
        let mut h2_err = h2.clone();
        h2_err.previous_block_hash = BlockHeaderHash([9; 32]);
        consensus
            .verify_header(&h2_err, &h1, &merkle_root_hash, &pedersen_merkle_root)
            .unwrap_err();

        // invalid merkle root hash
        let mut h2_err = h2.clone();
        h2_err.merkle_root_hash = MerkleRootHash([3; 32]);
        consensus
            .verify_header(&h2_err, &h1, &merkle_root_hash, &pedersen_merkle_root)
            .unwrap_err();

        // past block
        let mut h2_err = h2.clone();
        h2_err.time = 100;
        consensus
            .verify_header(&h2_err, &h1, &merkle_root_hash, &pedersen_merkle_root)
            .unwrap_err();

        // far in the future block
        let mut h2_err = h2.clone();
        h2_err.time = Utc::now().timestamp() + 7201;
        consensus
            .verify_header(&h2_err, &h1, &merkle_root_hash, &pedersen_merkle_root)
            .unwrap_err();

        // invalid difficulty
        let mut h2_err = h2.clone();
        h2_err.difficulty_target = 100; // set the difficulty very very high
        consensus
            .verify_header(&h2_err, &h1, &merkle_root_hash, &pedersen_merkle_root)
            .unwrap_err();

        // invalid nonce
        let mut h2_err = h2.clone();
        h2_err.nonce = std::u32::MAX; // over the max nonce
        consensus
            .verify_header(&h2_err, &h1, &merkle_root_hash, &pedersen_merkle_root)
            .unwrap_err();

        // invalid pedersen merkle root hash
        let mut h2_err = h2.clone();
        h2_err.pedersen_merkle_root_hash = PedersenMerkleRootHash([9; 32]);
        consensus
            .verify_header(&h2_err, &h1, &merkle_root_hash, &pedersen_merkle_root)
            .unwrap_err();

        // expected difficulty did not match the difficulty target
        let mut h2_err = h2;
        h2_err.difficulty_target = consensus.get_block_difficulty(&h1, Utc::now().timestamp()) + 1;
        consensus
            .verify_header(&h2_err, &h1, &merkle_root_hash, &pedersen_merkle_root)
            .unwrap_err();
    }
}
