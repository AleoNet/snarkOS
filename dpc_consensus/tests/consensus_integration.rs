//mod consensus_integration {
//    use snarkos_dpc_consensus::{
//        miner::{MemoryPool, Miner},
//        test_data::*,
//    };
//    use snarkos_objects::{
//        block::Block,
//        transaction::Transaction,
//        BlockHeader,
//        BlockHeaderHash,
//        MerkleRootHash,
//        Transactions,
//    };
//    use snarkos_dpc_storage::test_data::*;
//
//    use std::str::FromStr;
//    use wagyu_bitcoin::{BitcoinAddress, Mainnet};
//
//    fn test_find_block(
//        transactions: &Transactions,
//        parent_header: &BlockHeader,
//        expected_previous_block_hash: &BlockHeaderHash,
//        expected_merkle_root_hash: &MerkleRootHash,
//    ) {
//        let consensus = TEST_CONSENSUS;
//        let miner_address = BitcoinAddress::<Mainnet>::from_str(TEST_WALLETS[4].address).unwrap();
////        let miner = Miner::new(miner_address, consensus);
//
//        let header = miner.find_block(transactions, parent_header).unwrap();
//        assert_eq!(header.previous_block_hash, *expected_previous_block_hash);
//        assert_eq!(header.merkle_root_hash, *expected_merkle_root_hash);
//    }
//
//    fn test_verify_header(parent_header: &BlockHeader, child_header: &BlockHeader, merkle_root_hash: &MerkleRootHash) {
//        let consensus = TEST_CONSENSUS;
//        consensus
//            .verify_header(child_header, parent_header, merkle_root_hash)
//            .unwrap();
//    }
//
//    // CONSENSUS_PARAMS format: [ transaction_bytes, parent_header, child_header, parent_header_hash, merkle_root_hash ]
//    const CONSENSUS_PARAMS: [([u8; 183], BlockHeader, BlockHeader, BlockHeaderHash, MerkleRootHash); 2] = [
//        (
//            [
//                1, 0, 0, 0, 1, 97, 213, 32, 204, 183, 66, 136, 201, 107, 193, 162, 178, 14, 161, 192, 213, 167, 4, 119,
//                109, 208, 22, 74, 57, 110, 254, 195, 234, 112, 64, 52, 157, 0, 0, 0, 0, 106, 71, 48, 69, 2, 33, 0, 229,
//                3, 151, 79, 16, 139, 3, 218, 116, 76, 140, 175, 184, 109, 130, 14, 125, 243, 53, 154, 98, 38, 99, 10,
//                31, 62, 193, 86, 234, 195, 117, 244, 2, 32, 65, 28, 208, 127, 70, 251, 168, 166, 233, 253, 33, 74, 179,
//                138, 84, 216, 133, 13, 124, 52, 230, 132, 27, 88, 179, 70, 2, 87, 201, 42, 18, 52, 33, 2, 159, 80, 245,
//                29, 99, 179, 69, 3, 154, 41, 12, 148, 191, 253, 49, 128, 201, 158, 214, 89, 255, 110, 166, 177, 36, 43,
//                202, 71, 235, 147, 181, 159, 1, 224, 46, 0, 0, 0, 0, 0, 0, 25, 118, 169, 20, 6, 175, 212, 107, 205,
//                253, 34, 239, 148, 172, 18, 42, 161, 31, 36, 18, 68, 163, 126, 204, 136, 172,
//            ],
//            BlockHeader {
//                previous_block_hash: BlockHeaderHash([0u8; 32]),
//                merkle_root_hash: MerkleRootHash([0u8; 32]),
//                time: 0i64,
//                difficulty_target: 0x07FF_FFFF_FFFF_FFFF_u64,
//                nonce: 0u32,
//            },
//            BlockHeader {
//                previous_block_hash: BlockHeaderHash([
//                    34, 41, 138, 133, 212, 97, 218, 105, 103, 149, 244, 65, 29, 63, 202, 157, 79, 184, 117, 83, 54,
//                    165, 78, 178, 91, 245, 248, 4, 235, 112, 78, 121,
//                ]),
//                merkle_root_hash: MerkleRootHash([
//                    234, 31, 222, 25, 47, 22, 247, 145, 153, 112, 125, 137, 120, 183, 226, 74, 140, 75, 78, 30, 12, 80,
//                    235, 218, 183, 219, 17, 233, 215, 107, 188, 149,
//                ]),
//                time: 0i64,
//                difficulty_target: 11_765_080_809_439_605_706_u64,
//                nonce: 0u32,
//            },
//            BlockHeaderHash([
//                34, 41, 138, 133, 212, 97, 218, 105, 103, 149, 244, 65, 29, 63, 202, 157, 79, 184, 117, 83, 54, 165,
//                78, 178, 91, 245, 248, 4, 235, 112, 78, 121,
//            ]),
//            MerkleRootHash([
//                234, 31, 222, 25, 47, 22, 247, 145, 153, 112, 125, 137, 120, 183, 226, 74, 140, 75, 78, 30, 12, 80,
//                235, 218, 183, 219, 17, 233, 215, 107, 188, 149,
//            ]),
//        ),
//        (
//            [
//                1, 0, 0, 0, 1, 97, 213, 32, 204, 183, 66, 136, 201, 107, 193, 162, 178, 14, 161, 192, 213, 167, 4, 119,
//                109, 208, 22, 74, 57, 110, 254, 195, 234, 112, 64, 52, 157, 0, 0, 0, 0, 106, 71, 48, 69, 2, 33, 0, 229,
//                3, 151, 79, 16, 139, 3, 218, 116, 76, 140, 175, 184, 109, 130, 14, 125, 243, 53, 154, 98, 38, 99, 10,
//                31, 62, 193, 86, 234, 195, 117, 244, 2, 32, 65, 28, 208, 127, 70, 251, 168, 166, 233, 253, 33, 74, 179,
//                138, 84, 216, 133, 13, 124, 52, 230, 132, 27, 88, 179, 70, 2, 87, 201, 42, 18, 52, 33, 2, 159, 80, 245,
//                29, 99, 179, 69, 3, 154, 41, 12, 148, 191, 253, 49, 128, 201, 158, 214, 89, 255, 110, 166, 177, 36, 43,
//                202, 71, 235, 147, 181, 159, 1, 224, 46, 0, 0, 0, 0, 0, 0, 25, 118, 169, 20, 6, 175, 212, 107, 205,
//                253, 34, 239, 148, 172, 18, 42, 161, 31, 36, 18, 68, 163, 126, 204, 136, 172,
//            ],
//            BlockHeader {
//                previous_block_hash: BlockHeaderHash([0u8; 32]),
//                merkle_root_hash: MerkleRootHash([0u8; 32]),
//                time: 0i64,
//                difficulty_target: 0x0000_7FFF_FFFF_FFFF_u64,
//                nonce: 69950u32,
//            },
//            BlockHeader {
//                previous_block_hash: BlockHeaderHash([
//                    71, 96, 136, 138, 156, 60, 0, 0, 172, 219, 151, 28, 109, 226, 132, 171, 235, 109, 113, 92, 207, 54,
//                    69, 213, 19, 158, 217, 13, 154, 191, 146, 241,
//                ]),
//                merkle_root_hash: MerkleRootHash([
//                    234, 31, 222, 25, 47, 22, 247, 145, 153, 112, 125, 137, 120, 183, 226, 74, 140, 75, 78, 30, 12, 80,
//                    235, 218, 183, 219, 17, 233, 215, 107, 188, 149,
//                ]),
//                time: 0i64,
//                difficulty_target: 140_737_488_355_327_u64,
//                nonce: 55793u32,
//            },
//            BlockHeaderHash([
//                71, 96, 136, 138, 156, 60, 0, 0, 172, 219, 151, 28, 109, 226, 132, 171, 235, 109, 113, 92, 207, 54, 69,
//                213, 19, 158, 217, 13, 154, 191, 146, 241,
//            ]),
//            MerkleRootHash([
//                234, 31, 222, 25, 47, 22, 247, 145, 153, 112, 125, 137, 120, 183, 226, 74, 140, 75, 78, 30, 12, 80,
//                235, 218, 183, 219, 17, 233, 215, 107, 188, 149,
//            ]),
//        ),
//    ];
//
//    #[test]
//    fn find_valid_block() {
//        CONSENSUS_PARAMS.iter().for_each(
//            |(transaction_bytes, parent_header, _, expected_previous_block_hash, expected_merkle_root_hash)| {
//                let transactions =
//                    Transactions::from(&[Transaction::deserialize(&transaction_bytes.to_vec()).unwrap()]);
//                test_find_block(
//                    &transactions,
//                    parent_header,
//                    expected_previous_block_hash,
//                    expected_merkle_root_hash,
//                );
//            },
//        );
//    }
//
//    #[test]
//    fn verify_header() {
//        CONSENSUS_PARAMS
//            .iter()
//            .for_each(|(_, parent_header, child_header, _, expected_merkle_root_hash)| {
//                test_verify_header(parent_header, child_header, expected_merkle_root_hash);
//            });
//    }
//
//    #[test]
//    fn transaction_double_spend() {
//        let (mut blockchain, path) = initialize_test_blockchain();
//        let mut memory_pool = MemoryPool::new();
//
//        let consensus = TEST_CONSENSUS;
//        let miner_address = BitcoinAddress::<Mainnet>::from_str(TEST_WALLETS[4].address).unwrap();
//        let miner = Miner::new(miner_address, consensus.clone());
//
//        let previous_block = &blockchain.get_latest_block().unwrap();
//
//        let transactions = Transactions::from(&[
//            previous_block.transactions[0].clone(),
//            previous_block.transactions[0].clone(),
//        ]);
//
//        assert!(blockchain.check_block_transactions(&transactions).is_err());
//
//        let header = miner.find_block(&transactions, &previous_block.header).unwrap();
//
//        let test_block = Block { header, transactions };
//
//        assert!(
//            consensus
//                .process_block(&mut blockchain, &mut memory_pool, &test_block)
//                .is_err()
//        );
//
//        kill_storage_sync(blockchain, path);
//    }
//}
