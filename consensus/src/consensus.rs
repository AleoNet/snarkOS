use crate::{memory_pool::MemoryPool, MerkleTreeLedger};
use snarkos_algorithms::snark::PreparedVerifyingKey;
use snarkos_dpc::base_dpc::{
    instantiated::*,
    parameters::PublicParameters,
    predicate::{DPCPredicate, PrivatePredicateInput},
    predicate_circuit::{PredicateCircuit, PredicateLocalData},
    record::DPCRecord,
    record_payload::RecordPayload,
    LocalData,
};
use snarkos_errors::consensus::ConsensusError;
use snarkos_models::{
    algorithms::{CommitmentScheme, CRH, SNARK},
    dpc::{DPCComponents, DPCScheme},
    objects::{AccountScheme, LedgerScheme},
};
use snarkos_objects::{
    dpc::DPCTransactions,
    Account,
    AccountPrivateKey,
    AccountPublicKey,
    Block,
    BlockHeader,
    BlockHeaderHash,
    MerkleRootHash,
    PedersenMerkleRootHash,
};
use snarkos_posw::{txids_to_roots, Posw};
use snarkos_profiler::{end_timer, start_timer};
use snarkos_storage::BlockPath;

use chrono::Utc;
use rand::{thread_rng, Rng};

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

    /// Network identifier
    pub network_id: u8,

    /// The Proof of Succinct Work verifier (read-only mode, no proving key loaded)
    pub verifier: Posw,
}

/// Calculate a block reward that halves every 1000 blocks.
pub fn get_block_reward(block_num: u32) -> u64 {
    100_000_000u64 / (2_u64.pow(block_num / 1000))
}

