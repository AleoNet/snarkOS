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

use std::sync::Arc;

use rand::{thread_rng, Rng};
use snarkos_storage::{Address, Digest, DynStorage, SerialBlock, SerialTransaction, VMRecord};
use snarkvm_algorithms::CRH;
use snarkvm_dpc::{Account, AccountScheme, DPCComponents, testnet1::{instantiated::{Components, Testnet1DPC}, payload::Payload, Record as DPCRecord}};
use snarkvm_utilities::{to_bytes_le, ToBytes};
use tokio::sync::{mpsc, oneshot};

use crate::{error::ConsensusError, ConsensusParameters, DynLedger, MemoryPool};

use self::message::{ConsensusMessage, TransactionResponse};
use anyhow::*;

mod inner;
pub use inner::ConsensusInner;
mod message;
pub use message::*;

pub struct Consensus {
    pub parameters: ConsensusParameters,
    pub dpc: Arc<Testnet1DPC>,
    pub storage: DynStorage,
    genesis_block: SerialBlock,
    sender: mpsc::Sender<ConsensusMessageWrapped>,
}

impl Consensus {
    /// Creates a new consensus instance with the given parameters, genesis, ledger, storage, and memory pool.
    pub fn new(
        parameters: ConsensusParameters,
        dpc: Arc<Testnet1DPC>,
        genesis_block: SerialBlock,
        ledger: DynLedger,
        storage: DynStorage,
        memory_pool: MemoryPool,
    ) -> Arc<Self> {
        let (sender, receiver) = mpsc::channel(256);
        let created = Arc::new(Self {
            parameters,
            dpc,
            storage: storage.clone(),
            genesis_block,
            sender,
        });

        let created2 = created.clone();
        tokio::spawn(async move {
            ConsensusInner {
                public: created2,
                ledger,
                storage,
                memory_pool,
            }
            .agent(receiver)
            .await;
        });

        created
    }

