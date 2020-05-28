use crate::{miner::MemoryPool, txids_to_roots, ConsensusParameters, POSWVerifier};
use snarkos_algorithms::{crh::sha256d_to_u64, merkle_tree::MerkleParameters, snark::create_random_proof};
use snarkos_dpc::base_dpc::{instantiated::*, parameters::PublicParameters};
use snarkos_errors::consensus::ConsensusError;
use snarkos_models::{
    algorithms::CRH,
    dpc::{DPCScheme, Record},
    objects::Transaction,
};
use snarkos_objects::{dpc::DPCTransactions, AccountPublicKey, Block, BlockHeader, ProofOfSuccinctWork};
use snarkos_posw::{ProvingKey, POSW};
use snarkos_profiler::{end_timer, start_timer};
use snarkos_storage::Ledger;
use snarkos_utilities::{bytes::ToBytes, to_bytes};

use chrono::Utc;
use rand::{thread_rng, Rng};
use snarkos_dpc::dpc::base_dpc::record::DPCRecord;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Compiles transactions into blocks to be submitted to the network.
/// Uses a proof of work based algorithm to find valid blocks.
#[derive(Clone)]
pub struct Miner<V> {
    /// Receiving address that block rewards will be sent to.
    address: AccountPublicKey<Components>,

    /// Parameters for current blockchain consensus.
    pub consensus: ConsensusParameters<V>,

    pub proving_key: ProvingKey,
}

impl<V: POSWVerifier> Miner<V> {
    /// Returns a new instance of a miner with consensus params.
    pub fn new(
        address: AccountPublicKey<Components>,
        consensus: ConsensusParameters<V>,
        proving_key: ProvingKey,
    ) -> Self {
        Self {
            address,
            consensus,
            proving_key,
        }
    }

    /// Fetches new transactions from the memory pool.
    pub async fn fetch_memory_pool_transactions<T: Transaction, P: MerkleParameters>(
        storage: &Arc<Ledger<T, P>>,
        memory_pool: &Arc<Mutex<MemoryPool<T>>>,
        max_size: usize,
    ) -> Result<DPCTransactions<T>, ConsensusError> {
        let memory_pool = memory_pool.lock().await;
        Ok(memory_pool.get_candidates(&storage, max_size)?)
    }

    pub fn add_coinbase_transaction<R: Rng>(
        &self,
        parameters: &PublicParameters<Components>,
        storage: &MerkleTreeLedger,
        transactions: &mut DPCTransactions<Tx>,
        rng: &mut R,
    ) -> Result<Vec<DPCRecord<Components>>, ConsensusError> {
        let predicate_vk_hash = to_bytes![PredicateVerificationKeyHash::hash(
            &parameters.circuit_parameters.predicate_verification_key_hash,
            &to_bytes![parameters.predicate_snark_parameters.verification_key]?
        )?]?;

        let new_predicate = Predicate::new(predicate_vk_hash.clone());
        let new_birth_predicates = vec![new_predicate.clone(); NUM_OUTPUT_RECORDS];
        let new_death_predicates = vec![new_predicate.clone(); NUM_OUTPUT_RECORDS];

        let (records, tx) = ConsensusParameters::<V>::create_coinbase_transaction(
            storage.get_latest_block_height() + 1,
            transactions,
            parameters,
            &predicate_vk_hash,
            new_birth_predicates,
            new_death_predicates,
            self.address.clone(),
            &storage,
            rng,
        )?;

        transactions.push(tx);
        Ok(records)
    }

    /// Acquires the storage lock and returns the previous block header and verified transactions.
    pub fn establish_block(
        &self,
        parameters: &PublicParameters<Components>,
        storage: &MerkleTreeLedger,
        transactions: &DPCTransactions<Tx>,
    ) -> Result<(BlockHeader, DPCTransactions<Tx>, Vec<DPCRecord<Components>>), ConsensusError> {
        let rng = &mut thread_rng();
        let mut transactions = transactions.clone();
        let coinbase_records = self.add_coinbase_transaction(parameters, &storage, &mut transactions, rng)?;

        // Verify transactions
        assert!(InstantiatedDPC::verify_transactions(
            parameters,
            &transactions.0,
            storage
        )?);

        let previous_block_header = storage.get_latest_block()?.header;

        Ok((previous_block_header, transactions, coinbase_records))
    }