/// Bitcoin difficulty retarget algorithm.
pub fn bitcoin_retarget(
    block_timestamp: i64,
    parent_timestamp: i64,
    target_block_time: i64,
    parent_difficulty: u64,
) -> u64 {
    let mut time_elapsed = block_timestamp - parent_timestamp;

    // Limit difficulty adjustment by factor of 2
    if time_elapsed < target_block_time / 2 {
        time_elapsed = target_block_time / 2
    } else if time_elapsed > target_block_time * 2 {
        time_elapsed = target_block_time * 2
    }

    let mut x: u64;
    x = match parent_difficulty.checked_mul(time_elapsed as u64) {
        Some(x) => x,
        None => u64::max_value(),
    };

    x /= target_block_time as u64;

    x
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
            return Err(ConsensusError::PedersenMerkleRoot(header.merkle_root_hash.to_string()));
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
        let verification_timer = start_timer!(|| "POSW verify");
        self.verifier
            .verify(header.nonce, &header.proof, &header.pedersen_merkle_root_hash)?;
        end_timer!(verification_timer);

        Ok(())
    }

    /// Check if the block is valid
    /// Check all outpoints, verify signatures, and calculate transaction fees.
    pub fn verify_block(
        &self,
        parameters: &<InstantiatedDPC as DPCScheme<MerkleTreeLedger>>::Parameters,
        block: &Block<Tx>,
        ledger: &MerkleTreeLedger,
    ) -> Result<bool, ConsensusError> {
        let transaction_ids: Vec<Vec<u8>> = block.transactions.to_transaction_ids()?;
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
        let mut total_value_balance = 0;

        for transaction in block.transactions.iter() {
            let value_balance = transaction.value_balance;

            if value_balance.is_negative() {
                coinbase_transaction_count += 1;
            }

            total_value_balance += value_balance;
        }

        // Check that there is only 1 coinbase transaction
        if coinbase_transaction_count > 1 {
            println!("multiple coinbase error");
            return Ok(false);
        }

        // Check that the block value balances are correct
        let expected_block_reward = get_block_reward(ledger.len() as u32) as i64;
        if total_value_balance + expected_block_reward != 0 {
            println!("total_value_balance: {:?}", total_value_balance);
            println!("expected_block_reward: {:?}", expected_block_reward);

            return Ok(false);
        }

        // Check that all the transction proofs verify
        Ok(InstantiatedDPC::verify_transactions(
            parameters,
            &block.transactions.0,
            ledger,
        )?)
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

        // 1. verify that the block valid
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

    /// Receive a block from an external source and process it based on ledger state
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
            if Self::is_genesis(&block.header) && storage.is_empty() {
                self.process_block(parameters, &storage, memory_pool, &block)?;
            } else {
                storage.insert_only(block)?;
            }
        } else {
            // Find the origin of the block
            match storage.get_block_path(&block.header)? {
                BlockPath::ExistingBlock => {}
                BlockPath::CanonChain(_) => {
                    self.process_block(parameters, &storage, memory_pool, block)?;

                    let (_, child_path) = storage.longest_child_path(block.header.get_hash())?;
                    for child_block_hash in child_path {
                        let new_block = storage.get_block(&child_block_hash)?;
                        self.process_block(parameters, &storage, memory_pool, &new_block)?;
                    }
                }
                BlockPath::SideChain(side_chain_path) => {
                    if side_chain_path.new_block_number > storage.get_latest_block_height() {
                        // Fork to superior chain
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
                        storage.insert_only(block)?;
                    }
                }
            };
        }

        Ok(())
    }

    /// Generate a coinbase transaction given candidate block transactions
    pub fn create_coinbase_transaction<R: Rng>(
        block_num: u32,
        transactions: &DPCTransactions<Tx>,
        parameters: &PublicParameters<Components>,
        predicate_vk_hash: &Vec<u8>,
        new_birth_predicates: Vec<DPCPredicate<Components>>,
        new_death_predicates: Vec<DPCPredicate<Components>>,
        recipient: AccountPublicKey<Components>,
        network_id: u8,
        ledger: &MerkleTreeLedger,
        rng: &mut R,
    ) -> Result<(Vec<DPCRecord<Components>>, Tx), ConsensusError> {
        let mut total_value_balance = get_block_reward(block_num);

        for transaction in transactions.iter() {
            let tx_value_balance = transaction.value_balance;

            if tx_value_balance.is_negative() {
                return Err(ConsensusError::CoinbaseTransactionAlreadyExists());
            }

            total_value_balance += transaction.value_balance.abs() as u64;
        }

        // Generate a new account that owns the dummy input records
        let account_metadata: [u8; 32] = rng.gen();
        let new_account = Account::new(
            &parameters.circuit_parameters.account_signature,
            &parameters.circuit_parameters.account_commitment,
            &account_metadata,
            rng,
        )
        .unwrap();

        // Generate dummy input records having as address the genesis address.
        let old_account_private_keys = vec![new_account.private_key.clone(); Components::NUM_INPUT_RECORDS];
        let mut old_records = vec![];
        for _ in 0..Components::NUM_INPUT_RECORDS {
            let sn_nonce_input: [u8; 4] = rng.gen();

            let old_sn_nonce =
                SerialNumberNonce::hash(&parameters.circuit_parameters.serial_number_nonce, &sn_nonce_input)?;

            let old_record = InstantiatedDPC::generate_record(
                &parameters.circuit_parameters,
                &old_sn_nonce,
                &new_account.public_key,
                true, // The input record is dummy
                0,
                &RecordPayload::default(),
                // Filler predicate input
                &Predicate::new(predicate_vk_hash.clone()),
                &Predicate::new(predicate_vk_hash.clone()),
                rng,
            )?;

            old_records.push(old_record);
        }

        let new_account_public_keys = vec![recipient.clone(); Components::NUM_OUTPUT_RECORDS];
        let new_dummy_flags = [vec![false], vec![true; Components::NUM_OUTPUT_RECORDS - 1]].concat();
        let new_values = [vec![total_value_balance], vec![0; Components::NUM_OUTPUT_RECORDS - 1]].concat();
        let new_payloads = vec![RecordPayload::default(); NUM_OUTPUT_RECORDS];

        let memo: [u8; 32] = rng.gen();

        Self::create_transaction(
            parameters,
            old_records,
            old_account_private_keys,
            new_account_public_keys,
            new_birth_predicates,
            new_death_predicates,
            new_dummy_flags,
            new_values,
            new_payloads,
            memo,
            network_id,
            ledger,
            rng,
        )
    }

    /// Generate a transaction by spending old records and specifying new record attributes
    pub fn create_transaction<R: Rng>(
        parameters: &<InstantiatedDPC as DPCScheme<MerkleTreeLedger>>::Parameters,
        old_records: Vec<DPCRecord<Components>>,
        old_account_private_keys: Vec<AccountPrivateKey<Components>>,
        new_account_public_keys: Vec<AccountPublicKey<Components>>,
        new_birth_predicates: Vec<DPCPredicate<Components>>,
        new_death_predicates: Vec<DPCPredicate<Components>>,
        new_dummy_flags: Vec<bool>,
        new_values: Vec<u64>,
        new_payloads: Vec<RecordPayload>,

        memo: [u8; 32],

        network_id: u8,

        ledger: &MerkleTreeLedger,
        rng: &mut R,
    ) -> Result<(Vec<DPCRecord<Components>>, Tx), ConsensusError> {
        let pred_nizk_pvk: PreparedVerifyingKey<_> =
            parameters.predicate_snark_parameters.verification_key.clone().into();

        let old_death_vk_and_proof_generator = |local_data: &LocalData<Components>| {
            let mut rng = thread_rng();
            let mut old_proof_and_vk = vec![];
            for i in 0..Components::NUM_INPUT_RECORDS {
                // Instantiate death predicate circuit
                let death_predicate_circuit = PredicateCircuit::new(
                    &local_data.circuit_parameters,
                    &local_data.local_data_commitment,
                    i as u8,
                );

                // Generate the predicate proof
                let proof = PredicateSNARK::prove(
                    &parameters.predicate_snark_parameters.proving_key,
                    death_predicate_circuit,
                    &mut rng,
                )
                .expect("Proving should work");
                #[cfg(debug_assertions)]
                {
                    let pred_pub_input: PredicateLocalData<Components> = PredicateLocalData {
                        local_data_commitment_parameters: local_data
                            .circuit_parameters
                            .local_data_commitment
                            .parameters()
                            .clone(),
                        local_data_commitment: local_data.local_data_commitment.clone(),
                        position: i as u8,
                    };
                    assert!(
                        PredicateSNARK::verify(&pred_nizk_pvk, &pred_pub_input, &proof).expect("Proof should verify")
                    );
                }

                let private_input: PrivatePredicateInput<Components> = PrivatePredicateInput {
                    verification_key: parameters.predicate_snark_parameters.verification_key.clone(),
                    proof,
                };
                old_proof_and_vk.push(private_input);
            }

            Ok(old_proof_and_vk)
        };

        let new_birth_vk_and_proof_generator = |local_data: &LocalData<Components>| {
            let mut rng = thread_rng();
            let mut new_proof_and_vk = vec![];
            for j in 0..NUM_OUTPUT_RECORDS {
                // Instantiate birth predicate circuit
                let birth_predicate_circuit = PredicateCircuit::new(
                    &local_data.circuit_parameters,
                    &local_data.local_data_commitment,
                    j as u8,
                );

                // Generate the predicate proof
                let proof = PredicateSNARK::prove(
                    &parameters.predicate_snark_parameters.proving_key,
                    birth_predicate_circuit,
                    &mut rng,
                )
                .expect("Proving should work");
                #[cfg(debug_assertions)]
                {
                    let pred_pub_input: PredicateLocalData<Components> = PredicateLocalData {
                        local_data_commitment_parameters: local_data
                            .circuit_parameters
                            .local_data_commitment
                            .parameters()
                            .clone(),
                        local_data_commitment: local_data.local_data_commitment.clone(),
                        position: j as u8,
                    };
                    assert!(
                        PredicateSNARK::verify(&pred_nizk_pvk, &pred_pub_input, &proof).expect("Proof should verify")
                    );
                }
                let private_input: PrivatePredicateInput<Components> = PrivatePredicateInput {
                    verification_key: parameters.predicate_snark_parameters.verification_key.clone(),
                    proof,
                };
                new_proof_and_vk.push(private_input);
            }

            Ok(new_proof_and_vk)
        };

        let (new_records, transaction) = InstantiatedDPC::execute(
            &parameters,
            &old_records,
            &old_account_private_keys,
            &old_death_vk_and_proof_generator,
            &new_account_public_keys,
            &new_dummy_flags,
            &new_values,
            &new_payloads,
            &new_birth_predicates,
            &new_death_predicates,
            &new_birth_vk_and_proof_generator,
            &memo,
            network_id,
            ledger,
            rng,
        )?;

        Ok((new_records, transaction))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use snarkos_objects::PedersenMerkleRootHash;

    use rand::SeedableRng;
    use rand_xorshift::XorShiftRng;

    #[test]
    fn verify_header() {
        let rng = &mut XorShiftRng::seed_from_u64(1234567);

        // mine a PoSW proof
        let posw = Posw::load().unwrap();
        let difficulty_target = u64::MAX;

        let consensus: ConsensusParameters = ConsensusParameters {
            max_block_size: 1_000_000usize,
            max_nonce: std::u32::MAX - 1,
            target_block_time: 2i64, //unix seconds
            network_id: 0,
            verifier: posw,
        };

        // mine PoSW for block 1
        let transaction_ids = vec![vec![1u8; 32]; 8];
        let (merkle_root_hash1, pedersen_merkle_root1, subroots1) = txids_to_roots(&transaction_ids);
        let (nonce1, proof1) = consensus
            .verifier
            .mine(&subroots1, difficulty_target, rng, std::u32::MAX)
            .unwrap();

        let h1 = BlockHeader {
            previous_block_hash: BlockHeaderHash([0; 32]),
            merkle_root_hash: merkle_root_hash1,
            pedersen_merkle_root_hash: pedersen_merkle_root1,
            nonce: nonce1,
            proof: proof1,
            difficulty_target,
            time: 9999999,
        };

        // mine PoSW for block 2
        let other_transaction_ids = vec![vec![2u8; 32]; 8];
        let (merkle_root_hash, pedersen_merkle_root, subroots) = txids_to_roots(&other_transaction_ids);
        let new_difficulty_target = consensus.get_block_difficulty(&h1, Utc::now().timestamp());
        let (nonce2, proof2) = consensus
            .verifier
            .mine(&subroots, new_difficulty_target, rng, std::u32::MAX)
            .unwrap();

        let h2 = BlockHeader {
            previous_block_hash: h1.get_hash(),
            merkle_root_hash: merkle_root_hash.clone(),
            pedersen_merkle_root_hash: pedersen_merkle_root.clone(),
            nonce: nonce2,
            proof: proof2,
            difficulty_target: new_difficulty_target,
            time: 9999999,
        };

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
        let mut h2_err = h2.clone();
        h2_err.difficulty_target = consensus.get_block_difficulty(&h1, Utc::now().timestamp()) + 1;
        consensus
            .verify_header(&h2_err, &h1, &merkle_root_hash, &pedersen_merkle_root)
            .unwrap_err();
    }
}
