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

mod consensus_integration {
    use std::sync::atomic::AtomicBool;

    use snarkos_consensus::miner::MineContext;
    use snarkos_storage::{SerialBlockHeader, SerialTransaction};
    use snarkos_testing::sync::*;
    use snarkvm_posw::txids_to_roots;

    // this test ensures that a block is found by running the proof of work
    // and that it doesnt loop forever
    async fn test_find_block(transactions: &[SerialTransaction], parent_header: &SerialBlockHeader) {
        let consensus = snarkos_testing::sync::create_test_consensus().await;
        let miner_address = FIXTURE_VK.test_accounts[0].address.clone();
        let miner = MineContext::prepare(miner_address, consensus.clone()).await.unwrap();

        let header = miner.find_block(transactions, parent_header, &AtomicBool::new(false)).unwrap();

        let expected_prev_block_hash = parent_header.hash();
        assert_eq!(header.previous_block_hash, expected_prev_block_hash);

        let transaction_ids = transactions.iter().map(|x| x.id).collect::<Vec<_>>();
        let expected_merkle_root_hash = snarkvm_dpc::merkle_root(&transaction_ids[..]);
        assert_eq!(&header.merkle_root_hash.0[..], &expected_merkle_root_hash[..]);

        // generate the verifier args
        let (merkle_root, pedersen_merkle_root, _) = txids_to_roots(&transaction_ids[..]);

        // ensure that our POSW proof passes
        consensus
            .parameters
            .verify_header(&header, parent_header, &merkle_root, &pedersen_merkle_root)
            .unwrap();
    }

    #[tokio::test]
    async fn find_valid_block() {
        let transactions = vec![
            DATA.block_1.transactions[0].clone(),
            DATA.block_2.transactions[0].clone(),
        ];
        let parent_header = genesis().header.into();
        test_find_block(&transactions, &parent_header).await;
    }
}
