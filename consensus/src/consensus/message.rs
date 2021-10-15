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

use snarkos_storage::PrivateKey;
use snarkvm_dpc::{Block, Record, Transaction};

use std::any::Any;
use tokio::sync::oneshot;

#[derive(Debug)]
pub struct CreateTransactionRequest {
    pub old_records: Vec<Record<N>>,
    pub old_account_private_keys: Vec<PrivateKey>,
    pub new_records: Vec<Record<N>>,
    pub memo: [u8; 32],
}
pub struct CreatePartialTransactionRequest {
    // TransactionKernel
    pub kernel: Box<dyn Any + Send + Sync>,
    pub old_account_private_keys: Vec<PrivateKey>,
}

pub struct TransactionResponse {
    pub records: Vec<Record<N>>,
    pub transaction: Transaction<N>,
}

pub(super) enum ConsensusMessage {
    ReceiveTransaction(Box<Transaction<N>>),
    VerifyTransactions(Vec<Transaction<N>>),
    ReceiveBlock(Box<Block<N>>),
    FetchMemoryPool(usize), // max size of memory pool to fetch
    CreateTransaction(Box<CreateTransactionRequest>),
    CreatePartialTransaction(CreatePartialTransactionRequest),
    ForceDecommit(Vec<u8>),
    FastForward(),
    #[cfg(feature = "test")]
    Reset(),
}

pub(super) type ConsensusMessageWrapped = (ConsensusMessage, oneshot::Sender<Box<dyn Any + Send + Sync>>);
