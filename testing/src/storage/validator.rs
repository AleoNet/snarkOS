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

use crate::sync::{create_test_consensus_memdb, TestBlocks};
use snarkos_storage::{
    key_value::{KeyValueColumn, KEY_CURR_CM_INDEX, KEY_CURR_MEMO_INDEX, KEY_CURR_SN_INDEX},
    FixMode,
    ValidatorError,
};

use rand::prelude::*;

#[tokio::test]
async fn valid_storage_validates() {
    let consensus = create_test_consensus_memdb().await;

    let blocks = TestBlocks::load(Some(5), "test_blocks_100_1").0;
    for block in blocks {
        consensus.receive_block(block).await;
    }

    assert!(consensus.storage.validate(None, FixMode::Nothing).await.is_empty());
}

#[tokio::test]
async fn validator_vs_a_missing_serial_number() {
    let consensus = create_test_consensus_memdb().await;

    let blocks = TestBlocks::load(Some(5), "test_blocks_100_1").0;
    for block in blocks {
        consensus.receive_block(block).await;
    }

    // Remove a random tx serial number.
    let stored_sns = consensus.storage.get_serial_numbers().await.unwrap();
    let random_sn = &stored_sns.choose(&mut thread_rng()).unwrap().0;
    consensus
        .storage
        .delete_item(KeyValueColumn::SerialNumber, random_sn.to_vec())
        .await
        .unwrap();

    let errors = consensus.storage.validate(None, FixMode::Nothing).await;

    assert!(errors.len() <= 2); // the index could have become incontiguous too
    assert!(errors.contains(&ValidatorError::StorageEntryMissing(
        KeyValueColumn::SerialNumber,
        hex::encode(&random_sn)
    )));
    // Currently unsupported.
    // assert!(consensus.storage.validate(None, FixMode::MissingTestnet1TransactionComponents));
}

#[tokio::test]
async fn validator_vs_a_missing_commitment() {
    let consensus = create_test_consensus_memdb().await;

    let blocks = TestBlocks::load(Some(5), "test_blocks_100_1").0;
    for block in blocks {
        consensus.receive_block(block).await;
    }

    // Remove a random tx commitment.
    let stored_cms = consensus.storage.get_commitments().await.unwrap();
    let random_cm = &stored_cms.choose(&mut thread_rng()).unwrap().0;
    consensus
        .storage
        .delete_item(KeyValueColumn::Commitment, random_cm.to_vec())
        .await
        .unwrap();

    let errors = consensus.storage.validate(None, FixMode::Nothing).await;

    assert!(errors.len() <= 2); // the index could have become incontiguous too
    assert!(errors.contains(&ValidatorError::StorageEntryMissing(
        KeyValueColumn::Commitment,
        hex::encode(&random_cm)
    )));
    // Currently unsupported
    // assert!(consensus.storage.validate(None, FixMode::MissingTestnet1TransactionComponents));
}

#[tokio::test]
async fn validator_vs_a_missing_memorandum() {
    let consensus = create_test_consensus_memdb().await;

    let blocks = TestBlocks::load(Some(5), "test_blocks_100_1").0;
    for block in blocks {
        consensus.receive_block(block).await;
    }

    // Remove a random memo.
    let stored_memos = consensus.storage.get_memos().await.unwrap();
    let random_memo = &stored_memos.choose(&mut thread_rng()).unwrap().0;
    consensus
        .storage
        .delete_item(KeyValueColumn::Memo, random_memo.to_vec())
        .await
        .unwrap();

    let errors = consensus.storage.validate(None, FixMode::Nothing).await;

    assert!(errors.len() <= 2); // the index could have become incontiguous too
    assert!(errors.contains(&ValidatorError::StorageEntryMissing(
        KeyValueColumn::Memo,
        hex::encode(&random_memo)
    )));
    // Currently unsupported
    // assert!(consensus.storage.validate(None, FixMode::MissingTestnet1TransactionComponents));
}

#[tokio::test]
async fn validator_vs_a_missing_digest() {
    let consensus = create_test_consensus_memdb().await;

    let blocks = TestBlocks::load(Some(5), "test_blocks_100_1").0;
    for block in blocks {
        consensus.receive_block(block).await;
    }

    // Remove a random digest.
    let stored_digests = consensus.storage.get_ledger_digests().await.unwrap();
    let random_digest = &stored_digests.choose(&mut thread_rng()).unwrap().0;
    consensus
        .storage
        .delete_item(KeyValueColumn::DigestIndex, random_digest.to_vec())
        .await
        .unwrap();

    let errors = consensus.storage.validate(None, FixMode::Nothing).await;

    assert!(errors.len() <= 2); // the index could have become incontiguous too
    assert!(errors.contains(&ValidatorError::StorageEntryMissing(
        KeyValueColumn::DigestIndex,
        hex::encode(&random_digest)
    )));
    assert!(
        consensus
            .storage
            .validate(None, FixMode::MissingTestnet1TxComponents)
            .await
            .is_empty()
    );
}

