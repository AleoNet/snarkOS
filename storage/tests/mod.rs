use snarkos_algorithms::{
    crh::{PedersenCompressedCRH, PedersenSize},
    define_merkle_tree_parameters,
};
use snarkos_curves::edwards_bls12::EdwardsProjective as EdwardsBls;
use snarkos_errors::objects::TransactionError;
use snarkos_models::objects::Transaction;
use snarkos_objects::{Block, BlockHeader, BlockHeaderHash, DPCTransactions, MerkleRootHash};
use snarkos_storage::{test_data::*, Ledger};
use snarkos_utilities::bytes::{FromBytes, ToBytes};

use std::{
    io::{Read, Result as IoResult, Write},
    sync::Arc,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestTx;

impl Transaction for TestTx {
    type Commitment = [u8; 32];
    type Memorandum = [u8; 32];
    type SerialNumber = [u8; 32];
    type Stuff = [u8; 32];

    fn old_serial_numbers(&self) -> &[Self::SerialNumber] {
        &[[0u8; 32]]
    }

    fn new_commitments(&self) -> &[Self::Commitment] {
        &[[0u8; 32]]
    }

    fn memorandum(&self) -> &Self::Memorandum {
        &[0u8; 32]
    }

    fn stuff(&self) -> &Self::Stuff {
        &[0u8; 32]
    }

    fn transaction_id(&self) -> Result<[u8; 32], TransactionError> {
        Ok([0u8; 32])
    }

    fn size(&self) -> usize {
        0
    }

    fn value_balance(&self) -> i64 {
        0
    }
}

impl ToBytes for TestTx {
    #[inline]
    fn write<W: Write>(&self, mut _writer: W) -> IoResult<()> {
        Ok(())
    }
}

impl FromBytes for TestTx {
    #[inline]
    fn read<R: Read>(mut _reader: R) -> IoResult<Self> {
        Ok(Self)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Size;
// `WINDOW_SIZE * NUM_WINDOWS` = 2 * 256 bits
impl PedersenSize for Size {
    const NUM_WINDOWS: usize = 4;
    const WINDOW_SIZE: usize = 128;
}

define_merkle_tree_parameters!(TestMerkleParams, PedersenCompressedCRH<EdwardsBls, Size>, 32);

type Store = Ledger<TestTx, TestMerkleParams>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn test_new_blockchain() {
        let (blockchain, _): (Arc<Store>, _) = open_test_blockchain();

        assert_eq!(blockchain.get_latest_block_height(), 0);

        let _latest_block = blockchain.get_latest_block().unwrap();

        kill_storage_sync(blockchain);
    }

    #[test]
    pub fn remove_decrements_height() {
        let (blockchain, _): (Arc<Store>, _) = open_test_blockchain();

        assert_eq!(blockchain.get_latest_block_height(), 0);

        // insert a block
        let block = Block {
            header: BlockHeader {
                difficulty_target: 100,
                nonce: 99,
                merkle_root_hash: MerkleRootHash([0; 32]),
                previous_block_hash: BlockHeaderHash([0; 32]),
                time: 123,
            },
            transactions: DPCTransactions::new(),
        };

        blockchain.insert_block(&block).unwrap();
        assert_eq!(blockchain.get_latest_block_height(), 1);

        // removing it decrements the chain's height
        blockchain.remove_latest_block().unwrap();
        assert_eq!(blockchain.get_latest_block_height(), 0);

        kill_storage_sync(blockchain);
    }

    #[test]
    pub fn test_storage() {
        let (blockchain, _): (Arc<Store>, _) = open_test_blockchain();

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
        let (blockchain, _): (Arc<Store>, _) = open_test_blockchain();
        let transactions_serialized = vec![0u8];

        assert!(blockchain.store_to_memory_pool(transactions_serialized.clone()).is_ok());
        assert!(blockchain.get_memory_pool().is_ok());
        assert_eq!(transactions_serialized, blockchain.get_memory_pool().unwrap());

        kill_storage_sync(blockchain);
    }

    #[test]
    pub fn test_storage_peer_book() {
        let (blockchain, _): (Arc<Store>, _) = open_test_blockchain();
        let peers_serialized = vec![0u8];

        assert!(blockchain.store_to_peer_book(peers_serialized.clone()).is_ok());
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
            let (blockchain, _): (Arc<Store>, _) = open_test_blockchain();

            let latest_block = blockchain.get_latest_block().unwrap();

            assert!(blockchain.insert_block(&latest_block).is_err());

            kill_storage_sync(blockchain);
        }

        #[test]
        pub fn test_invalid_block_removal() {
            let (blockchain, _): (Arc<Store>, _) = open_test_blockchain();

            assert!(blockchain.remove_latest_block().is_err());
            assert!(blockchain.remove_latest_blocks(5).is_err());

            kill_storage_sync(blockchain);
        }

        #[test]
        pub fn test_invalid_block_retrieval() {
            let (blockchain, _): (Arc<Store>, _) = open_test_blockchain();

            assert!(blockchain.get_block_from_block_num(2).is_err());
            assert!(blockchain.get_block_from_block_num(10).is_err());

            kill_storage_sync(blockchain);
        }
    }
}
