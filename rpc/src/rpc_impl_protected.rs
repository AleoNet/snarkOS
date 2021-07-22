// Copyright (C) 2019-2021 Aleo Systems Inc.
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

//! Implementation of private RPC endpoints that require authentication.
//!
//! See [ProtectedRpcFunctions](../trait.ProtectedRpcFunctions.html) for documentation of private endpoints.

use crate::{RpcImpl, error::RpcError, rpc_trait::ProtectedRpcFunctions, rpc_types::*, transaction_kernel_builder::TransactionKernelBuilder};
use snarkos_consensus::{CreatePartialTransactionRequest, CreateTransactionRequest};
use snarkos_storage::VMRecord;
use snarkvm_algorithms::CRH;
use snarkvm_dpc::{Account, AccountScheme, Address, DPCComponents, PrivateKey, ProgramScheme, RecordScheme as RecordModel, ViewKey, testnet1::{
        instantiated::Components,
        EncryptedRecord,
        Payload,
        Record as DPCRecord,
        TransactionKernel,
    }};
use snarkvm_utilities::{
    bytes::{FromBytes, ToBytes},
    to_bytes_le,
};

use itertools::Itertools;
use jsonrpc_core::{IoDelegate, MetaIoHandler, Params, Value};
use rand::{thread_rng, Rng};
use std::{net::SocketAddr, str::FromStr, sync::Arc};

type JsonRPCError = jsonrpc_core::Error;

/// The following `*_protected` functions wrap an authentication check around sensitive functions
/// before being exposed as an RPC endpoint
impl RpcImpl {
    /// Validate the authentication header in the request metadata
    pub async fn validate_auth(&self, meta: Meta) -> Result<(), JsonRPCError> {
        if let Some(credentials) = &self.credentials {
            let auth = meta.auth.unwrap_or_else(String::new);
            let basic_auth_encoding = format!(
                "Basic {}",
                base64::encode(format!("{}:{}", credentials.username, credentials.password))
            );

            if basic_auth_encoding != auth {
                return Err(JsonRPCError::invalid_params("Authentication Error"));
            }
        }

        Ok(())
    }

    /// Wrap authentication around `create_raw_transaction`
    pub async fn create_raw_transaction_protected(self, params: Params, meta: Meta) -> Result<Value, JsonRPCError> {
        self.validate_auth(meta).await?;

        let value = match params {
            Params::Array(arr) => arr,
            _ => return Err(JsonRPCError::invalid_request()),
        };

        let val: TransactionInputs = serde_json::from_value(value[0].clone())
            .map_err(|e| JsonRPCError::invalid_params(format!("Invalid params: {}.", e)))?;

        match self.create_raw_transaction(val).await {
            Ok(result) => Ok(serde_json::to_value(result).expect("transaction output serialization failed")),
            Err(err) => Err(JsonRPCError::invalid_params(err.to_string())),
        }
    }

    /// Wrap authentication around `create_transaction_kernel`
    pub async fn create_transaction_kernel_protected(self, params: Params, meta: Meta) -> Result<Value, JsonRPCError> {
        self.validate_auth(meta).await?;

        let value = match params {
            Params::Array(arr) => arr,
            _ => return Err(JsonRPCError::invalid_request()),
        };

        let val: TransactionInputs = serde_json::from_value(value[0].clone())
            .map_err(|e| JsonRPCError::invalid_params(format!("Invalid params: {}.", e)))?;

        match self.create_transaction_kernel(val).await {
            Ok(result) => Ok(serde_json::to_value(result).expect("transaction kernel serialization failed")),
            Err(err) => Err(JsonRPCError::invalid_params(err.to_string())),
        }
    }

    /// Wrap authentication around `create_transaction`
    pub async fn create_transaction_protected(self, params: Params, meta: Meta) -> Result<Value, JsonRPCError> {
        self.validate_auth(meta).await?;

        let value = match params {
            Params::Array(arr) => arr,
            _ => return Err(JsonRPCError::invalid_request()),
        };

        if value.len() != 2 {
            return Err(JsonRPCError::invalid_params(format!(
                "invalid length {}, expected 2 element",
                value.len()
            )));
        }

        let private_keys: [String; 2] = serde_json::from_value(value[0].clone())
            .map_err(|e| JsonRPCError::invalid_params(format!("Invalid params: {}.", e)))?;

        let transaction_kernel: String = serde_json::from_value(value[1].clone())
            .map_err(|e| JsonRPCError::invalid_params(format!("Invalid params: {}.", e)))?;

        match self.create_transaction(private_keys, transaction_kernel).await {
            Ok(result) => Ok(serde_json::to_value(result).expect("transaction output serialization failed")),
            Err(err) => Err(JsonRPCError::invalid_params(err.to_string())),
        }
    }

