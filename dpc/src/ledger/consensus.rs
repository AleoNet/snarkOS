use crate::{
    base_dpc::{
        instantiated::*,
        payment_circuit::{PaymentCircuit, PaymentPredicateLocalData},
        predicate::{DPCPredicate, PrivatePredicateInput},
        record::DPCRecord,
        record_payload::PaymentRecordPayload,
        BaseDPCComponents,
        LocalData,
    },
    dpc::{
        address::{AddressPair, AddressPublicKey, AddressSecretKey},
        Transaction,
    },
    ledger::{block::Block, transactions::Transactions, Ledger},
    DPCScheme,
    Record,
};

use snarkos_algorithms::snark::PreparedVerifyingKey;
use snarkos_errors::consensus::ConsensusError;
use snarkos_models::{
    algorithms::{CommitmentScheme, CRH, SNARK},
    dpc::DPCComponents,
};
use snarkos_objects::{merkle_root, BlockHeader, BlockHeaderHash, MerkleRootHash};
use snarkos_utilities::rand::UniformRand;

use rand::{thread_rng, Rng};
use std::{
    marker::PhantomData,
    time::{SystemTime, UNIX_EPOCH},
};

pub const TWO_HOURS_UNIX: i64 = 7200;

/// Parameters for a proof of work blockchain.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConsensusParameters<L: Ledger> {
    /// Maximum block size in bytes
    pub max_block_size: usize,

    /// Maximum nonce value allowed
    pub max_nonce: u32,

    /// The amount of time it should take to find a block
    pub target_block_time: i64,

    // /// mainnet or testnet
    //    network: Network
    _ledger: PhantomData<L>,
}

/// Calculate a block reward that halves every 1000 blocks.
pub fn block_reward(block_num: u32) -> u64 {
    100_000_000u64 / (2_u64.pow(block_num / 1000))
}

impl<L: Ledger> ConsensusParameters<L> {
    /// Calculate the difficulty for the next block based off how long it took to mine the last one.
    pub fn get_block_difficulty(&self, prev_header: &BlockHeader, _block_timestamp: i64) -> u64 {
        //        bitcoin_retarget(
        //            block_timestamp,
        //            prev_header.time,
        //            self.target_block_time,
        //            prev_header.difficulty_target,
        //        )

        prev_header.difficulty_target
    }

