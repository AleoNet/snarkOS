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

use crate::{
    error::RpcError,
    rpc_trait::ProtectedRpcFunctions,
    rpc_types::*,
    transaction_authorization_builder::TransactionAuthorizationBuilder,
    RpcImpl,
};
use snarkvm::{
    algorithms::{merkle_tree::MerkleTreeDigest, CRH},
    dpc::{prelude::*, testnet1::Testnet1Parameters},
    ledger::prelude::*,
    prelude::UniformRand,
    utilities::{to_bytes_le, FromBytes, ToBytes},
};

use itertools::Itertools;
use jsonrpc_core::{IoDelegate, MetaIoHandler, Params, Value};
use rand::{thread_rng, Rng};
use std::{net::SocketAddr, ops::Deref, str::FromStr, sync::Arc};

type JsonRPCError = jsonrpc_core::Error;

/// The following `*_protected` functions wrap an authentication check around sensitive functions
/// before being exposed as an RPC endpoint
impl<S: Storage + Send + Sync + 'static> RpcImpl<S> {
    /// Validate the authentication header in the request metadata
    pub fn validate_auth(&self, meta: Meta) -> Result<(), JsonRPCError> {
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
        self.validate_auth(meta)?;

        let value = match params {
            Params::Array(arr) => arr,
            _ => return Err(JsonRPCError::invalid_request()),
        };

        let val: TransactionInputs = serde_json::from_value(value[0].clone())
            .map_err(|e| JsonRPCError::invalid_params(format!("Invalid params: {}.", e)))?;

        match self.create_raw_transaction(val) {
            Ok(result) => Ok(serde_json::to_value(result).expect("transaction output serialization failed")),
            Err(err) => Err(JsonRPCError::invalid_params(err.to_string())),
        }
    }

    /// Wrap authentication around `create_transaction_authorization`
    pub async fn create_transaction_authorization_protected(
        self,
        params: Params,
        meta: Meta,
    ) -> Result<Value, JsonRPCError> {
        self.validate_auth(meta)?;

        let value = match params {
            Params::Array(arr) => arr,
            _ => return Err(JsonRPCError::invalid_request()),
        };

        let val: TransactionInputs = serde_json::from_value(value[0].clone())
            .map_err(|e| JsonRPCError::invalid_params(format!("Invalid params: {}.", e)))?;

        match self.create_transaction_authorization(val) {
            Ok(result) => Ok(serde_json::to_value(result).expect("transaction authorization serialization failed")),
            Err(err) => Err(JsonRPCError::invalid_params(err.to_string())),
        }
    }

    /// Wrap authentication around `create_transaction`
    pub async fn create_transaction_protected(self, params: Params, meta: Meta) -> Result<Value, JsonRPCError> {
        self.validate_auth(meta)?;

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

        let transaction_authorization: String = serde_json::from_value(value[1].clone())
            .map_err(|e| JsonRPCError::invalid_params(format!("Invalid params: {}.", e)))?;

        match self.create_transaction(private_keys, transaction_authorization) {
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
        self.validate_auth(meta)?;

        params.expect_no_params()?;

        match self.get_record_commitment_count() {
            Ok(num_record_commitments) => Ok(Value::from(num_record_commitments)),
            Err(_) => Err(JsonRPCError::invalid_request()),
        }
    }

    /// Wrap authentication around `get_record_commitments`
    pub async fn get_record_commitments_protected(self, params: Params, meta: Meta) -> Result<Value, JsonRPCError> {
        self.validate_auth(meta)?;

        params.expect_no_params()?;

        match self.get_record_commitments() {
            Ok(record_commitments) => Ok(Value::from(record_commitments)),
            Err(_) => Err(JsonRPCError::invalid_request()),
        }
    }

    /// Wrap authentication around `get_raw_record`
    pub async fn get_raw_record_protected(self, params: Params, meta: Meta) -> Result<Value, JsonRPCError> {
        self.validate_auth(meta)?;

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

        match self.get_raw_record(record_commitment) {
            Ok(record) => Ok(Value::from(record)),
            Err(err) => Err(JsonRPCError::invalid_params(err.to_string())),
        }
    }

    /// Wrap authentication around `decrypt_record`
    pub async fn decrypt_record_protected(self, params: Params, meta: Meta) -> Result<Value, JsonRPCError> {
        self.validate_auth(meta)?;

        let value = match params {
            Params::Array(arr) => arr,
            _ => return Err(JsonRPCError::invalid_request()),
        };

        let decrypt_record_input: DecryptRecordInput = serde_json::from_value(value[0].clone())
            .map_err(|e| JsonRPCError::invalid_params(format!("Invalid params: {}.", e)))?;

        match self.decrypt_record(decrypt_record_input) {
            Ok(result) => Ok(serde_json::to_value(result).expect("record serialization failed")),
            Err(err) => Err(JsonRPCError::invalid_params(err.to_string())),
        }
    }

    /// Wrap authentication around `create_account`
    pub async fn create_account_protected(self, params: Params, meta: Meta) -> Result<Value, JsonRPCError> {
        self.validate_auth(meta)?;

        params.expect_no_params()?;

        match self.create_account() {
            Ok(account) => Ok(serde_json::to_value(account).expect("account serialization failed")),
            Err(err) => Err(JsonRPCError::invalid_params(err.to_string())),
        }
    }

    /// Disconnects from the given address
    pub async fn disconnect_protected(self, params: Params, meta: Meta) -> Result<Value, JsonRPCError> {
        self.validate_auth(meta)?;

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
        self.validate_auth(meta)?;

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

    /// Provides the current ledger digest and returns the Merkle paths for the given commitments
    pub async fn ledger_commitment_proofs_protected(self, params: Params, meta: Meta) -> Result<Value, JsonRPCError> {
        self.validate_auth(meta)?;

        let value = match params {
            Params::Array(arr) => arr,
            _ => return Err(JsonRPCError::invalid_request()),
        };

        let commitments: Vec<<Testnet1Parameters as Parameters>::RecordCommitment> = value
            .into_iter()
            .map(|value| {
                <Testnet1Parameters as Parameters>::RecordCommitment::from_bytes_le(
                    &hex::decode(serde_json::from_value::<String>(value).unwrap()).unwrap(),
                )
            })
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| JsonRPCError::invalid_params(format!("Invalid params: {}.", e)))?;

        match self.ledger_commitment_proofs(commitments) {
            Ok(result) => Ok(serde_json::to_value(result).expect("record serialization failed")),
            Err(e) => Err(JsonRPCError::invalid_params(e.to_string())),
        }
    }

    /// Expose the protected functions as RPC enpoints
    pub fn add_protected(&self, io: &mut MetaIoHandler<Meta>) {
        let mut d = IoDelegate::<Self, Meta>::new(Arc::new(self.clone()));

        d.add_method_with_meta("createrawtransaction", |rpc, params, meta| {
            let rpc = rpc.clone();
            rpc.create_raw_transaction_protected(params, meta)
        });
        d.add_method_with_meta("createtransactionauthorization", |rpc, params, meta| {
            let rpc = rpc.clone();
            rpc.create_transaction_authorization_protected(params, meta)
        });
        d.add_method_with_meta("createtransaction", |rpc, params, meta| {
            let rpc = rpc.clone();
            rpc.create_transaction_protected(params, meta)
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
        d.add_method_with_meta("ledgercommitmentproofs", |rpc, params, meta| {
            let rpc = rpc.clone();
            rpc.ledger_commitment_proofs_protected(params, meta)
        });

        io.extend_with(d)
    }
}

/// Functions that are sensitive and need to be protected with authentication.
/// The authentication logic is defined in `validate_auth`
impl<S: Storage + Send + Sync + 'static> ProtectedRpcFunctions for RpcImpl<S> {
    /// Generate a new account private key, account view key, and account address.
    fn create_account(&self) -> Result<RpcAccount, RpcError> {
        let account = Account::<Testnet1Parameters>::new(&mut thread_rng())?;

        Ok(RpcAccount {
            private_key: account.private_key().to_string(),
            view_key: account.view_key.to_string(),
            address: account.address.to_string(),
        })
    }

    // TODO (raychu86): Deprecate this rpc endpoint in favor of the more secure offline/online model.
    /// Create a new transaction, returning the encoded transaction and the new records.
    fn create_raw_transaction(
        &self,
        transaction_input: TransactionInputs,
    ) -> Result<CreateRawTransactionOutput, RpcError> {
        let rng = &mut thread_rng();

        assert!(!transaction_input.old_records.is_empty());
        assert!(transaction_input.old_records.len() <= Testnet1Parameters::NUM_INPUT_RECORDS);
        assert!(!transaction_input.old_account_private_keys.is_empty());
        assert!(transaction_input.old_account_private_keys.len() <= Testnet1Parameters::NUM_OUTPUT_RECORDS);
        assert!(!transaction_input.recipients.is_empty());
        assert!(transaction_input.recipients.len() <= Testnet1Parameters::NUM_OUTPUT_RECORDS);

        // Decode old records
        let mut old_records = Vec::with_capacity(transaction_input.old_records.len());
        for record_string in transaction_input.old_records {
            let record_bytes = hex::decode(record_string)?;
            old_records.push(Record::<Testnet1Parameters>::read_le(&record_bytes[..])?);
        }

        let mut old_account_private_keys = Vec::with_capacity(transaction_input.old_account_private_keys.len());
        for private_key_string in transaction_input.old_account_private_keys {
            old_account_private_keys.push(PrivateKey::<Testnet1Parameters>::from_str(&private_key_string)?);
        }

        let sn_randomness: [u8; 32] = rng.gen();
        // Fill any unused old_record indices with dummy records
        while old_records.len() < Testnet1Parameters::NUM_OUTPUT_RECORDS {
            let old_sn_nonce = Testnet1Parameters::serial_number_nonce_crh().hash(&sn_randomness)?;

            let private_key = old_account_private_keys[0].clone();
            let address = Address::<Testnet1Parameters>::from_private_key(&private_key)?;

            let dummy_record = Record::<Testnet1Parameters>::new_input(
                self.dpc()?.noop_program.deref(),
                address,
                true, // The input record is dummy
                0,
                Payload::default(),
                old_sn_nonce,
                rng.gen(),
            )?;

            old_records.push(dummy_record);
            old_account_private_keys.push(private_key);
        }

        assert_eq!(old_records.len(), Testnet1Parameters::NUM_INPUT_RECORDS);
        assert_eq!(old_account_private_keys.len(), Testnet1Parameters::NUM_INPUT_RECORDS);

        // Decode new recipient data
        let mut new_record_owners = Vec::with_capacity(Testnet1Parameters::NUM_OUTPUT_RECORDS);
        let mut new_is_dummy_flags = Vec::with_capacity(Testnet1Parameters::NUM_OUTPUT_RECORDS);
        let mut new_values = Vec::with_capacity(Testnet1Parameters::NUM_OUTPUT_RECORDS);
        for recipient in transaction_input.recipients {
            new_record_owners.push(Address::<Testnet1Parameters>::from_str(&recipient.address)?);
            new_is_dummy_flags.push(false);
            new_values.push(recipient.amount);
        }

        // Fill any unused new_record indices with dummy output values
        while new_record_owners.len() < Testnet1Parameters::NUM_OUTPUT_RECORDS {
            new_record_owners.push(new_record_owners[0].clone());
            new_is_dummy_flags.push(true);
            new_values.push(0);
        }

        assert_eq!(new_record_owners.len(), Testnet1Parameters::NUM_OUTPUT_RECORDS);
        assert_eq!(new_is_dummy_flags.len(), Testnet1Parameters::NUM_OUTPUT_RECORDS);
        assert_eq!(new_values.len(), Testnet1Parameters::NUM_OUTPUT_RECORDS);

        let mut joint_serial_numbers = vec![];
        for i in 0..Testnet1Parameters::NUM_INPUT_RECORDS {
            let (sn, _) = old_records[i].to_serial_number(&old_account_private_keys[i].compute_key())?;
            joint_serial_numbers.extend_from_slice(&to_bytes_le![sn]?);
        }

        let mut new_records = vec![];
        for j in 0..Testnet1Parameters::NUM_OUTPUT_RECORDS {
            new_records.push(Record::new_output(
                self.dpc()?.noop_program.deref(),
                new_record_owners[j].clone(),
                new_is_dummy_flags[j],
                new_values[j],
                Payload::default(),
                (Testnet1Parameters::NUM_INPUT_RECORDS + j) as u8,
                &joint_serial_numbers,
                rng,
            )?);
        }

        // Decode memo.
        let mut memo = None;
        if let Some(memo_string) = transaction_input.memo {
            if let Ok(bytes) = hex::decode(memo_string) {
                let mut memo_buffer = [0u8; 64];
                bytes.write_le(&mut memo_buffer[..])?;
                memo = Some(memo_buffer);
            }
        }

        // Generate transaction
        let transaction = self.sync_handler()?.consensus.create_transaction(
            old_records,
            old_account_private_keys,
            new_records,
            memo,
            rng,
        )?;

        Ok(CreateRawTransactionOutput {
            encoded_transaction: hex::encode(to_bytes_le![transaction]?),
        })
    }

    /// Generates and returns a new transaction authorization.
    fn create_transaction_authorization(&self, transaction_input: TransactionInputs) -> Result<String, RpcError> {
        let rng = &mut thread_rng();

        assert!(!transaction_input.old_records.is_empty());
        assert!(transaction_input.old_records.len() <= Testnet1Parameters::NUM_INPUT_RECORDS);
        assert!(!transaction_input.old_account_private_keys.is_empty());
        assert!(transaction_input.old_account_private_keys.len() <= Testnet1Parameters::NUM_OUTPUT_RECORDS);
        assert!(!transaction_input.recipients.is_empty());
        assert!(transaction_input.recipients.len() <= Testnet1Parameters::NUM_OUTPUT_RECORDS);

        let mut builder = TransactionAuthorizationBuilder::new();

        // Add individual transaction inputs to the transaction authorization builder.
        for (record_string, private_key_string) in transaction_input
            .old_records
            .iter()
            .zip_eq(&transaction_input.old_account_private_keys)
        {
            let record = Record::<Testnet1Parameters>::from_str(record_string)?;
            let private_key = PrivateKey::<Testnet1Parameters>::from_str(private_key_string)?;

            builder = builder.add_input(private_key, record)?;
        }

        // Add individual transaction outputs to the transaction authorization builder.
        for recipient in &transaction_input.recipients {
            let address = Address::<Testnet1Parameters>::from_str(&recipient.address)?;

            builder = builder.add_output(address, recipient.amount)?;
        }

        // Decode memo.
        let mut memo = [0u8; 64];
        (0..64)
            .map(|_| u8::rand(rng))
            .collect::<Vec<u8>>()
            .write_le(&mut memo[..])?;
        if let Some(memo_string) = transaction_input.memo {
            if let Ok(bytes) = hex::decode(memo_string) {
                bytes.write_le(&mut memo[..])?;
            }
        }

        // Set the memo in the transaction authorization builder.
        builder = builder.memo(memo);

        // Set the network id in the transaction authorization builder.
        builder = builder.network_id(transaction_input.network_id);

        // Construct the transaction authorization.
        let authorization = builder.build(rng)?;

        Ok(hex::encode(authorization.to_bytes()))
    }

    /// Create a new transaction for a given transaction authorization.
    fn create_transaction(
        &self,
        private_keys_string: [String; Testnet1Parameters::NUM_INPUT_RECORDS],
        authorization: String,
    ) -> Result<CreateRawTransactionOutput, RpcError> {
        let rng = &mut thread_rng();

        // Decode the compute keys.
        let mut compute_keys = Vec::with_capacity(Testnet1Parameters::NUM_INPUT_RECORDS);
        for private_key in private_keys_string {
            compute_keys.push(
                PrivateKey::<Testnet1Parameters>::from_str(&private_key)?
                    .compute_key()
                    .clone(),
            );
        }

        // Decode the transaction authorization.
        let authorization = TransactionAuthorization::<Testnet1Parameters>::read_le(&hex::decode(authorization)?[..])?;

        // TODO (raychu86): Genericize this model to allow for generic programs.
        // Construct the executable.
        let noop = Executable::Noop(Arc::new(self.dpc()?.noop_program.clone()));
        let executables = vec![noop.clone(), noop.clone(), noop.clone(), noop];

        // Online execution to generate a transaction
        let transaction = self
            .dpc()?
            .execute(&compute_keys, authorization, &executables, &*self.storage, rng)?;

        Ok(CreateRawTransactionOutput {
            encoded_transaction: hex::encode(to_bytes_le![transaction]?),
        })
    }

    /// Returns the number of record commitments that are stored on the full node.
    fn get_record_commitment_count(&self) -> Result<usize, RpcError> {
        let storage = &self.storage;
        let primary_height = self.sync_handler()?.current_block_height();
        storage.catch_up_secondary(false, primary_height)?;

        let record_commitments = storage.get_record_commitments(None)?;

        Ok(record_commitments.len())
    }

    /// Returns a list of record commitments that are stored on the full node.
    fn get_record_commitments(&self) -> Result<Vec<String>, RpcError> {
        let storage = &self.storage;
        let primary_height = self.sync_handler()?.current_block_height();
        storage.catch_up_secondary(false, primary_height)?;

        let record_commitments = storage.get_record_commitments(Some(100))?;
        let record_commitment_strings: Vec<String> = record_commitments.iter().map(hex::encode).collect();

        Ok(record_commitment_strings)
    }

    /// Returns the hex encoded bytes of a record from its record commitment
    fn get_raw_record(&self, record_commitment: String) -> Result<String, RpcError> {
        match self
            .storage
            .get_record::<Record<Testnet1Parameters>>(&hex::decode(record_commitment)?)?
        {
            Some(record) => {
                let record_bytes = to_bytes_le![record]?;
                Ok(hex::encode(record_bytes))
            }
            None => Ok("Record not found".to_string()),
        }
    }

    /// Decrypts the record ciphertext and returns the hex encoded bytes of the record.
    fn decrypt_record(&self, decryption_input: DecryptRecordInput) -> Result<String, RpcError> {
        // Read the encrypted_record
        let encrypted_record_bytes = hex::decode(decryption_input.encrypted_record)?;
        let encrypted_record = EncryptedRecord::<Testnet1Parameters>::read_le(&encrypted_record_bytes[..])?;

        // Read the view key
        let view_key = ViewKey::<Testnet1Parameters>::from_str(&decryption_input.account_view_key)?;

        // Decrypt the record ciphertext
        let record = encrypted_record.decrypt(&view_key)?;

        Ok(record.to_string())
    }

    fn disconnect(&self, address: SocketAddr) {
        let node = self.node.clone();
        tokio::spawn(async move { node.disconnect_from_peer(address).await });
    }

    fn connect(&self, addresses: Vec<SocketAddr>) {
        let node = self.node.clone();
        tokio::spawn(async move {
            for addr in &addresses {
                node.peer_book.add_peer(*addr, false).await;
            }
            node.connect_to_addresses(&addresses).await
        });
    }

    fn ledger_commitment_proofs(
        &self,
        cms: Vec<<Testnet1Parameters as Parameters>::RecordCommitment>,
    ) -> Result<
        (
            MerkleTreeDigest<<Testnet1Parameters as Parameters>::RecordCommitmentTreeParameters>,
            Vec<LeanMerklePath>,
        ),
        RpcError,
    > {
        // Check the commitment count.
        let expected_cm_count = <Testnet1Parameters as Parameters>::NUM_INPUT_RECORDS;
        if cms.len() != expected_cm_count {
            return Err(RpcError::InvalidCommitmentCount(cms.len(), expected_cm_count));
        }

        // Fetch the latest digest.
        let storage = &self.storage;
        let primary_height = self.sync_handler()?.current_block_height();
        storage.catch_up_secondary(false, primary_height)?;
        let latest_digest = self.storage.latest_digest()?;

        // Generate the Merkle path for each commitment to the latest digest.
        let paths = cms
            .into_iter()
            .map(|cm| storage.prove_cm(&cm).map(LeanMerklePath::from))
            .collect::<Result<Vec<_>, _>>()?;

        Ok((latest_digest, paths))
    }
}
