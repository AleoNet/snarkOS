use crate::miner::MemoryPool;
use snarkos_dpc::base_dpc::{
    instantiated::*,
    parameters::PublicParameters,
    payment_circuit::PaymentCircuit,
    predicate::{DPCPredicate, PrivatePredicateInput},
    record::DPCRecord,
    record_payload::PaymentRecordPayload,
    BaseDPCComponents,
    LocalData,
};
use snarkos_errors::consensus::ConsensusError;
use snarkos_models::{
    algorithms::{CommitmentScheme, CRH, SNARK},
    curves::to_field_vec::ToConstraintField,
    dpc::{DPCComponents, DPCScheme, Record},
    objects::{Ledger, Transaction},
};
use snarkos_objects::{
    dpc::DPCTransactions,
    merkle_root_with_subroots,
    pedersen_merkle_root,
    Account,
    AccountPrivateKey,
    AccountPublicKey,
    Block,
    BlockHeader,
    BlockHeaderHash,
    MerkleRootHash,
    PedersenMerkleRootHash,
    ProofOfSuccinctWork,
    MASKED_TREE_HEIGHT,
};
use snarkos_utilities::{bytes::FromBytes, rand::UniformRand};

use snarkos_algorithms::snark::{prepare_verifying_key, verify_proof};
use snarkos_posw::{commit, Field, Proof, VerifyingKey};
use snarkos_profiler::{end_timer, start_timer};

use chrono::Utc;
use rand::{thread_rng, Rng};

#[cfg(debug_assertions)]
use snarkos_algorithms::snark::PreparedVerifyingKey;
#[cfg(debug_assertions)]
use snarkos_dpc::base_dpc::payment_circuit::PaymentPredicateLocalData;

pub const TWO_HOURS_UNIX: i64 = 7200;