    /// Wrap authentication around `get_record_commitment_count`
    pub async fn get_record_commitment_count_protected(
        self,
        params: Params,
        meta: Meta,
    ) -> Result<Value, JsonRPCError> {
        self.validate_auth(meta).await?;

        params.expect_no_params()?;

        match self.get_record_commitment_count().await {
            Ok(num_record_commitments) => Ok(Value::from(num_record_commitments)),
            Err(_) => Err(JsonRPCError::invalid_request()),
        }
    }

    /// Wrap authentication around `get_record_commitments`
    pub async fn get_record_commitments_protected(self, params: Params, meta: Meta) -> Result<Value, JsonRPCError> {
        self.validate_auth(meta).await?;

        params.expect_no_params()?;

        match self.get_record_commitments().await {
            Ok(record_commitments) => Ok(Value::from(record_commitments)),
            Err(_) => Err(JsonRPCError::invalid_request()),
        }
    }

    /// Wrap authentication around `get_raw_record`
    pub async fn get_raw_record_protected(self, params: Params, meta: Meta) -> Result<Value, JsonRPCError> {
        self.validate_auth(meta).await?;

        let value = match params {
            Params::Array(arr) => arr,
            _ => return Err(JsonRPCError::invalid_request()),
        };

        if value.len() != 1 {
            return Err(JsonRPCError::invalid_params(format!(
                "invalid length {}, expected 1 element",
                value.len()
            )));
        }

        let record_commitment: String = serde_json::from_value(value[0].clone())
            .map_err(|e| JsonRPCError::invalid_params(format!("Invalid params: {}.", e)))?;

        match self.get_raw_record(record_commitment).await {
            Ok(record) => Ok(Value::from(record)),
            Err(err) => Err(JsonRPCError::invalid_params(err.to_string())),
        }
    }

    /// Wrap authentication around `decode_record`
    pub async fn decode_record_protected(self, params: Params, meta: Meta) -> Result<Value, JsonRPCError> {
        self.validate_auth(meta).await?;

        let value = match params {
            Params::Array(arr) => arr,
            _ => return Err(JsonRPCError::invalid_request()),
        };

        if value.len() != 1 {
            return Err(JsonRPCError::invalid_params(format!(
                "invalid length {}, expected 1 element",
                value.len()
            )));
        }

        let record_bytes: String = serde_json::from_value(value[0].clone())
            .map_err(|e| JsonRPCError::invalid_params(format!("Invalid params: {}.", e)))?;

        match self.decode_record(record_bytes).await {
            Ok(record) => Ok(serde_json::to_value(record).expect("record deserialization failed")),
            Err(err) => Err(JsonRPCError::invalid_params(err.to_string())),
        }
    }

    /// Wrap authentication around `decrypt_record`
    pub async fn decrypt_record_protected(self, params: Params, meta: Meta) -> Result<Value, JsonRPCError> {
        self.validate_auth(meta).await?;

        let value = match params {
            Params::Array(arr) => arr,
            _ => return Err(JsonRPCError::invalid_request()),
        };

        let decrypt_record_input: DecryptRecordInput = serde_json::from_value(value[0].clone())
            .map_err(|e| JsonRPCError::invalid_params(format!("Invalid params: {}.", e)))?;

        match self.decrypt_record(decrypt_record_input).await {
            Ok(result) => Ok(serde_json::to_value(result).expect("record serialization failed")),
            Err(err) => Err(JsonRPCError::invalid_params(err.to_string())),
        }
    }

    /// Wrap authentication around `create_account`
    pub async fn create_account_protected(self, params: Params, meta: Meta) -> Result<Value, JsonRPCError> {
        self.validate_auth(meta).await?;

        params.expect_no_params()?;

        match self.create_account().await {
            Ok(account) => Ok(serde_json::to_value(account).expect("account serialization failed")),
            Err(err) => Err(JsonRPCError::invalid_params(err.to_string())),
        }
    }

