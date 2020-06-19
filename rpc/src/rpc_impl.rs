use crate::{rpc_types::*, RpcFunctions};
use snarkos_consensus::{get_block_reward, ConsensusParameters, MemoryPool, MerkleTreeLedger};
use snarkos_dpc::base_dpc::{
    instantiated::{Components, InstantiatedDPC, Predicate, Tx},
    parameters::PublicParameters,
    record::DPCRecord,
    record_payload::RecordPayload,
};
use snarkos_errors::rpc::RpcError;
use snarkos_models::{
    algorithms::CRH,
    dpc::{DPCComponents, Record},
    objects::Transaction,
};
use snarkos_network::{context::Context, process_transaction_internal};
use snarkos_objects::{AccountPrivateKey, AccountPublicKey, BlockHeaderHash};
use snarkos_utilities::{
    bytes::{FromBytes, ToBytes},
    to_bytes,
    CanonicalSerialize,
};

use chrono::Utc;
use rand::thread_rng;
use std::sync::Arc;
use tokio::{runtime::Runtime, sync::Mutex};

/// Implements JSON-RPC HTTP endpoint functions for a node.
/// The constructor is given Arc::clone() copies of all needed node components.
pub struct RpcImpl {
    /// Blockchain database storage.
    storage: Arc<MerkleTreeLedger>,

    /// Public Parameters
    parameters: PublicParameters<Components>,

    /// Network context held by the server.
    server_context: Arc<Context>,

    /// Consensus parameters generated from node config.
    consensus: ConsensusParameters,

    /// Handle to access the memory pool of transactions.
    memory_pool_lock: Arc<Mutex<MemoryPool<Tx>>>,
}

impl RpcImpl {
    pub fn new(
        storage: Arc<MerkleTreeLedger>,
        parameters: PublicParameters<Components>,
        server_context: Arc<Context>,
        consensus: ConsensusParameters,
        memory_pool_lock: Arc<Mutex<MemoryPool<Tx>>>,
    ) -> Self {
        Self {
            storage,
            parameters,
            server_context,
            consensus,
            memory_pool_lock,
        }
    }
}

impl RpcFunctions for RpcImpl {
    /// Returns information about a block from a block hash.
    fn get_block(&self, block_hash_string: String) -> Result<BlockInfo, RpcError> {
        let block_hash = hex::decode(&block_hash_string)?;
        assert_eq!(block_hash.len(), 32);

        let block_header_hash = BlockHeaderHash::new(block_hash);
        let height = match self.storage.get_block_num(&block_header_hash) {
            Ok(block_num) => match self.storage.is_canon(&block_header_hash) {
                true => Some(block_num),
                false => None,
            },
            Err(_) => None,
        };

        let confirmations = match height {
            Some(block_height) => self.storage.get_latest_block_height() - block_height,
            None => 0,
        };

        if let Ok(block) = self.storage.get_block(&block_header_hash) {
            let mut transactions = vec![];

            for transaction in block.transactions.iter() {
                transactions.push(hex::encode(&transaction.transaction_id()?));
            }

            Ok(BlockInfo {
                hash: block_hash_string,
                height,
                confirmations,
                size: block.serialize()?.len(),
                previous_block_hash: block.header.previous_block_hash.to_string(),
                merkle_root: block.header.merkle_root_hash.to_string(),
                pedersen_merkle_root_hash: block.header.pedersen_merkle_root_hash.to_string(),
                proof: block.header.proof.to_string(),
                time: block.header.time,
                difficulty_target: block.header.difficulty_target,
                nonce: block.header.nonce,
                transactions,
            })
        } else {
            Err(RpcError::InvalidBlockHash(block_hash_string))
        }
    }

    /// Returns the number of blocks in the canonical chain.
    fn get_block_count(&self) -> Result<u32, RpcError> {
        Ok(self.storage.get_block_count())
    }

    /// Returns the block hash of the head of the canonical chain.
    fn get_best_block_hash(&self) -> Result<String, RpcError> {
        let best_block_hash = self.storage.get_block_hash(self.storage.get_latest_block_height())?;

        Ok(hex::encode(&best_block_hash.0))
    }

