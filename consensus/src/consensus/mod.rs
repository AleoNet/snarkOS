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

use std::{convert::TryInto, sync::Arc};

use rand::{thread_rng, Rng};
use snarkos_storage::{Address, Digest, DynStorage, SerialBlock, SerialRecord, SerialTransaction, VMRecord};
use snarkvm_algorithms::CRH;
use snarkvm_dpc::{
    testnet1::{
        instantiated::{Components, Testnet1DPC},
        payload::Payload,
        Record as DPCRecord,
    },
    Account,
    AccountScheme,
    AleoAmount,
    DPCComponents,
    ProgramScheme,
};
use snarkvm_utilities::{to_bytes_le, ToBytes};
use tokio::sync::{mpsc, oneshot};

use crate::{error::ConsensusError, ConsensusParameters, DynLedger, MemoryPool};

use anyhow::*;

mod inner;
pub use inner::ConsensusInner;
mod message;
pub use message::*;
mod utility;

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
    pub async fn scan_forks(&self) -> Result<Vec<(Digest, Digest)>> {
        self.send(ConsensusMessage::ScanForks()).await
    }

    /// Diagnostic function to rebuild the stored ledger components
    pub async fn recommit_canon(&self) -> Result<()> {
        self.send(ConsensusMessage::RecommitCanon()).await
    }
}