    /// Disconnects from the given address
    pub async fn disconnect_protected(self, params: Params, meta: Meta) -> Result<Value, JsonRPCError> {
        self.validate_auth(meta).await?;

        let value = match params {
            Params::Array(arr) => arr,
            _ => return Err(JsonRPCError::invalid_request()),
        };

        let address: SocketAddr = serde_json::from_value(value[0].clone())
            .map_err(|e| JsonRPCError::invalid_params(format!("Invalid params: {}.", e)))?;

        self.node.disconnect_from_peer(address).await;

        Ok(Value::Null)
    }

    /// Connects to the given addresses
    pub async fn connect_protected(self, params: Params, meta: Meta) -> Result<Value, JsonRPCError> {
        self.validate_auth(meta).await?;

        let value = match params {
            Params::Array(arr) => arr,
            _ => return Err(JsonRPCError::invalid_request()),
        };

        let addresses: Vec<SocketAddr> = value
            .into_iter()
            .map(serde_json::from_value)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| JsonRPCError::invalid_params(format!("Invalid params: {}.", e)))?;

        for addr in &addresses {
            self.node.peer_book.add_peer(*addr, false).await;
        }
        self.node.connect_to_addresses(&addresses).await;

        Ok(Value::Null)
    }

    /// Expose the protected functions as RPC enpoints
    pub fn add_protected(&self, io: &mut MetaIoHandler<Meta>) {
        let mut d = IoDelegate::<Self, Meta>::new(Arc::new(self.clone()));

        d.add_method_with_meta("createrawtransaction", |rpc, params, meta| {
            let rpc = rpc.clone();
            rpc.create_raw_transaction_protected(params, meta)
        });
        d.add_method_with_meta("createtransactionkernel", |rpc, params, meta| {
            let rpc = rpc.clone();
            rpc.create_transaction_kernel_protected(params, meta)
        });
        d.add_method_with_meta("createtransaction", |rpc, params, meta| {
            let rpc = rpc.clone();
            rpc.create_transaction_protected(params, meta)
        });
        d.add_method_with_meta("decoderecord", |rpc, params, meta| {
            let rpc = rpc.clone();
            rpc.decode_record_protected(params, meta)
        });
        d.add_method_with_meta("decryptrecord", |rpc, params, meta| {
            let rpc = rpc.clone();
            rpc.decrypt_record_protected(params, meta)
        });
        d.add_method_with_meta("getrecordcommitmentcount", |rpc, params, meta| {
            let rpc = rpc.clone();
            rpc.get_record_commitment_count_protected(params, meta)
        });
        d.add_method_with_meta("getrecordcommitments", |rpc, params, meta| {
            let rpc = rpc.clone();
            rpc.get_record_commitments_protected(params, meta)
        });
        d.add_method_with_meta("getrawrecord", |rpc, params, meta| {
            let rpc = rpc.clone();
            rpc.get_raw_record_protected(params, meta)
        });
        d.add_method_with_meta("createaccount", |rpc, params, meta| {
            let rpc = rpc.clone();
            rpc.create_account_protected(params, meta)
        });
        d.add_method_with_meta("disconnect", |rpc, params, meta| {
            let rpc = rpc.clone();
            rpc.disconnect_protected(params, meta)
        });
        d.add_method_with_meta("connect", |rpc, params, meta| {
            let rpc = rpc.clone();
            rpc.connect_protected(params, meta)
        });

        io.extend_with(d)
    }
}

/// Functions that are sensitive and need to be protected with authentication.
/// The authentication logic is defined in `validate_auth`
#[async_trait::async_trait]
impl ProtectedRpcFunctions for RpcImpl {
    /// Generate a new account private key, account view key, and account address.
    async fn create_account(&self) -> Result<RpcAccount, RpcError> {
        let rng = &mut thread_rng();

        let account = Account::<Components>::new(
            &self.dpc()?.system_parameters.account_signature,
            &self.dpc()?.system_parameters.account_commitment,
            &self.dpc()?.system_parameters.account_encryption,
            rng,
        )?;

        let view_key = ViewKey::<Components>::from_private_key(
            &self.dpc()?.system_parameters.account_signature,
            &self.dpc()?.system_parameters.account_commitment,
            &account.private_key,
        )?;

        Ok(RpcAccount {
            private_key: account.private_key.to_string(),
            view_key: view_key.to_string(),
            address: account.address.to_string(),
        })
    }