    /// Returns the block hash of the index specified if it exists in the canonical chain.
    fn get_block_hash(&self, block_height: u32) -> Result<String, RpcError> {
        let block_hash = self.storage.get_block_hash(block_height)?;

        Ok(hex::encode(&block_hash.0))
    }

    /// Returns hex encoded bytes of a transaction from its transaction id.
    fn get_raw_transaction(&self, transaction_id: String) -> Result<String, RpcError> {
        Ok(hex::encode(
            &self.storage.get_transaction_bytes(&hex::decode(transaction_id)?)?,
        ))
    }

    /// Returns information about a transaction from a transaction id.
    fn get_transaction_info(&self, transaction_id: String) -> Result<TransactionInfo, RpcError> {
        let transaction_bytes = self.get_raw_transaction(transaction_id)?;
        self.decode_raw_transaction(transaction_bytes)
    }

    fn create_raw_transaction(&self, transaction_input: TransactionInputs) -> Result<(String, Vec<String>), RpcError> {
        let rng = &mut thread_rng();

        assert!(transaction_input.old_records.len() > 0);
        assert!(transaction_input.old_records.len() <= Components::NUM_OUTPUT_RECORDS);
        assert!(transaction_input.old_account_private_keys.len() > 0);
        assert!(transaction_input.old_account_private_keys.len() <= Components::NUM_OUTPUT_RECORDS);
        assert!(transaction_input.recipients.len() > 0);
        assert!(transaction_input.recipients.len() <= Components::NUM_OUTPUT_RECORDS);

        // Fetch birth/death predicates
        let predicate_vk_hash = self
            .parameters
            .circuit_parameters
            .predicate_verification_key_hash
            .hash(&to_bytes![self.parameters.predicate_snark_parameters.verification_key]?)?;
        let predicate_vk_hash_bytes = to_bytes![predicate_vk_hash]?;

        let predicate = Predicate::new(predicate_vk_hash_bytes.clone());
        let new_birth_predicates = vec![predicate.clone(); Components::NUM_OUTPUT_RECORDS];
        let new_death_predicates = vec![predicate.clone(); Components::NUM_OUTPUT_RECORDS];

        // Decode old records
        let mut old_records = vec![];
        for record_string in transaction_input.old_records {
            let record_bytes = hex::decode(record_string)?;
            old_records.push(DPCRecord::<Components>::read(&record_bytes[..])?);
        }

        let mut old_account_private_keys = vec![];
        for private_key_string in transaction_input.old_account_private_keys {
            let private_key_bytes = hex::decode(private_key_string)?;
            old_account_private_keys.push(AccountPrivateKey::<Components>::read(&private_key_bytes[..])?);
        }

        // Fill with dummy records
        while old_records.len() < Components::NUM_OUTPUT_RECORDS {
            let old_sn_nonce = self
                .parameters
                .circuit_parameters
                .serial_number_nonce
                .hash(&[64u8; 1])?;

            let private_key = old_account_private_keys[0].clone();
            let public_key = AccountPublicKey::<Components>::from(
                &self.parameters.circuit_parameters.account_commitment,
                &private_key,
            )?;

            let dummy_record = InstantiatedDPC::generate_record(
                &self.parameters.circuit_parameters,
                &old_sn_nonce,
                &public_key,
                true, // The input record is dummy
                0,
                &RecordPayload::default(),
                &predicate,
                &predicate,
                rng,
            )?;

            old_records.push(dummy_record);
            old_account_private_keys.push(private_key);
        }

        assert_eq!(old_records.len(), Components::NUM_INPUT_RECORDS);
        assert_eq!(old_account_private_keys.len(), Components::NUM_INPUT_RECORDS);

        // Decode new recipient data
        let mut new_account_public_keys = vec![];
        let mut new_dummy_flags = vec![];
        let mut new_values = vec![];
        for recipient in transaction_input.recipients {
            let public_key_bytes = hex::decode(recipient.address)?;
            new_account_public_keys.push(AccountPublicKey::<Components>::read(&public_key_bytes[..])?);
            new_dummy_flags.push(false);
            new_values.push(recipient.amount);
        }

        // Fill dummy output values
        while new_account_public_keys.len() < Components::NUM_OUTPUT_RECORDS {
            new_account_public_keys.push(new_account_public_keys[0].clone());
            new_dummy_flags.push(true);
            new_values.push(0);
        }

        assert_eq!(new_account_public_keys.len(), Components::NUM_OUTPUT_RECORDS);
        assert_eq!(new_dummy_flags.len(), Components::NUM_OUTPUT_RECORDS);
        assert_eq!(new_values.len(), Components::NUM_OUTPUT_RECORDS);

        // Default record payload
        let new_payloads = vec![RecordPayload::default(); Components::NUM_OUTPUT_RECORDS];

        // Decode auxiliary
        let mut auxiliary = [0u8; 32];
        if let Some(auxiliary_string) = transaction_input.auxiliary {
            if let Ok(bytes) = hex::decode(auxiliary_string) {
                bytes.write(&mut auxiliary[..])?;
            }
        }

        // Decode memo
        let mut memo = [0u8; 32];
        if let Some(memo_string) = transaction_input.memo {
            if let Ok(bytes) = hex::decode(memo_string) {
                bytes.write(&mut memo[..])?;
            }
        }

        // Generate transaction
        let (records, transaction) = ConsensusParameters::create_transaction(
            &self.parameters,
            old_records,
            old_account_private_keys,
            new_account_public_keys,
            new_birth_predicates,
            new_death_predicates,
            new_dummy_flags,
            new_values,
            new_payloads,
            auxiliary,
            memo,
            transaction_input.network_id,
            &self.storage,
            rng,
        )?;

        let encoded_transaction = hex::encode(to_bytes![transaction]?);
        let mut encoded_records = vec![];
        for record in records {
            encoded_records.push(hex::encode(to_bytes![record]?));
        }

        Ok((encoded_transaction, encoded_records))
    }