    #[allow(clippy::ok_expect)] // SendError is not Debug
    async fn send<T: Send + Sync + 'static>(&self, message: ConsensusMessage) -> T {
        let (sender, receiver) = oneshot::channel();
        self.sender.send((message, sender)).await.ok();
        *receiver
            .await
            .ok()
            .expect("consensus agent missing")
            .downcast()
            .expect("type mismatch for consensus agent handle")
    }

    /// Receives a live transaction (into the memory pool)
    pub async fn receive_transaction(&self, transaction: SerialTransaction) -> bool {
        self.send(ConsensusMessage::ReceiveTransaction(Box::new(transaction)))
            .await
    }

    /// Verify a set of transactions
    /// Used for tests and RPC
    pub async fn verify_transactions(&self, transactions: Vec<SerialTransaction>) -> bool {
        self.send(ConsensusMessage::VerifyTransactions(transactions)).await
    }

    /// Receives any block into consensus
    pub async fn receive_block(&self, block: SerialBlock) -> bool {
        self.send(ConsensusMessage::ReceiveBlock(Box::new(block))).await
    }

    /// Fetches a snapshot of the memory pool
    pub async fn fetch_memory_pool(&self) -> Vec<SerialTransaction> {
        self.send(ConsensusMessage::FetchMemoryPool(self.parameters.max_block_size))
            .await
    }

    /// Creates a new valid transaction
    pub async fn create_transaction(
        &self,
        request: CreateTransactionRequest,
    ) -> Result<TransactionResponse, ConsensusError> {
        self.send(ConsensusMessage::CreateTransaction(Box::new(request))).await
    }

    /// Creates a new valid transaction from a pre-formed snarkvm transaction kernel
    /// Used for RPC
    pub async fn create_partial_transaction(
        &self,
        request: CreatePartialTransactionRequest,
    ) -> Result<TransactionResponse, ConsensusError> {
        self.send(ConsensusMessage::CreatePartialTransaction(request)).await
    }

    /// Forcefully decommit a block hash and its decendents
    /// Used for testing
    pub async fn force_decommit(&self, hash: Digest) -> Result<(), ConsensusError> {
        self.send(ConsensusMessage::ForceDecommit(hash.0.to_vec())).await
    }

    /// Initiate a fast forward operation
    /// Used for testing/rectifying use of `force_decommit`
    pub async fn fast_forward(&self) -> Result<(), ConsensusError> {
        self.send(ConsensusMessage::FastForward()).await
    }

    /// Diagnostic function to scan for valid forks
    pub async fn scan_forks(&self) -> Result<()> {
        self.send(ConsensusMessage::ScanForks()).await
    }

    /// Diagnostic function to rebuild the stored ledger components
    pub async fn recommit_canon(&self) -> Result<()> {
        self.send(ConsensusMessage::RecommitCanon()).await
    }

    /// Generate a coinbase transaction given candidate block transactions
    #[allow(clippy::too_many_arguments)]
    pub async fn create_coinbase_transaction(
        &self,
        block_num: u32,
        transactions: &[SerialTransaction],
        program_vk_hash: Vec<u8>,
        new_birth_program_ids: Vec<Vec<u8>>,
        new_death_program_ids: Vec<Vec<u8>>,
        recipients: Vec<Address>,
    ) -> Result<TransactionResponse, ConsensusError> {
        let mut rng = thread_rng();
        let mut total_value_balance = crate::get_block_reward(block_num);

        for transaction in transactions.iter() {
            let tx_value_balance = transaction.value_balance;

            if tx_value_balance.is_negative() {
                return Err(ConsensusError::CoinbaseTransactionAlreadyExists());
            }

            total_value_balance = total_value_balance.add(transaction.value_balance);
        }

        // Generate a new account that owns the dummy input records
        let new_account = Account::<Components>::new(
            &self.dpc.system_parameters.account_signature,
            &self.dpc.system_parameters.account_commitment,
            &self.dpc.system_parameters.account_encryption,
            &mut rng,
        )
        .unwrap();

        // Generate dummy input records having as address the genesis address.
        let old_account_private_keys = vec![new_account.private_key.clone(); Components::NUM_INPUT_RECORDS]
            .into_iter()
            .map(|x| x.into())
            .collect::<Vec<_>>();
        let mut old_records = Vec::with_capacity(Components::NUM_INPUT_RECORDS);
        let mut joint_serial_numbers = vec![];

        for i in 0..Components::NUM_INPUT_RECORDS {
            let sn_nonce_input: [u8; 4] = rng.gen();

            let old_sn_nonce = <Components as DPCComponents>::SerialNumberNonceCRH::hash(
                &self.dpc.system_parameters.serial_number_nonce,
                &sn_nonce_input,
            )?;

            let old_record = DPCRecord::new(
                &self.dpc.system_parameters.record_commitment,
                new_account.address.clone(),
                true, // The input record is dummy
                0,
                Payload::default(),
                // Filler program input
                program_vk_hash.clone(),
                program_vk_hash.clone(),
                old_sn_nonce,
                &mut rng,
            )?;

            let (sn, _) = old_record.to_serial_number(&self.dpc.system_parameters.account_signature, &old_account_private_keys[i])?;
            joint_serial_numbers.extend_from_slice(&to_bytes_le![sn]?);

            old_records.push(old_record.serialize()?);
        }

        let new_is_dummy_flags = [vec![false], vec![true; Components::NUM_OUTPUT_RECORDS - 1]].concat();
        let new_values = [vec![total_value_balance.0 as u64], vec![
            0;
            Components::NUM_OUTPUT_RECORDS
                - 1
        ]]
        .concat();
        let new_payloads = vec![Payload::default(); Components::NUM_OUTPUT_RECORDS];

        let mut new_records = vec![];
        for j in 0..Components::NUM_OUTPUT_RECORDS {
            new_records.push(DPCRecord::new_full(
                &self.dpc.system_parameters.serial_number_nonce,
                &self.dpc.system_parameters.record_commitment,
                recipients[j].clone().into(),
                new_is_dummy_flags[j],
                new_values[j],
                new_payloads[j].clone(),
                new_birth_program_ids[j].clone(),
                new_death_program_ids[j].clone(),
                j as u8,
                joint_serial_numbers.clone(),
                &mut rng,
            )?.serialize()?);
        }

        let memo: [u8; 32] = rng.gen();

        self.create_transaction(CreateTransactionRequest {
            old_records,
            old_account_private_keys: old_account_private_keys.into_iter().map(|x| x.into()).collect(),
            new_records,
            memo,
        })
        .await
    }
}
