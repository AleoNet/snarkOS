// Copyright (C) 2019-2022 Aleo Systems Inc.
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

use crate::{
    rocksdb::{self, DataMap, Database},
    DataID,
    TransactionDB,
    TransitionDB,
};
use snarkvm::prelude::*;

/// A RocksDB block storage.
#[derive(Clone)]
pub struct BlockDB<N: Network> {
    /// The mapping of `block height` to `state root`.
    state_root_map: DataMap<u32, N::StateRoot>,
    /// The mapping of `state root` to `block height`.
    reverse_state_root_map: DataMap<N::StateRoot, u32>,
    /// The mapping of `block height` to `block hash`.
    id_map: DataMap<u32, N::BlockHash>,
    /// The mapping of `block hash` to `block height`.
    reverse_id_map: DataMap<N::BlockHash, u32>,
    /// The header map.
    header_map: DataMap<N::BlockHash, Header<N>>,
    /// The transactions map.
    transactions_map: DataMap<N::BlockHash, Vec<N::TransactionID>>,
    /// The reverse transactions map.
    reverse_transactions_map: DataMap<N::TransactionID, N::BlockHash>,
    /// The transaction store.
    transaction_store: TransactionStore<N, TransactionDB<N>>,
    /// The coinbase solution map.
    coinbase_solution_map: DataMap<N::BlockHash, Option<CoinbaseSolution<N>>>,
    /// The coinbase puzzle commitment map.
    coinbase_puzzle_commitment_map: DataMap<PuzzleCommitment<N>, N::BlockHash>,
    /// The signature map.
    signature_map: DataMap<N::BlockHash, Signature<N>>,
}

#[rustfmt::skip]
impl<N: Network> BlockStorage<N> for BlockDB<N> {
    type StateRootMap = DataMap<u32, N::StateRoot>;
    type ReverseStateRootMap = DataMap<N::StateRoot, u32>;
    type IDMap = DataMap<u32, N::BlockHash>;
    type ReverseIDMap = DataMap<N::BlockHash, u32>;
    type HeaderMap = DataMap<N::BlockHash, Header<N>>;
    type TransactionsMap = DataMap<N::BlockHash, Vec<N::TransactionID>>;
    type ReverseTransactionsMap = DataMap<N::TransactionID, N::BlockHash>;
    type TransactionStorage = TransactionDB<N>;
    type TransitionStorage = TransitionDB<N>;
    type CoinbaseSolutionMap = DataMap<N::BlockHash, Option<CoinbaseSolution<N>>>;
    type CoinbasePuzzleCommitmentMap = DataMap<PuzzleCommitment<N>, N::BlockHash>;
    type SignatureMap = DataMap<N::BlockHash, Signature<N>>;

    /// Initializes the block storage.
    fn open(dev: Option<u16>) -> Result<Self> {
        // Initialize the transition store.
        let transition_store = TransitionStore::<N, TransitionDB<N>>::open(dev)?;
        // Initialize the transaction store.
        let transaction_store = TransactionStore::<N, TransactionDB<N>>::open(transition_store)?;
        // Return the block storage.
        Ok(Self {
            state_root_map: rocksdb::RocksDB::open_map(N::ID, dev, DataID::BlockStateRootMap)?,
            reverse_state_root_map: rocksdb::RocksDB::open_map(N::ID, dev, DataID::BlockReverseStateRootMap)?,
            id_map: rocksdb::RocksDB::open_map(N::ID, dev, DataID::BlockIDMap)?,
            reverse_id_map: rocksdb::RocksDB::open_map(N::ID, dev, DataID::BlockReverseIDMap)?,
            header_map: rocksdb::RocksDB::open_map(N::ID, dev, DataID::BlockHeaderMap)?,
            transactions_map: rocksdb::RocksDB::open_map(N::ID, dev, DataID::BlockTransactionsMap)?,
            reverse_transactions_map: rocksdb::RocksDB::open_map(N::ID, dev, DataID::BlockReverseTransactionsMap)?,
            transaction_store,
            coinbase_solution_map: rocksdb::RocksDB::open_map(N::ID, dev, DataID::BlockCoinbaseSolutionMap)?,
            coinbase_puzzle_commitment_map: rocksdb::RocksDB::open_map(N::ID, dev, DataID::BlockCoinbasePuzzleCommitmentMap)?,
            signature_map: rocksdb::RocksDB::open_map(N::ID, dev, DataID::BlockSignatureMap)?,
        })
    }

    /// Returns the state root map.
    fn state_root_map(&self) -> &Self::StateRootMap {
        &self.state_root_map
    }

    /// Returns the reverse state root map.
    fn reverse_state_root_map(&self) -> &Self::ReverseStateRootMap {
        &self.reverse_state_root_map
    }

    /// Returns the ID map.
    fn id_map(&self) -> &Self::IDMap {
        &self.id_map
    }

    /// Returns the reverse ID map.
    fn reverse_id_map(&self) -> &Self::ReverseIDMap {
        &self.reverse_id_map
    }

    /// Returns the header map.
    fn header_map(&self) -> &Self::HeaderMap {
        &self.header_map
    }

    /// Returns the transactions map.
    fn transactions_map(&self) -> &Self::TransactionsMap {
        &self.transactions_map
    }

    /// Returns the reverse transactions map.
    fn reverse_transactions_map(&self) -> &Self::ReverseTransactionsMap {
        &self.reverse_transactions_map
    }

    /// Returns the transaction store.
    fn transaction_store(&self) -> &TransactionStore<N, Self::TransactionStorage> {
        &self.transaction_store
    }

    /// Returns the coinbase solution map.
    fn coinbase_solution_map(&self) -> &Self::CoinbaseSolutionMap {
        &self.coinbase_solution_map
    }

    /// Returns the coinbase puzzle commitment map.
    fn coinbase_puzzle_commitment_map(&self) -> &Self::CoinbasePuzzleCommitmentMap {
        &self.coinbase_puzzle_commitment_map
    }

    /// Returns the signature map.
    fn signature_map(&self) -> &Self::SignatureMap {
        &self.signature_map
    }
}