    pub fn is_genesis(block: &Block<Tx>) -> bool {
        block.header.previous_block_hash == BlockHeaderHash([0u8; 32])
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
    ) -> Result<(), ConsensusError> {
        let hash_result = header.to_difficulty_hash();

        let since_the_epoch = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();

        let future_timelimit: i64 = since_the_epoch as i64 + TWO_HOURS_UNIX;

        if parent_header.get_hash() != header.previous_block_hash {
            Err(ConsensusError::NoParent(
                parent_header.get_hash().to_string(),
                header.previous_block_hash.to_string(),
            ))
        } else if header.merkle_root_hash != *merkle_root_hash {
            Err(ConsensusError::MerkleRoot(header.merkle_root_hash.to_string()))
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

    /// Check if the block is valid
    /// Check all outpoints, verify signatures, and calculate transaction fees.
    pub fn valid_block(
        &self,
        parameters: &<InstantiatedDPC as DPCScheme<MerkleTreeLedger>>::Parameters,
        block: &Block<Tx>,
        ledger: &L,
    ) -> Result<bool, ConsensusError> {
        //        if let Err(_) = self.verify_header() {
        //            return Ok(false);
        //        }

        // Verify block amounts and check that there is a single coinbase

        for transaction in block.transactions.iter() {
            // run dpc verify
            let _x = transaction.stuff();
        }

        // Verify that the transactions are valid in the ledger
        //
        //        Ok(InstantiatedDPC::verify_block(parameters, block, ledger)?)

        Ok(true)
    }

    /// Verifies that the block header is valid.
    pub fn valid_block_header(&self, block: &Block<Tx>) -> Result<(), ConsensusError> {
        let mut merkle_root_slice = [0u8; 32];
        merkle_root_slice.copy_from_slice(&merkle_root(&block.transactions.to_transaction_ids()?));
        let _merkle_root_hash = &MerkleRootHash(merkle_root_slice);

        // Do not verify headers of genesis blocks
        //        if !Self::is_genesis(block) {
        //            let parent_block = storage.get_latest_block()?;
        //            self.verify_header(&block.header, &parent_block.header, merkle_root_hash)?;
        //        }

        // Check not genesis
        // Add
        Ok(())
    }

    /// Return whether or not the given block is valid and insert it.
    /// 1. Verify that the block header is valid.
    /// 2. Verify that the transactions are valid.
    /// 3. Insert/canonize block.
    /// 4. Check cached blocks to insert/canonize.
    pub fn process_block(
        &self,
        //        storage: &BlockStorage,
        //        memory_pool: &mut MemoryPool,
        _block: &Block<Tx>,
    ) -> Result<u32, ConsensusError> {
        //        let total_balance = 0;

        Ok(0)
    }

    pub fn create_coinbase_transaction<R: Rng>(
        block_num: u32,
        transactions: &Transactions<Tx>,
        parameters: &<InstantiatedDPC as DPCScheme<MerkleTreeLedger>>::Parameters,
        genesis_pred_vk_bytes: &Vec<u8>,
        new_birth_predicates: Vec<DPCPredicate<Components>>,
        new_death_predicates: Vec<DPCPredicate<Components>>,
        genesis_address: AddressPair<Components>,
        recipient: AddressPublicKey<Components>,

        ledger: &MerkleTreeLedger,
        rng: &mut R,
    ) -> Result<(Vec<DPCRecord<Components>>, Tx), ConsensusError> {
        let mut total_value_balance = block_reward(block_num);

        for transaction in transactions.iter() {
            let tx_value_balance = transaction.stuff.value_balance;

            if tx_value_balance.is_negative() {
                return Err(ConsensusError::CoinbaseTransactionAlreadyExists());
            }

            total_value_balance += transaction.stuff.value_balance.abs() as u64;
        }

        // Generate dummy input records having as address the genesis address.
        let old_asks = vec![genesis_address.secret_key.clone(); Components::NUM_INPUT_RECORDS];
        let mut old_records = vec![];
        for i in 0..Components::NUM_INPUT_RECORDS {
            let old_sn_nonce = SerialNumberNonce::hash(
                &parameters.circuit_parameters.serial_number_nonce_parameters,
                &[64u8 + (i as u8); 1],
            )?;

            let old_record = InstantiatedDPC::generate_record(
                &parameters.circuit_parameters,
                &old_sn_nonce,
                &genesis_address.public_key,
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
            lock: block_num,
        };
        let dummy_payload = PaymentRecordPayload { balance: 0, lock: 0 };

        let new_apks = vec![recipient.clone(); Components::NUM_OUTPUT_RECORDS];
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
            old_asks,
            new_apks,
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
        old_asks: Vec<AddressSecretKey<Components>>,
        new_apks: Vec<AddressPublicKey<Components>>,
        new_birth_predicates: Vec<DPCPredicate<Components>>,
        new_death_predicates: Vec<DPCPredicate<Components>>,
        new_dummy_flags: Vec<bool>,
        new_payloads: Vec<PaymentRecordPayload>,

        auxiliary: [u8; 32],
        memo: [u8; 32],

        ledger: &MerkleTreeLedger,
        rng: &mut R,
    ) -> Result<(Vec<DPCRecord<Components>>, Tx), ConsensusError> {
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
                    .value_commitment_parameters
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
                            .local_data_commitment_parameters
                            .parameters()
                            .clone(),
                        local_data_commitment: local_data.local_data_commitment.clone(),
                        value_commitment_parameters: local_data
                            .circuit_parameters
                            .value_commitment_parameters
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
                    .value_commitment_parameters
                    .commit(&output_value.to_le_bytes(), &value_commitment_randomness)
                    .unwrap();

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
                            .local_data_commitment_parameters
                            .parameters()
                            .clone(),
                        local_data_commitment: local_data.local_data_commitment.clone(),
                        value_commitment_parameters: local_data
                            .circuit_parameters
                            .value_commitment_parameters
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
            &old_asks,
            &old_death_vk_and_proof_generator,
            &new_apks,
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

    // TODO:
    // Coinbase transaction (make sure the sum of block's value balances = -block_reward).
    // Block selection/ confirmation
    //   - Make sure sum of vb adds up. Make sure the nonce is correct.
}
