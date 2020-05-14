use crate::{miner::MemoryPool, ConsensusParameters};
use snarkos_algorithms::{crh::sha256d_to_u64, merkle_tree::MerkleParameters, snark::create_random_proof};
use snarkos_dpc::{
    base_dpc::{instantiated::*, parameters::PublicParameters},
    DPCScheme,
};
use snarkos_errors::consensus::ConsensusError;
use snarkos_models::dpc::Record;
use snarkos_objects::{
    dpc::{Block, DPCTransactions, Transaction},
    merkle_root,
    pedersen_merkle_root,
    AccountPublicKey,
    BlockHeader,
    MerkleRootHash,
    ProofOfSuccinctWork,
};
use snarkos_posw::{ProvingKey, POSW};
use snarkos_profiler::{end_timer, start_timer};
use snarkos_storage::BlockStorage;
use snarkos_utilities::{
    bytes::{FromBytes, ToBytes},
    to_bytes,
};

use chrono::Utc;
use rand::{thread_rng, Rng};
use snarkos_dpc::dpc::base_dpc::record::DPCRecord;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Compiles transactions into blocks to be submitted to the network.
/// Uses a proof of work based algorithm to find valid blocks.
#[derive(Clone)]
pub struct Miner {
    /// Receiving address that block rewards will be sent to.
    address: AccountPublicKey<Components>,

    /// Parameters for current blockchain consensus.
    pub consensus: ConsensusParameters,

    pub proving_key: ProvingKey,
}

impl Miner {
    /// Returns a new instance of a miner with consensus params.
    pub fn new(address: AccountPublicKey<Components>, consensus: ConsensusParameters, proving_key: ProvingKey) -> Self {
        Self {
            address,
            consensus,
            proving_key,
        }
    }

    /// Fetches new transactions from the memory pool.
    pub async fn fetch_memory_pool_transactions<T: Transaction, P: MerkleParameters>(
        storage: &Arc<BlockStorage<T, P>>,
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
        let genesis_pred_vk_bytes = storage.genesis_pred_vk_bytes()?;
        let genesis_account = FromBytes::read(&storage.genesis_account_bytes()?[..])?;

        let new_predicate = Predicate::new(genesis_pred_vk_bytes.clone());
        let new_birth_predicates = vec![new_predicate.clone(); NUM_OUTPUT_RECORDS];
        let new_death_predicates = vec![new_predicate.clone(); NUM_OUTPUT_RECORDS];

        let (records, tx) = ConsensusParameters::create_coinbase_transaction(
            storage.get_latest_block_height() + 1,
            transactions,
            parameters,
            &genesis_pred_vk_bytes,
            new_birth_predicates,
            new_death_predicates,
            genesis_account,
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
        InstantiatedDPC::verify_transactions(parameters, &transactions.0, storage)?;

        let previous_block_header = storage.get_latest_block()?.header;

        Ok((previous_block_header, transactions, coinbase_records))
    }

    /// Run proof of work to find block.
    /// Returns BlockHeader with nonce solution.
    pub fn find_block<R: Rng>(
        &self,
        transactions: &DPCTransactions<Tx>,
        parent_header: &BlockHeader,
        rng: &mut R,
    ) -> Result<BlockHeader, ConsensusError> {
        let transaction_ids = transactions.to_transaction_ids()?;

        let mut merkle_root_bytes = [0u8; 32];
        merkle_root_bytes[..].copy_from_slice(&merkle_root(&transaction_ids));

        let pedersen_merkle_root = pedersen_merkle_root(&transaction_ids);

        let time = Utc::now().timestamp();
        let difficulty_target = self.consensus.get_block_difficulty(parent_header, time);

        let mut nonce;
        let mut proof;
        loop {
            nonce = rng.gen_range(0, self.consensus.max_nonce);
            proof = {
                // instantiate the circuit with the nonce
                let circuit = POSW::new(nonce, &transaction_ids);

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
            merkle_root_hash: MerkleRootHash(merkle_root_bytes),
            pedersen_merkle_root_hash: pedersen_merkle_root,
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