/// Parameters for a proof of work blockchain.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConsensusParameters {
    /// Maximum block size in bytes
    pub max_block_size: usize,

    /// Maximum nonce value allowed
    pub max_nonce: u32,

    /// The amount of time it should take to find a block
    pub target_block_time: i64,

    // /// Mainnet or testnet
    // network: Network
    /// The verifying key for the PoSW Merkle Tree SNARK
    pub verifying_key: VerifyingKey,
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
        let future_timelimit: i64 = Utc::now().timestamp() as i64 + TWO_HOURS_UNIX;

        // Verify the proof
        let verification_timer = start_timer!(|| "POSW verify");
        self.verify_proof(header.nonce, &header.proof, &header.pedersen_merkle_root_hash)?;
        end_timer!(verification_timer);

        if parent_header.get_hash() != header.previous_block_hash {
            Err(ConsensusError::NoParent(
                parent_header.get_hash().to_string(),
                header.previous_block_hash.to_string(),
            ))
        } else if header.merkle_root_hash != *merkle_root_hash {
            Err(ConsensusError::MerkleRoot(header.merkle_root_hash.to_string()))
        } else if header.pedersen_merkle_root_hash != *pedersen_merkle_root_hash {
            Err(ConsensusError::PedersenMerkleRoot(header.merkle_root_hash.to_string()))
        } else if header.time > future_timelimit {
            Err(ConsensusError::FuturisticTimestamp(future_timelimit, header.time))
        } else if header.time < parent_header.time {
            Err(ConsensusError::TimestampInvalid(header.time, parent_header.time))
        } else if hash_result > header.difficulty_target {
            Err(ConsensusError::PowInvalid(header.difficulty_target, hash_result))
        } else if header.nonce >= self.max_nonce {
            Err(ConsensusError::NonceInvalid(header.nonce, self.max_nonce))
        } else {
            Ok(())
        }
    }

    fn verify_proof(
        &self,
        nonce: u32,
        proof: &ProofOfSuccinctWork,
        pedersen_merkle_root: &PedersenMerkleRootHash,
    ) -> Result<(), ConsensusError> {
        let mask = commit(nonce, pedersen_merkle_root.clone());
        let merkle_root = Field::read(&pedersen_merkle_root.0[..])?;
        let inputs = [ToConstraintField::<Field>::to_field_elements(&mask[..])?, vec![
            merkle_root,
        ]]
        .concat();

        // deserialize the snark proof
        let proof = Proof::read(&proof.0[..])?;

        let res = verify_proof(&prepare_verifying_key(&self.verifying_key), &proof, &inputs)?;
        if !res {
            return Err(ConsensusError::PoswVerificationFailed);
        }

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

        let (root, subroots) = merkle_root_with_subroots(&transaction_ids, MASKED_TREE_HEIGHT);
        let mut merkle_root_bytes = [0u8; 32];
        merkle_root_bytes[..].copy_from_slice(&root);

        let pedersen_merkle_root = pedersen_merkle_root(&subroots);

        // Verify the block header
        if !Self::is_genesis(&block.header) {
            let parent_block = ledger.get_latest_block()?;
            if let Err(err) = self.verify_header(
                &block.header,
                &parent_block.header,
                &MerkleRootHash(merkle_root_bytes),
                &pedersen_merkle_root,
            ) {
                println!("header failed to verify: {:?}", err);
                return Ok(false);
            }
        }
        // Verify block amounts and check that there is a single coinbase transaction

        let mut coinbase_transaction_count = 0;
        let mut total_value_balance = 0;

        for transaction in block.transactions.iter() {
            let value_balance = transaction.stuff().value_balance;

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
    /// TODO 4. Check cached blocks to insert/canonize.
    pub fn process_block(
        &self,
        parameters: &PublicParameters<Components>,
        storage: &MerkleTreeLedger,
        memory_pool: &mut MemoryPool<Tx>,
        block: &Block<Tx>,
    ) -> Result<(), ConsensusError> {
        // 1. verify that the block valid
        if !self.verify_block(parameters, block, storage)? {
            return Err(ConsensusError::InvalidBlock(block.header.get_hash().0.to_vec()));
        }

        // 2. Insert/canonize block
        storage.insert_block(block)?;

        // 3. Remove transactions from the mempool
        for transaction_id in block.transactions.to_transaction_ids()? {
            memory_pool.remove_by_hash(&transaction_id)?;
        }

        Ok(())
    }

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

        if !storage.block_hash_exists(&block.header.get_hash()) {
            self.process_block(parameters, &storage, memory_pool, &block)?;
        }

        Ok(())
    }

    pub fn create_coinbase_transaction<R: Rng>(
        block_num: u32,
        transactions: &DPCTransactions<Tx>,
        parameters: &PublicParameters<Components>,
        genesis_pred_vk_bytes: &Vec<u8>,
        new_birth_predicates: Vec<DPCPredicate<Components>>,
        new_death_predicates: Vec<DPCPredicate<Components>>,
        genesis_account: Account<Components>,
        recipient: AccountPublicKey<Components>,
        ledger: &MerkleTreeLedger,
        rng: &mut R,
    ) -> Result<(Vec<DPCRecord<Components>>, Tx), ConsensusError> {
        let mut total_value_balance = get_block_reward(block_num);

        for transaction in transactions.iter() {
            let tx_value_balance = transaction.stuff.value_balance;

            if tx_value_balance.is_negative() {
                return Err(ConsensusError::CoinbaseTransactionAlreadyExists());
            }

            total_value_balance += transaction.stuff.value_balance.abs() as u64;
        }

        // Generate dummy input records having as address the genesis address.
        let old_account_private_keys = vec![genesis_account.private_key.clone(); Components::NUM_INPUT_RECORDS];
        let mut old_records = vec![];
        for _ in 0..Components::NUM_INPUT_RECORDS {
            let sn_nonce_input: [u8; 4] = rng.gen();

            let old_sn_nonce =
                SerialNumberNonce::hash(&parameters.circuit_parameters.serial_number_nonce, &sn_nonce_input)?;

            let old_record = InstantiatedDPC::generate_record(
                &parameters.circuit_parameters,
                &old_sn_nonce,
                &genesis_account.public_key,
                true, // The input record is dummy
                &PaymentRecordPayload::default(),
                // Filler predicate input
                &Predicate::new(genesis_pred_vk_bytes.clone()),
                &Predicate::new(genesis_pred_vk_bytes.clone()),
                rng,
            )?;

            old_records.push(old_record);
        }

        let new_payload = PaymentRecordPayload {
            balance: total_value_balance,
            lock: 0,
        };
        let dummy_payload = PaymentRecordPayload { balance: 0, lock: 0 };

        let new_account_public_keys = vec![recipient.clone(); Components::NUM_OUTPUT_RECORDS];
        let new_dummy_flags = [vec![false], vec![true; Components::NUM_OUTPUT_RECORDS - 1]].concat();
        let new_payloads = [vec![new_payload], vec![
            dummy_payload;
            Components::NUM_OUTPUT_RECORDS - 1
        ]]
        .concat();

        let auxiliary: [u8; 32] = rng.gen();
        let memo: [u8; 32] = rng.gen();

        Self::create_transaction(
            parameters,
            old_records,
            old_account_private_keys,
            new_account_public_keys,
            new_birth_predicates,
            new_death_predicates,
            new_dummy_flags,
            new_payloads,
            auxiliary,
            memo,
            ledger,
            rng,
        )
    }

    pub fn create_transaction<R: Rng>(
        parameters: &<InstantiatedDPC as DPCScheme<MerkleTreeLedger>>::Parameters,
        old_records: Vec<DPCRecord<Components>>,
        old_account_private_keys: Vec<AccountPrivateKey<Components>>,
        new_account_public_keys: Vec<AccountPublicKey<Components>>,
        new_birth_predicates: Vec<DPCPredicate<Components>>,
        new_death_predicates: Vec<DPCPredicate<Components>>,
        new_dummy_flags: Vec<bool>,
        new_payloads: Vec<PaymentRecordPayload>,

        auxiliary: [u8; 32],
        memo: [u8; 32],

        ledger: &MerkleTreeLedger,
        rng: &mut R,
    ) -> Result<(Vec<DPCRecord<Components>>, Tx), ConsensusError> {
        #[cfg(debug_assertions)]
        let pred_nizk_pvk: PreparedVerifyingKey<_> =
            parameters.predicate_snark_parameters.verification_key.clone().into();

        let old_death_vk_and_proof_generator = |local_data: &LocalData<Components>| {
            let mut rng = thread_rng();
            let mut old_proof_and_vk = vec![];
            for i in 0..Components::NUM_INPUT_RECORDS {
                // If the record is a dummy, then the value should be 0
                let input_value = match local_data.old_records[i].is_dummy() {
                    true => 0,
                    false => local_data.old_records[i].payload().balance,
                };

                // Generate value commitment randomness
                let value_commitment_randomness =
                    <<Components as BaseDPCComponents>::ValueCommitment as CommitmentScheme>::Randomness::rand(
                        &mut rng,
                    );

                // Generate the value commitment
                let value_commitment = local_data
                    .circuit_parameters
                    .value_commitment
                    .commit(&input_value.to_le_bytes(), &value_commitment_randomness)?;

                // Instantiate death predicate circuit
                let death_predicate_circuit = PaymentCircuit::new(
                    &local_data.circuit_parameters,
                    &local_data.local_data_commitment,
                    &value_commitment_randomness,
                    &value_commitment,
                    i as u8,
                    input_value,
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
                    let pred_pub_input: PaymentPredicateLocalData<Components> = PaymentPredicateLocalData {
                        local_data_commitment_parameters: local_data
                            .circuit_parameters
                            .local_data_commitment
                            .parameters()
                            .clone(),
                        local_data_commitment: local_data.local_data_commitment.clone(),
                        value_commitment_parameters: local_data
                            .circuit_parameters
                            .value_commitment
                            .parameters()
                            .clone(),
                        value_commitment_randomness: value_commitment_randomness.clone(),
                        value_commitment: value_commitment.clone(),
                        position: i as u8,
                    };
                    assert!(
                        PredicateSNARK::verify(&pred_nizk_pvk, &pred_pub_input, &proof).expect("Proof should verify")
                    );
                }

                let private_input: PrivatePredicateInput<Components> = PrivatePredicateInput {
                    verification_key: parameters.predicate_snark_parameters.verification_key.clone(),
                    proof,
                    value_commitment,
                    value_commitment_randomness,
                };
                old_proof_and_vk.push(private_input);
            }

            Ok(old_proof_and_vk)
        };

        let new_birth_vk_and_proof_generator = |local_data: &LocalData<Components>| {
            let mut rng = thread_rng();
            let mut new_proof_and_vk = vec![];
            for j in 0..NUM_OUTPUT_RECORDS {
                // If the record is a dummy, then the value should be 0
                let output_value = match local_data.new_records[j].is_dummy() {
                    true => 0,
                    false => local_data.new_records[j].payload().balance,
                };

                // Generate value commitment randomness
                let value_commitment_randomness =
                    <<Components as BaseDPCComponents>::ValueCommitment as CommitmentScheme>::Randomness::rand(
                        &mut rng,
                    );

                // Generate the value commitment
                let value_commitment = local_data
                    .circuit_parameters
                    .value_commitment
                    .commit(&output_value.to_le_bytes(), &value_commitment_randomness)?;

                // Instantiate birth predicate circuit
                let birth_predicate_circuit = PaymentCircuit::new(
                    &local_data.circuit_parameters,
                    &local_data.local_data_commitment,
                    &value_commitment_randomness,
                    &value_commitment,
                    j as u8,
                    output_value,
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
                    let pred_pub_input: PaymentPredicateLocalData<Components> = PaymentPredicateLocalData {
                        local_data_commitment_parameters: local_data
                            .circuit_parameters
                            .local_data_commitment
                            .parameters()
                            .clone(),
                        local_data_commitment: local_data.local_data_commitment.clone(),
                        value_commitment_parameters: local_data
                            .circuit_parameters
                            .value_commitment
                            .parameters()
                            .clone(),
                        value_commitment_randomness: value_commitment_randomness.clone(),
                        value_commitment: value_commitment.clone(),
                        position: j as u8,
                    };
                    assert!(
                        PredicateSNARK::verify(&pred_nizk_pvk, &pred_pub_input, &proof).expect("Proof should verify")
                    );
                }
                let private_input: PrivatePredicateInput<Components> = PrivatePredicateInput {
                    verification_key: parameters.predicate_snark_parameters.verification_key.clone(),
                    proof,
                    value_commitment,
                    value_commitment_randomness,
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
            &new_payloads,
            &new_birth_predicates,
            &new_death_predicates,
            &new_birth_vk_and_proof_generator,
            &auxiliary,
            &memo,
            ledger,
            rng,
        )?;

        Ok((new_records, transaction))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_header() {
        let consensus: ConsensusParameters = ConsensusParameters {
            max_block_size: 1_000_000usize,
            max_nonce: 1000,
            target_block_time: 2i64, //unix seconds
        };

        let h1 = BlockHeader {
            previous_block_hash: BlockHeaderHash([0; 32]),
            merkle_root_hash: MerkleRootHash([1; 32]),
            difficulty_target: u64::MAX,
            nonce: 100,
            time: 9999999,
        };
        let h1_clone = h1.clone();

        let merkle_root_hash = MerkleRootHash([2; 32]);
        let h2 = BlockHeader {
            previous_block_hash: h1.get_hash(),
            merkle_root_hash: merkle_root_hash.clone(),
            ..h1_clone
        };

        // OK
        consensus.verify_header(&h2, &h1, &merkle_root_hash).unwrap();

        // invalid parent hash
        let mut h2_err = h2.clone();
        h2_err.previous_block_hash = BlockHeaderHash([9; 32]);
        consensus.verify_header(&h2_err, &h1, &merkle_root_hash).unwrap_err();

        // invalid merkle root hash
        let mut h2_err = h2.clone();
        h2_err.merkle_root_hash = MerkleRootHash([3; 32]);
        consensus.verify_header(&h2_err, &h1, &merkle_root_hash).unwrap_err();

        // past block
        let mut h2_err = h2.clone();
        h2_err.time = 100;
        consensus.verify_header(&h2_err, &h1, &merkle_root_hash).unwrap_err();

        // far in the future block
        let mut h2_err = h2.clone();
        h2_err.time = Utc::now().timestamp() as i64 + 7201;
        consensus.verify_header(&h2_err, &h1, &merkle_root_hash).unwrap_err();

        // invalid difficulty
        let mut h2_err = h2.clone();
        h2_err.difficulty_target = 100; // set the difficulty very very high
        consensus.verify_header(&h2_err, &h1, &merkle_root_hash).unwrap_err();

        // invalid nonce
        let mut h2_err = h2.clone();
        h2_err.nonce = 1001; // over the max nonce
        consensus.verify_header(&h2_err, &h1, &merkle_root_hash).unwrap_err();
    }
}