    // TODO (raychu86): Deprecate this rpc endpoint in favor of the more secure offline/online model.
    /// Create a new transaction, returning the encoded transaction and the new records.
    async fn create_raw_transaction(
        &self,
        transaction_input: TransactionInputs,
    ) -> Result<CreateRawTransactionOuput, RpcError> {
        assert!(!transaction_input.old_records.is_empty());
        assert!(transaction_input.old_records.len() <= Components::NUM_INPUT_RECORDS);
        assert!(!transaction_input.old_account_private_keys.is_empty());
        assert!(transaction_input.old_account_private_keys.len() <= Components::NUM_OUTPUT_RECORDS);
        assert!(!transaction_input.recipients.is_empty());
        assert!(transaction_input.recipients.len() <= Components::NUM_OUTPUT_RECORDS);

        let consensus = &self.node.expect_sync().consensus;

        // Fetch birth/death programs
        let program_id = self.dpc()?.noop_program.id();
        let new_birth_program_ids = vec![program_id.clone(); Components::NUM_OUTPUT_RECORDS];
        let new_death_program_ids = vec![program_id.clone(); Components::NUM_OUTPUT_RECORDS];

        // Decode old records
        let mut old_records = Vec::with_capacity(transaction_input.old_records.len());
        for record_string in transaction_input.old_records {
            let record_bytes = hex::decode(record_string)?;
            old_records.push(DPCRecord::<Components>::read_le(&record_bytes[..])?.serialize()?);
        }

        let mut old_account_private_keys = Vec::with_capacity(transaction_input.old_account_private_keys.len());
        for private_key_string in transaction_input.old_account_private_keys {
            old_account_private_keys.push(PrivateKey::<Components>::from_str(&private_key_string)?);
        }

        let sn_randomness: [u8; 32] = thread_rng().gen();
        let mut joint_serial_numbers = vec![];

        // Fill any unused old_record indices with dummy records
        for i in 0..Components::NUM_OUTPUT_RECORDS {
            let old_sn_nonce = self.dpc()?.system_parameters.serial_number_nonce.hash(&sn_randomness)?;

            let address = Address::<Components>::from_private_key(
                &self.dpc()?.system_parameters.account_signature,
                &self.dpc()?.system_parameters.account_commitment,
                &self.dpc()?.system_parameters.account_encryption,
                &old_account_private_keys[i],
            )?;

            let dummy_record = DPCRecord::<Components>::new(
                &self.dpc()?.system_parameters.record_commitment,
                address,
                true, // The input record is dummy
                0,
                Payload::default(),
                program_id.clone(),
                program_id.clone(),
                old_sn_nonce,
                &mut thread_rng(),
            )?;

            let (sn, _) = dummy_record.to_serial_number(&consensus.dpc.system_parameters.account_signature, &old_account_private_keys[i])?;
            joint_serial_numbers.extend_from_slice(&to_bytes_le![sn]?);

            old_records.push(dummy_record.serialize()?);
        }

        assert_eq!(old_records.len(), Components::NUM_INPUT_RECORDS);
        assert_eq!(old_account_private_keys.len(), Components::NUM_INPUT_RECORDS);

        // Decode new recipient data
        let mut new_record_owners = Vec::with_capacity(Components::NUM_OUTPUT_RECORDS);
        let mut new_is_dummy_flags = Vec::with_capacity(Components::NUM_OUTPUT_RECORDS);
        let mut new_values = Vec::with_capacity(Components::NUM_OUTPUT_RECORDS);
        for recipient in transaction_input.recipients {
            new_record_owners.push(Address::<Components>::from_str(&recipient.address)?);
            new_is_dummy_flags.push(false);
            new_values.push(recipient.amount);
        }

        // Fill any unused new_record indices with dummy output values
        while new_record_owners.len() < Components::NUM_OUTPUT_RECORDS {
            new_record_owners.push(new_record_owners[0].clone());
            new_is_dummy_flags.push(true);
            new_values.push(0);
        }

        assert_eq!(new_record_owners.len(), Components::NUM_OUTPUT_RECORDS);
        assert_eq!(new_is_dummy_flags.len(), Components::NUM_OUTPUT_RECORDS);
        assert_eq!(new_values.len(), Components::NUM_OUTPUT_RECORDS);

        // Default record payload
        let new_payloads = vec![Payload::default(); Components::NUM_OUTPUT_RECORDS];

        // Decode memo
        let mut memo = [0u8; 32];
        if let Some(memo_string) = transaction_input.memo {
            if let Ok(bytes) = hex::decode(memo_string) {
                bytes.write_le(&mut memo[..])?;
            }
        }

        // If the request did not specify a valid memo, generate one from random
        if memo == [0u8; 32] {
            memo = thread_rng().gen();
        }

        let mut new_records = vec![];
        for j in 0..Components::NUM_OUTPUT_RECORDS {
            new_records.push(DPCRecord::new_full(
                &consensus.dpc.system_parameters.serial_number_nonce,
                &consensus.dpc.system_parameters.record_commitment,
                new_record_owners[j].clone().into(),
                new_is_dummy_flags[j],
                new_values[j],
                new_payloads[j].clone(),
                new_birth_program_ids[j].clone(),
                new_death_program_ids[j].clone(),
                j as u8,
                joint_serial_numbers.clone(),
                &mut thread_rng(),
            )?.serialize()?);
        }

        // Generate transaction
        let response = self
            .sync_handler()?
            .consensus
            .create_transaction(CreateTransactionRequest {
                old_records,
                old_account_private_keys: old_account_private_keys.into_iter().map(Into::into).collect(),
                new_records,
                memo,
            })
            .await?;

        let encoded_transaction = hex::encode(to_bytes_le![response.transaction]?);
        let mut encoded_records = Vec::with_capacity(response.records.len());
        for record in response.records {
            encoded_records.push(hex::encode(to_bytes_le![record]?));
        }

        Ok(CreateRawTransactionOuput {
            encoded_transaction,
            encoded_records,
        })
    }

