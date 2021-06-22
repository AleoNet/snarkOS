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

#[cfg(test)]
use crate::sync::{create_test_consensus, TestBlocks};

use crate::sync::TestTx;
pub use snarkos_storage::{validator::FixMode, Ledger, LedgerStorage};
use snarkvm_algorithms::traits::merkle_tree::LoadableMerkleParameters;
use snarkvm_dpc::{
    testnet1::instantiated::CommitmentMerkleParameters,
    Block,
    LedgerScheme,
    Storage,
    TransactionScheme,
};

use rand::{thread_rng, Rng};
use std::sync::Arc;

pub type Store = Ledger<TestTx, CommitmentMerkleParameters, LedgerStorage>;

pub fn random_storage_path() -> String {
    let random_path: usize = thread_rng().gen();
    format!("./test_db-{}", random_path)
}

// Initialize a test blockchain given genesis attributes
pub fn initialize_test_blockchain<T: TransactionScheme, P: LoadableMerkleParameters, S: Storage>(
    parameters: Arc<P>,
    genesis_block: Block<T>,
) -> Ledger<T, P, S> {
    let mut path = std::env::temp_dir();
    path.push(random_storage_path());

    Ledger::<T, P, S>::new(Some(&path), parameters, genesis_block).unwrap()
}

#[tokio::test]
async fn valid_storage_validates() {
    //tracing_subscriber::fmt::init();

    let consensus = create_test_consensus();

    let blocks = TestBlocks::load(Some(10), "test_blocks_100_1").0;
    for block in blocks {
        consensus.receive_block(&block).await.unwrap();
    }

    assert!(consensus.ledger.validate(None, FixMode::Everything));
}
