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

use crate::{
    sync::{create_test_consensus, TestBlocks},
};
use rand::{Rng, thread_rng};
use snarkos_storage::VMBlock;
use snarkvm_dpc::{testnet1::instantiated::Testnet1Transaction, Block};
use snarkvm_utilities::FromBytes;

use std::{env, fs, io};

pub fn random_storage_path() -> String {
    let random_path: usize = thread_rng().gen();
    format!("./test_db-{}", random_path)
}

#[tokio::test]
async fn import_export_blocks() {
    // Create an instance that loads some test blocks.
    let consensus = create_test_consensus();
    let test_blocks = TestBlocks::load(Some(10), "test_blocks_100_1").0;
    for block in &test_blocks {
        assert!(consensus.receive_block(block.clone()).await);
    }

    // Export the canon blocks to a temp file.
    let mut path = env::temp_dir();
    path.push(random_storage_path());
    snarkos_storage::export_canon_blocks(consensus.storage.clone(), 0, &path)
        .await
        .unwrap();

    // Ensure that the exported blocks are the same as the ones initially imported.
    let mut imported_blocks = io::Cursor::new(fs::read(&path).unwrap());

    for test_block in test_blocks {
        let imported_block: Block<Testnet1Transaction> = FromBytes::read_le(&mut imported_blocks).unwrap();
        let imported_block = <Block<_> as VMBlock>::serialize(&imported_block).unwrap();
        assert_eq!(imported_block, test_block);
    }

    // Clean up the test file.
    let _ = fs::remove_file(&path);
}