    /// Generates and returns a new transaction kernel.
    async fn create_transaction_kernel(&self, transaction_input: TransactionInputs) -> Result<String, RpcError> {
        let rng = &mut thread_rng();

        assert!(!transaction_input.old_records.is_empty());
        assert!(transaction_input.old_records.len() <= Components::NUM_INPUT_RECORDS);
        assert!(!transaction_input.old_account_private_keys.is_empty());
        assert!(transaction_input.old_account_private_keys.len() <= Components::NUM_OUTPUT_RECORDS);
        assert!(!transaction_input.recipients.is_empty());
        assert!(transaction_input.recipients.len() <= Components::NUM_OUTPUT_RECORDS);

        let mut builder = TransactionKernelBuilder::new();

        // Add individual transaction inputs to the transaction kernel builder.
        for (record_string, private_key_string) in transaction_input
            .old_records
            .iter()
            .zip_eq(&transaction_input.old_account_private_keys)
        {
            let record = DPCRecord::<Components>::from_str(record_string)?;
            let private_key = PrivateKey::<Components>::from_str(private_key_string)?;

            builder = builder.add_input(private_key, record)?;
        }

        // Add individual transaction outputs to the transaction kernel builder.
        for recipient in &transaction_input.recipients {
            let address = Address::<Components>::from_str(&recipient.address)?;

            builder = builder.add_output(address, recipient.amount)?;
        }

        // Decode memo
        let mut memo = [0u8; 32];
        if let Some(memo_string) = transaction_input.memo {
            if let Ok(bytes) = hex::decode(memo_string) {
                bytes.write_le(&mut memo[..])?;
            }
        }

        // If the request did not specify a valid memo, generate one from random.
        if memo == [0u8; 32] {
            memo = rng.gen();
        }

        // Set the memo in the transaction kernel builder.
        builder = builder.memo(memo);

        // Set the network id in the transaction kernel builder.
        builder = builder.network_id(transaction_input.network_id);

        // Construct the transaction kernel
        let transaction_kernel = builder.build(rng)?;

        Ok(hex::encode(transaction_kernel.to_bytes()))
    }

