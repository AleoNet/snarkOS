// Copyright (C) 2019-2020 Aleo Systems Inc.
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

use snarkvm_errors::{
    algorithms::MerkleError,
    objects::{BlockError, TransactionError},
    parameters::ParametersError,
};

use std::fmt::Debug;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("{}: {}", _0, _1)]
    Crate(&'static str, String),

    #[error("duplicate commitment")]
    DuplicateCm,

    #[error("duplicate serial number")]
    DuplicateSn,

    #[error("duplicate transaction memo")]
    DuplicateMemo,

    #[error("Block has already been committed: {:?}", _0)]
    ExistingCanonBlock(String),

    #[error("existing record commitment {:?}", _0)]
    ExistingCm(Vec<u8>),

    #[error("existing transaction memo {:?}", _0)]
    ExistingMemo(Vec<u8>),

    #[error("existing serial number {:?}", _0)]
    ExistingSn(Vec<u8>),

    #[error("Can't decommit the genesis block")]
    InvalidBlockDecommit,

    #[error("Can't remove a canon block with hash")]
    InvalidBlockRemovalCanon(String),

    #[error("invalid number of blocks to remove {}. There are only {} existing blocks", _0, _1)]
    InvalidBlockRemovalNum(u32, u32),

    #[error("invalid column family {}", _0)]
    InvalidColumnFamily(u32),

    #[error("missing outpoint with transaction with id {} and index {}", _0, _1)]
    InvalidOutpoint(String, usize),

    #[error("missing transaction with id {}", _0)]
    InvalidTransactionId(String),

    #[error("{}", _0)]
    Message(String),

    #[error("missing block hash value given block number {}", _0)]
    MissingBlockHash(u32),

    #[error("missing block header value given block hash {}", _0)]
    MissingBlockHeader(String),

    #[error("missing block number value given block hash {}", _0)]
    MissingBlockNumber(String),

    #[error("missing block transactions value for block hash {}", _0)]
    MissingBlockTransactions(String),

    #[error("missing child block hashes value for block hash {}", _0)]
    MissingChildBlock(String),

    #[error("missing current commitment index")]
    MissingCurrentCmIndex,

    #[error("missing current merkle tree digest")]
    MissingCurrentDigest,

    #[error("missing current memo index")]
    MissingCurrentMemoIndex,

    #[error("missing current serial number index")]
    MissingCurrentSnIndex,

    #[error("missing genesis address")]
    MissingGenesisAccount,

    #[error("missing genesis commitment")]
    MissingGenesisCm,

    #[error("missing genesis memo")]
    MissingGenesisMemo,

    #[error("missing genesis program vk bytes")]
    MissingGenesisProgramVkBytes,

    #[error("missing genesis serial number")]
    MissingGenesisSn,

    #[error("missing transaction meta value for transaction id {}", _0)]
    MissingTransactionMeta(String),

    #[error("missing value given key {}", _0)]
    MissingValue(String),

    #[error("Null Error {:?}", _0)]
    NullError(()),

    #[error("{}", _0)]
    BlockError(BlockError),

    #[error("{}", _0)]
    MerkleError(MerkleError),

    #[error("{}", _0)]
    ParametersError(ParametersError),

    #[error("{}", _0)]
    TransactionError(TransactionError),
}

impl From<bincode::Error> for StorageError {
    fn from(error: bincode::Error) -> Self {
        StorageError::Crate("bincode", format!("{:?}", error))
    }
}

impl From<hex::FromHexError> for StorageError {
    fn from(error: hex::FromHexError) -> Self {
        StorageError::Crate("hex", format!("{:?}", error))
    }
}

#[cfg(feature = "librocksdb")]
impl From<rocksdb::Error> for StorageError {
    fn from(error: rocksdb::Error) -> Self {
        StorageError::Crate("rocksdb", format!("{:?}", error))
    }
}

impl From<std::io::Error> for StorageError {
    fn from(error: std::io::Error) -> Self {
        StorageError::Crate("std::io", format!("{:?}", error))
    }
}

impl From<()> for StorageError {
    fn from(_error: ()) -> Self {
        StorageError::NullError(())
    }
}

impl From<&'static str> for StorageError {
    fn from(msg: &'static str) -> Self {
        StorageError::Message(msg.into())
    }
}

impl From<BlockError> for StorageError {
    fn from(error: BlockError) -> Self {
        StorageError::BlockError(error)
    }
}

impl From<MerkleError> for StorageError {
    fn from(error: MerkleError) -> Self {
        StorageError::MerkleError(error)
    }
}

impl From<ParametersError> for StorageError {
    fn from(error: ParametersError) -> Self {
        StorageError::ParametersError(error)
    }
}

impl From<TransactionError> for StorageError {
    fn from(error: TransactionError) -> Self {
        StorageError::TransactionError(error)
    }
}
