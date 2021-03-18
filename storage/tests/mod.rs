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

mod test_storage {
    use snarkos_testing::storage::*;
    use snarkvm_objects::{
        Block,
        BlockHeader,
        BlockHeaderHash,
        DPCTransactions,
        MerkleRootHash,
        PedersenMerkleRootHash,
        ProofOfSuccinctWork,
    };

    #[test]
    pub fn test_new_blockchain() {
        let (blockchain, _): (Store, _) = open_test_blockchain();

        assert_eq!(blockchain.get_current_block_height(), 0);

        let _latest_block = blockchain.get_latest_block().unwrap();

        kill_storage_sync(blockchain);
    }

    #[test]
    pub fn remove_decrements_height() {
        let (blockchain, _): (Store, _) = open_test_blockchain();

        assert_eq!(blockchain.get_current_block_height(), 0);

        // insert a block
        let block = Block {
            header: BlockHeader {
                difficulty_target: 100,
                nonce: 99,
                merkle_root_hash: MerkleRootHash([0; 32]),
                previous_block_hash: BlockHeaderHash([0; 32]),
                time: 123,
                proof: ProofOfSuccinctWork::default(),
                pedersen_merkle_root_hash: PedersenMerkleRootHash([0; 32]),
            },
            transactions: DPCTransactions::new(),
        };

        blockchain.insert_and_commit(&block).unwrap();
        assert_eq!(blockchain.get_current_block_height(), 1);

        // removing it decrements the chain's height
        blockchain.remove_latest_block().unwrap();
        assert_eq!(blockchain.get_current_block_height(), 0);

        kill_storage_sync(blockchain);
    }

    #[test]
    pub fn test_storage() {
        let (blockchain, _): (Store, _) = open_test_blockchain();

        blockchain.storage.db.put(b"my key", b"my value").unwrap();

        match blockchain.storage.db.get(b"my key") {
            Ok(Some(value)) => println!("retrieved value {}", String::from_utf8(value).unwrap()),
            Ok(None) => println!("value not found"),
            Err(e) => println!("operational problem encountered: {}", e),
        }

        assert!(blockchain.storage.db.get(b"my key").is_ok());

        kill_storage_sync(blockchain);
    }

    #[test]
    pub fn test_storage_memory_pool() {
        let (blockchain, _): (Store, _) = open_test_blockchain();
        let transactions_serialized = vec![0u8];

        assert!(blockchain.store_to_memory_pool(transactions_serialized.clone()).is_ok());
        assert!(blockchain.get_memory_pool().is_ok());
        assert_eq!(transactions_serialized, blockchain.get_memory_pool().unwrap());

        kill_storage_sync(blockchain);
    }

    #[test]
    pub fn test_storage_peer_book() {
        let (blockchain, _): (Store, _) = open_test_blockchain();
        let peers_serialized = vec![0u8];

        assert!(blockchain.save_peer_book_to_storage(peers_serialized.clone()).is_ok());
        assert!(blockchain.get_peer_book().is_ok());
        assert_eq!(peers_serialized, blockchain.get_peer_book().unwrap());

        kill_storage_sync(blockchain);
    }

    #[test]
    pub fn test_destroy_storage() {
        let mut path = std::env::temp_dir();
        path.push(random_storage_path());

        Store::destroy_storage(path).unwrap();
    }

    mod test_invalid {
        use super::*;

        #[test]
        pub fn test_invalid_block_addition() {
            let (blockchain, _): (Store, _) = open_test_blockchain();

            let latest_block = blockchain.get_latest_block().unwrap();

            assert!(blockchain.insert_and_commit(&latest_block).is_err());

            kill_storage_sync(blockchain);
        }

        #[test]
        pub fn test_invalid_block_removal() {
            let (blockchain, _): (Store, _) = open_test_blockchain();

            assert!(blockchain.remove_latest_block().is_err());
            assert!(blockchain.remove_latest_blocks(5).is_err());

            kill_storage_sync(blockchain);
        }

        #[test]
        pub fn test_invalid_block_retrieval() {
            let (blockchain, _): (Store, _) = open_test_blockchain();

            assert!(blockchain.get_block_from_block_number(2).is_err());
            assert!(blockchain.get_block_from_block_number(10).is_err());

            kill_storage_sync(blockchain);
        }
    }
}