    /// Create a new transaction for a given transaction kernel.
    async fn create_transaction(
        &self,
        private_keys: [String; Components::NUM_INPUT_RECORDS],
        transaction_kernel: String,
    ) -> Result<CreateRawTransactionOuput, RpcError> {

        // Decode the private keys
        let mut old_private_keys = Vec::with_capacity(Components::NUM_INPUT_RECORDS);
        for private_key in private_keys {
            old_private_keys.push(PrivateKey::<Components>::from_str(&private_key)?.into());
        }
        
        // Decode the transaction kernel
        let transaction_kernel_bytes = hex::decode(transaction_kernel)?;
        let transaction_kernel = TransactionKernel::<Components>::read_le(&transaction_kernel_bytes[..])?;

        let response = self
            .node
            .expect_sync()
            .consensus
            .create_partial_transaction(CreatePartialTransactionRequest {
                kernel: Box::new(transaction_kernel),
                old_account_private_keys: old_private_keys,
            })
            .await?;

        let encoded_transaction = hex::encode(to_bytes_le![response.transaction]?);
        let mut encoded_records = Vec::with_capacity(response.records.len());
        for record in response.records {
            encoded_records.push(hex::encode(to_bytes_le![record]?));
        }

        Ok(CreateRawTransactionOuput {
            encoded_transaction,
            encoded_records,
        })
    }

    /// Returns the number of record commitments that are stored on the full node.
    async fn get_record_commitment_count(&self) -> Result<usize, RpcError> {
        let storage = &self.storage;
        let record_commitments = storage.get_record_commitments(None).await?;

        Ok(record_commitments.len())
    }

    /// Returns a list of record commitments that are stored on the full node.
    async fn get_record_commitments(&self) -> Result<Vec<String>, RpcError> {
        let record_commitments = self.storage.get_record_commitments(Some(100)).await?;
        let record_commitment_strings: Vec<String> = record_commitments.iter().map(hex::encode).collect();

        Ok(record_commitment_strings)
    }

    /// Returns the hex encoded bytes of a record from its record commitment
    async fn get_raw_record(&self, record_commitment: String) -> Result<String, RpcError> {
        let decoded = hex::decode(record_commitment)?;
        match self.storage.get_record(decoded[..].into()).await? {
            Some(record) => Ok(hex::encode(to_bytes_le![record]?)),
            None => Ok("Record not found".to_string()),
        }
    }

    /// Decrypts the record ciphertext and returns the hex encoded bytes of the record.
    async fn decrypt_record(&self, decryption_input: DecryptRecordInput) -> Result<String, RpcError> {
        // Read the encrypted_record
        let encrypted_record_bytes = hex::decode(decryption_input.encrypted_record)?;
        let encrypted_record = EncryptedRecord::<Components>::read_le(&encrypted_record_bytes[..])?;

        // Read the view key
        let view_key = ViewKey::<Components>::from_str(&decryption_input.account_view_key)?;

        // Decrypt the record ciphertext
        let record = encrypted_record.decrypt(&self.dpc()?.system_parameters, &view_key)?;
        let record_bytes = to_bytes_le![record]?;

        Ok(hex::encode(record_bytes))
    }

    /// Returns information about a record from serialized record bytes.
    async fn decode_record(&self, record_bytes: String) -> Result<RecordInfo, RpcError> {
        let record_bytes = hex::decode(record_bytes)?;
        let record = DPCRecord::<Components>::read_le(&record_bytes[..])?;

        let owner = record.owner().to_string();
        let payload = RPCRecordPayload {
            payload: hex::encode(to_bytes_le![record.payload()]?),
        };
        let birth_program_id = hex::encode(record.birth_program_id());
        let death_program_id = hex::encode(record.death_program_id());
        let serial_number_nonce = hex::encode(to_bytes_le![record.serial_number_nonce()]?);
        let commitment = hex::encode(to_bytes_le![record.commitment()]?);
        let commitment_randomness = hex::encode(to_bytes_le![record.commitment_randomness()]?);

        Ok(RecordInfo {
            owner,
            is_dummy: record.is_dummy(),
            value: record.value(),
            payload,
            birth_program_id,
            death_program_id,
            serial_number_nonce,
            commitment,
            commitment_randomness,
        })
    }

    async fn disconnect(&self, address: SocketAddr) {
        let node = self.node.clone();
        tokio::spawn(async move { node.disconnect_from_peer(address).await });
    }

    async fn connect(&self, addresses: Vec<SocketAddr>) {
        let node = self.node.clone();
        tokio::spawn(async move {
            for addr in &addresses {
                node.peer_book.add_peer(*addr, false).await;
            }
            node.connect_to_addresses(&addresses).await
        });
    }
}