    /// Returns information about a transaction from serialized transaction bytes.
    fn decode_raw_transaction(&self, transaction_bytes: String) -> Result<TransactionInfo, RpcError> {
        let transaction_bytes = hex::decode(transaction_bytes)?;
        let transaction = Tx::read(&transaction_bytes[..])?;

        let mut old_serial_numbers = vec![];

        for sn in transaction.old_serial_numbers() {
            let mut serial_number: Vec<u8> = vec![];
            CanonicalSerialize::serialize(sn, &mut serial_number).unwrap();
            old_serial_numbers.push(hex::encode(serial_number));
        }

        let mut new_commitments = vec![];

        for cm in transaction.new_commitments() {
            new_commitments.push(hex::encode(to_bytes![cm]?));
        }

        let memo = hex::encode(to_bytes![transaction.memorandum()]?);

        let mut signatures = vec![];
        for sig in &transaction.signatures {
            signatures.push(hex::encode(to_bytes![sig]?));
        }

        Ok(TransactionInfo {
            txid: hex::encode(&transaction.transaction_id()?),
            size: transaction_bytes.len(),
            old_serial_numbers,
            new_commitments,
            memo,
            digest: hex::encode(to_bytes![transaction.digest]?),
            transaction_proof: hex::encode(to_bytes![transaction.transaction_proof]?),
            predicate_commitment: hex::encode(to_bytes![transaction.predicate_commitment]?),
            local_data_commitment: hex::encode(to_bytes![transaction.local_data_commitment]?),
            value_balance: transaction.value_balance,
            signatures,
        })
    }

    /// Send raw transaction bytes to this node to be added into the mempool.
    /// If valid, the transaction will be stored and propagated to all peers.
    /// Returns the transaction id if valid.
    fn send_raw_transaction(&self, transaction_bytes: String) -> Result<String, RpcError> {
        let transaction_bytes = hex::decode(transaction_bytes)?;
        let transaction = Tx::read(&transaction_bytes[..])?;

        if !self
            .consensus
            .verify_transaction(&self.parameters, &transaction, &self.storage)?
        {
            // TODO (raychu86) Add more descriptive message. (e.g. tx already exists)
            return Ok("Transaction did not verify".into());
        }

        match self.storage.transcation_conflicts(&transaction) {
            Ok(_) => {
                Runtime::new()?.block_on(process_transaction_internal(
                    self.server_context.clone(),
                    &self.consensus,
                    &self.parameters,
                    self.storage.clone(),
                    self.memory_pool_lock.clone(),
                    to_bytes![transaction]?.to_vec(),
                    *Runtime::new()?.block_on(self.server_context.local_address.read()),
                ))?;

                Ok(hex::encode(transaction.transaction_id()?))
            }
            Err(_) => Ok("Transaction contains spent records".into()),
        }
    }