#[tokio::test]
async fn validator_vs_a_superfluous_serial_number() {
    let consensus = create_test_consensus_memdb().await;

    let blocks = TestBlocks::load(Some(5), "test_blocks_100_1").0;
    for block in blocks {
        consensus.receive_block(block).await;
    }

    // Add an extra random tx serial number.
    let next_sn_idx = consensus.storage.get_serial_numbers().await.unwrap().len();
    let sn_idx = (next_sn_idx as u32).to_le_bytes().to_vec();

    consensus
        .storage
        .store_item(KeyValueColumn::SerialNumber, vec![0; 32], sn_idx.clone())
        .await
        .unwrap();

    consensus
        .storage
        .store_item(KeyValueColumn::Meta, KEY_CURR_SN_INDEX.as_bytes().to_vec(), sn_idx)
        .await
        .unwrap();

    let errors = consensus.storage.validate(None, FixMode::Nothing).await;
    assert_eq!(errors.len(), 1);
    assert!(errors.contains(&ValidatorError::SuperfluousTxComponents(
        KeyValueColumn::SerialNumber,
        1
    )));

    assert!(
        consensus
            .storage
            .validate(None, FixMode::SuperfluousTestnet1TxComponents)
            .await
            .is_empty()
    );
}

#[tokio::test]
async fn validator_vs_a_superfluous_commitment() {
    let consensus = create_test_consensus_memdb().await;

    let blocks = TestBlocks::load(Some(5), "test_blocks_100_1").0;
    for block in blocks {
        consensus.receive_block(block).await;
    }

    // Add an extra random tx commitment.
    let next_cm_idx = consensus.storage.get_commitments().await.unwrap().len();
    let cm_idx = (next_cm_idx as u32).to_le_bytes().to_vec();

    consensus
        .storage
        .store_item(KeyValueColumn::Commitment, vec![0; 32], cm_idx.clone())
        .await
        .unwrap();

    consensus
        .storage
        .store_item(KeyValueColumn::Meta, KEY_CURR_CM_INDEX.as_bytes().to_vec(), cm_idx)
        .await
        .unwrap();

    let errors = consensus.storage.validate(None, FixMode::Nothing).await;
    assert_eq!(errors.len(), 1);
    assert!(errors.contains(&ValidatorError::SuperfluousTxComponents(KeyValueColumn::Commitment, 1)));

    assert!(
        consensus
            .storage
            .validate(None, FixMode::SuperfluousTestnet1TxComponents)
            .await
            .is_empty()
    );
}

#[tokio::test]
async fn validator_vs_a_superfluous_memorandum() {
    let consensus = create_test_consensus_memdb().await;

    let blocks = TestBlocks::load(Some(5), "test_blocks_100_1").0;
    for block in blocks {
        consensus.receive_block(block).await;
    }

    // Add an extra random memo.
    let next_memo_idx = consensus.storage.get_memos().await.unwrap().len();
    let memo_idx = (next_memo_idx as u32).to_le_bytes().to_vec();

    consensus
        .storage
        .store_item(KeyValueColumn::Memo, vec![9; 32], memo_idx.clone())
        .await
        .unwrap();

    consensus
        .storage
        .store_item(KeyValueColumn::Meta, KEY_CURR_MEMO_INDEX.as_bytes().to_vec(), memo_idx)
        .await
        .unwrap();

    let errors = consensus.storage.validate(None, FixMode::Nothing).await;
    assert_eq!(errors.len(), 1);
    assert!(errors.contains(&ValidatorError::SuperfluousTxComponents(KeyValueColumn::Memo, 1)));

    assert!(
        consensus
            .storage
            .validate(None, FixMode::SuperfluousTestnet1TxComponents)
            .await
            .is_empty()
    );
}

#[tokio::test]
async fn validator_vs_a_superfluous_digest() {
    let consensus = create_test_consensus_memdb().await;

    let blocks = TestBlocks::load(Some(5), "test_blocks_100_1").0;
    for block in blocks {
        consensus.receive_block(block).await;
    }

    // Add an extra random digest.
    let digest_height = 6u32.to_le_bytes().to_vec();

    consensus
        .storage
        .store_item(KeyValueColumn::DigestIndex, vec![0; 32], digest_height.clone())
        .await
        .unwrap();

    consensus
        .storage
        .store_item(KeyValueColumn::DigestIndex, digest_height.clone(), vec![0; 32])
        .await
        .unwrap();

    let errors = consensus.storage.validate(None, FixMode::Nothing).await;
    assert_eq!(errors.len(), 1);
    assert!(errors.contains(&ValidatorError::SuperfluousTxComponents(KeyValueColumn::DigestIndex, 1)));

    assert!(
        consensus
            .storage
            .validate(None, FixMode::SuperfluousTestnet1TxComponents)
            .await
            .is_empty()
    );
}