    /// Run proof of work to find block.
    /// Returns BlockHeader with nonce solution.
    pub fn find_block<R: Rng, T: Transaction>(
        &self,
        transactions: &DPCTransactions<T>,
        parent_header: &BlockHeader,
        rng: &mut R,
    ) -> Result<BlockHeader, ConsensusError> {
        let transaction_ids = transactions.to_transaction_ids()?;
        let (merkle_root_hash, pedersen_merkle_root_hash, subroots) = txids_to_roots(&transaction_ids);

        let time = Utc::now().timestamp();
        let difficulty_target = self.consensus.get_block_difficulty(parent_header, time);

        let mut nonce;
        let mut proof;
        loop {
            nonce = rng.gen_range(0, self.consensus.max_nonce);
            proof = {
                // instantiate the circuit with the nonce
                let circuit = POSW::new(nonce, &subroots);

                // generate the proof
                let proof_timer = start_timer!(|| "POSW proof");
                let proof = create_random_proof(circuit, &self.proving_key, rng)?;
                end_timer!(proof_timer);

                // serialize it
                let proof_bytes = to_bytes![proof]?;
                let mut p = [0; ProofOfSuccinctWork::size()];
                p.copy_from_slice(&proof_bytes);
                ProofOfSuccinctWork(p)
            };

            // Hash the proof and parse it as a u64
            // TODO: replace u64 with bigint
            let hash_result = sha256d_to_u64(&proof.0[..]);

            // if it passes the difficulty chec, use the proof/nonce pairs and return the header
            if hash_result <= difficulty_target {
                break;
            }
        }

        let header = BlockHeader {
            merkle_root_hash,
            pedersen_merkle_root_hash,
            previous_block_hash: parent_header.get_hash(),
            time,
            difficulty_target,
            nonce,
            proof,
        };

        Ok(header)
    }

    /// Returns a mined block.
    /// Calls methods to fetch transactions, run proof of work, and add the block into the chain for storage.
    pub async fn mine_block<R: Rng>(
        &self,
        parameters: &PublicParameters<Components>,
        storage: &Arc<MerkleTreeLedger>,
        memory_pool: &Arc<Mutex<MemoryPool<Tx>>>,
        rng: &mut R,
    ) -> Result<(Vec<u8>, Vec<DPCRecord<Components>>), ConsensusError> {
        let mut candidate_transactions =
            Self::fetch_memory_pool_transactions(&storage.clone(), memory_pool, self.consensus.max_block_size).await?;

        println!("Miner creating block");

        let (previous_block_header, transactions, coinbase_records) =
            self.establish_block(parameters, storage, &mut candidate_transactions)?;

        println!("Miner generated coinbase transaction");

        for (index, record) in coinbase_records.iter().enumerate() {
            let record_commitment = hex::encode(&to_bytes![record.commitment()]?);
            println!("Coinbase record {:?} commitment: {:?}", index, record_commitment);
        }

        let header = self.find_block(&transactions, &previous_block_header, rng)?;

        println!("Miner found block block");

        let block = Block { header, transactions };

        let mut memory_pool = memory_pool.lock().await;

        self.consensus
            .receive_block(parameters, storage, &mut memory_pool, &block)?;

        storage.store_records(&coinbase_records)?;

        Ok((block.serialize()?, coinbase_records))
    }
}

#[cfg(test)]
mod tests {
    use crate::{miner::Miner, test_data::*, txids_to_roots};
    use rand::{Rng, SeedableRng};
    use rand_xorshift::XorShiftRng;
    use snarkos_models::{
        algorithms::{commitment::CommitmentScheme, signature::SignatureScheme},
        dpc::DPCComponents,
    };
    use snarkos_objects::{dpc::DPCTransactions, AccountPrivateKey, AccountPublicKey, BlockHeader};

    fn keygen<C: DPCComponents, R: Rng>(rng: &mut R) -> (AccountPrivateKey<C>, AccountPublicKey<C>) {
        let sig_params = C::AccountSignature::setup(rng).unwrap();
        let comm_params = C::AccountCommitment::setup(rng);

        let key = AccountPrivateKey::<C>::new(&sig_params, &[0; 32], rng).unwrap();
        let pubkey = AccountPublicKey::from(&comm_params, &key).unwrap();

        (key, pubkey)
    }

    // this test ensures that a block is found by running the proof of work
    // and that it doesnt loop forever
    fn test_find_block(transactions: &DPCTransactions<TestTx>, parent_header: &BlockHeader) {
        let consensus = TEST_CONSENSUS.clone();
        let mut rng = XorShiftRng::seed_from_u64(3); // use this rng so that a valid solution is found quickly

        let (_, miner_address) = keygen(&mut rng);
        let miner = Miner::new(miner_address, consensus.clone(), POSW_PP.0.clone());

        let header = miner.find_block(transactions, parent_header, &mut rng).unwrap();
        // assert_eq!(header.nonce, 3146114823);

        // generate the verifier args
        let (merkle_root, pedersen_merkle_root, _) = txids_to_roots(&transactions.to_transaction_ids().unwrap());

        // ensure that our POSW proof passes
        consensus
            .verify_header(&header, parent_header, &merkle_root, &pedersen_merkle_root)
            .unwrap();
    }

    #[test]
    fn find_valid_block() {
        let transactions = DPCTransactions(vec![TestTx; 3]);
        let parent_header = genesis().header;
        test_find_block(&transactions, &parent_header);
    }
}