    /// Fetch the number of connected peers this node has.
    fn get_connection_count(&self) -> Result<usize, RpcError> {
        // Create a temporary tokio runtime to make an asynchronous function call
        let peer_book = Runtime::new()?.block_on(self.server_context.peer_book.read());

        Ok(peer_book.connected_total() as usize)
    }

    /// Returns this nodes connected peers.
    fn get_peer_info(&self) -> Result<PeerInfo, RpcError> {
        // Create a temporary tokio runtime to make an asynchronous function call
        let peer_book = Runtime::new()?.block_on(self.server_context.peer_book.read());

        let mut peers = vec![];

        for (peer, _last_seen) in &peer_book.get_connected() {
            peers.push(peer.clone());
        }

        Ok(PeerInfo { peers })
    }

    /// Returns the current mempool and consensus information known by this node.
    fn get_block_template(&self) -> Result<BlockTemplate, RpcError> {
        let block_height = self.storage.get_latest_block_height();
        let block = self.storage.get_block_from_block_num(block_height)?;

        let time = Utc::now().timestamp();

        let memory_pool = Runtime::new()?.block_on(self.memory_pool_lock.lock());
        let full_transactions = memory_pool.get_candidates(&self.storage, self.consensus.max_block_size)?;

        let transaction_strings = full_transactions.serialize_as_str()?;

        let coinbase_value = get_block_reward(block_height + 1) + full_transactions.calculate_transaction_fees();

        Ok(BlockTemplate {
            previous_block_hash: hex::encode(&block.header.get_hash().0),
            block_height: block_height + 1,
            time,
            difficulty_target: self.consensus.get_block_difficulty(&block.header, time),
            transactions: transaction_strings,
            coinbase_value,
        })
    }

    // Record handling

    /// Returns information about a record from serialized record bytes.
    fn decode_record(&self, record_bytes: String) -> Result<RecordInfo, RpcError> {
        let record_bytes = hex::decode(record_bytes)?;
        let record = DPCRecord::<Components>::read(&record_bytes[..])?;

        let account_public_key = hex::encode(to_bytes![record.account_public_key()]?);
        let payload = RPCRecordPayload {
            payload: hex::encode(to_bytes![record.payload()]?),
        };
        let birth_predicate_repr = hex::encode(record.birth_predicate_repr());
        let death_predicate_repr = hex::encode(record.death_predicate_repr());
        let serial_number_nonce = hex::encode(to_bytes![record.serial_number_nonce()]?);
        let commitment = hex::encode(to_bytes![record.commitment()]?);
        let commitment_randomness = hex::encode(to_bytes![record.commitment_randomness()]?);

        Ok(RecordInfo {
            account_public_key,
            is_dummy: record.is_dummy(),
            value: record.value(),
            payload,
            birth_predicate_repr,
            death_predicate_repr,
            serial_number_nonce,
            commitment,
            commitment_randomness,
        })
    }

    // TODO (raychu86) add password guarding

    /// Fetch the node's stored record commitments
    fn fetch_record_commtiments(&self) -> Result<Vec<String>, RpcError> {
        let record_commitments = self.storage.get_record_commitments(100)?;
        let record_commitment_strings: Vec<String> = record_commitments.iter().map(|cm| hex::encode(cm)).collect();

        Ok(record_commitment_strings)
    }

    /// Returns hex encoded bytes of a record from its record commitment
    fn get_raw_record(&self, record_commitment: String) -> Result<String, RpcError> {
        match self
            .storage
            .get_record::<DPCRecord<Components>>(&hex::decode(record_commitment)?)?
        {
            Some(record) => {
                let record_bytes = to_bytes![record]?;
                Ok(hex::encode(record_bytes))
            }
            None => Ok("Record not found".to_string()),
        }
    }
}

impl RpcImpl {}
