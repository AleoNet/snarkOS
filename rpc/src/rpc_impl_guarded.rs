use crate::{rpc_trait::GuardedRpcFunctions, rpc_types::*, RpcImpl};
use snarkos_consensus::ConsensusParameters;
use snarkos_dpc::base_dpc::{
    instantiated::{Components, InstantiatedDPC, Predicate},
    record::DPCRecord,
    record_payload::RecordPayload,
};
use snarkos_errors::rpc::RpcError;
use snarkos_models::{algorithms::CRH, dpc::DPCComponents};
use snarkos_objects::{AccountPrivateKey, AccountPublicKey};
use snarkos_utilities::{
    bytes::{FromBytes, ToBytes},
    to_bytes,
};

use jsonrpc_http_server::jsonrpc_core::{IoDelegate, MetaIoHandler, Params, Value};

use base64;
use rand::thread_rng;
use std::sync::Arc;

type JsonrpcError = jsonrpc_core::Error;

impl RpcImpl {
    pub fn validate_auth(&self, meta: Meta) -> Result<(), JsonrpcError> {
        if let Some(credentials) = &self.credentials {
            let auth = meta.auth.unwrap_or_else(String::new);
            let basic_auth_encoding = format!(
                "Basic {}",
                base64::encode(format!("{}:{}", credentials.username, credentials.password))
            );

            if basic_auth_encoding != auth {
                return Err(JsonrpcError::invalid_params("Authentication Error"));
            }
        }

        Ok(())
    }

    pub fn create_raw_transaction_guarded(&self, params: Params, meta: Meta) -> Result<Value, JsonrpcError> {
        self.validate_auth(meta)?;

        let value = match params {
            Params::Array(arr) => arr,
            _ => return Err(JsonrpcError::invalid_request()),
        };

        let val: TransactionInputs = serde_json::from_value(value[0].clone())
            .map_err(|e| JsonrpcError::invalid_params(format!("Invalid params: {}.", e)))?;
        Ok(serde_json::to_value(self.create_raw_transaction(val).unwrap()).unwrap())
    }

    pub fn fetch_record_commitments_guarded(&self, params: Params, meta: Meta) -> Result<Value, JsonrpcError> {
        self.validate_auth(meta)?;

        params.expect_no_params()?;

        Ok(Value::from(self.fetch_record_commtiments().unwrap()))
    }

    pub fn get_raw_record_guarded(&self, params: Params, meta: Meta) -> Result<Value, JsonrpcError> {
        self.validate_auth(meta)?;

        let value = match params {
            Params::Array(arr) => arr,
            _ => return Err(JsonrpcError::invalid_request()),
        };

        if value.len() != 1 {
            return Err(JsonrpcError::invalid_params(format!(
                "invalid length {}, expected 1 element",
                value.len()
            )));
        }

        let record_commitment: String = serde_json::from_value(value[0].clone())
            .map_err(|e| JsonrpcError::invalid_params(format!("Invalid params: {}.", e)))?;
        Ok(Value::from(self.get_raw_record(record_commitment).unwrap()))
    }

    pub fn add_guarded(&self, io: &mut MetaIoHandler<Meta>) {
        let mut d = IoDelegate::<Self, Meta>::new(Arc::new(self.clone()));

        d.add_method_with_meta("createrawtransaction", Self::create_raw_transaction_guarded);
        d.add_method_with_meta("fetchrecordcommitments", Self::fetch_record_commitments_guarded);
        d.add_method_with_meta("getrawrecord", Self::get_raw_record_guarded);

        io.extend_with(d)
    }
}

impl GuardedRpcFunctions for RpcImpl {
    fn create_raw_transaction(
        &self,
        transaction_input: TransactionInputs,
    ) -> Result<CreateRawTransactionOuput, RpcError> {
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

        Ok(CreateRawTransactionOuput {
            encoded_transaction,
            encoded_records,
        })
    }

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
