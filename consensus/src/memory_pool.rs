//! Transactions memory pool
//!
//! `MemoryPool` keeps a vector of transactions seen by the miner.

use snarkos_errors::consensus::ConsensusError;
use snarkos_models::{
    algorithms::MerkleParameters,
    objects::{LedgerScheme, Transaction},
};
use snarkos_objects::dpc::DPCTransactions;
use snarkos_storage::{has_duplicates, Ledger};
use snarkos_utilities::{
    bytes::{FromBytes, ToBytes},
    to_bytes,
};

use std::collections::HashMap;

/// Stores a transaction and it's size in the memory pool.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Entry<T: Transaction> {
    pub size: usize,
    pub transaction: T,
}

/// Stores transactions received by the server.
/// Transaction entries will eventually be fetched by the miner and assembled into blocks.
#[derive(Debug, Clone)]
pub struct MemoryPool<T: Transaction> {
    pub total_size: usize,

    // Hashmap transaction_id -> Entry
    pub transactions: HashMap<Vec<u8>, Entry<T>>,
}

const BLOCK_HEADER_SIZE: usize = 84;
const COINBASE_TRANSACTION_SIZE: usize = 1889; // TODO Find the value for actual coinbase transaction size

impl<T: Transaction> MemoryPool<T> {
    /// Initialize a new memory pool with no transactions
    #[inline]
    pub fn new() -> Self {
        Self {
            total_size: 0,
            transactions: HashMap::<Vec<u8>, Entry<T>>::new(),
        }
    }

    /// Load the memory pool from previously stored state in storage
    #[inline]
    pub fn from_storage<P: MerkleParameters>(storage: &Ledger<T, P>) -> Result<Self, ConsensusError> {
        let mut memory_pool = Self::new();

        if let Ok(serialized_transactions) = storage.get_memory_pool() {
            if let Ok(transaction_bytes) = DPCTransactions::<T>::read(&serialized_transactions[..]) {
                for transaction in transaction_bytes.0 {
                    let size = transaction.size();
                    let entry = Entry { transaction, size };
                    memory_pool.insert(storage, entry)?;
                }
            }
        }

        Ok(memory_pool)
    }

    /// Store the memory pool state to the database
    #[inline]
    pub fn store<P: MerkleParameters>(&self, storage: &Ledger<T, P>) -> Result<(), ConsensusError> {
        let mut transactions = DPCTransactions::<T>::new();

        for (_transaction_id, entry) in self.transactions.iter() {
            transactions.push(entry.transaction.clone())
        }

        let serialized_transactions = to_bytes![transactions]?.to_vec();

        storage.store_to_memory_pool(serialized_transactions)?;

        Ok(())
    }

    /// Adds entry to memory pool if valid in the current blockchain.
    #[inline]
    pub fn insert<P: MerkleParameters>(
        &mut self,
        storage: &Ledger<T, P>,
        entry: Entry<T>,
    ) -> Result<Option<Vec<u8>>, ConsensusError> {
        let transaction_serial_numbers = entry.transaction.old_serial_numbers();
        let transaction_commitments = entry.transaction.new_commitments();
        let transaction_memo = entry.transaction.memorandum();

        if has_duplicates(transaction_serial_numbers)
            || has_duplicates(transaction_commitments)
            || self.contains(&entry)
        {
            return Ok(None);
        }

        let mut holding_serial_numbers = vec![];
        let mut holding_commitments = vec![];
        let mut holding_memos = vec![];

        for (_, tx) in self.transactions.iter() {
            holding_serial_numbers.extend(tx.transaction.old_serial_numbers());
            holding_commitments.extend(tx.transaction.new_commitments());
            holding_memos.push(tx.transaction.memorandum());
        }

        for sn in transaction_serial_numbers {
            if storage.contains_sn(sn) || holding_serial_numbers.contains(&sn) {
                return Ok(None);
            }
        }

        for cm in transaction_commitments {
            if storage.contains_cm(cm) || holding_commitments.contains(&cm) {
                return Ok(None);
            }
        }

        if storage.contains_memo(transaction_memo) || holding_memos.contains(&transaction_memo) {
            return Ok(None);
        }

        let transaction_id = entry.transaction.transaction_id()?.to_vec();

        self.total_size += entry.size;
        self.transactions.insert(transaction_id.clone(), entry);

        Ok(Some(transaction_id))
    }

    /// Cleanse the memory pool of outdated transactions.
    #[inline]
    pub fn cleanse<P: MerkleParameters>(&mut self, storage: &Ledger<T, P>) -> Result<(), ConsensusError> {
        let mut new_memory_pool = Self::new();

        for (_, entry) in self.clone().transactions.iter() {
            new_memory_pool.insert(&storage, entry.clone())?;
        }

        self.total_size = new_memory_pool.total_size;
        self.transactions = new_memory_pool.transactions;

        Ok(())
    }

    /// Removes transaction from memory pool or error.
    #[inline]
    pub fn remove(&mut self, entry: &Entry<T>) -> Result<Option<Vec<u8>>, ConsensusError> {
        if self.contains(entry) {
            self.total_size -= entry.size;

            let transaction_id = entry.transaction.transaction_id()?.to_vec();

            self.transactions.remove(&transaction_id);

            return Ok(Some(transaction_id));
        }

        Ok(None)
    }

    /// Removes transaction from memory pool based on the transaction id.
    #[inline]
    pub fn remove_by_hash(&mut self, transaction_id: &Vec<u8>) -> Result<Option<Entry<T>>, ConsensusError> {
        match self.transactions.clone().get(transaction_id) {
            Some(entry) => {
                self.total_size -= entry.size;
                self.transactions.remove(transaction_id);

                Ok(Some(entry.clone()))
            }
            None => Ok(None),
        }
    }

    /// Returns whether or not the memory pool contains the entry.
    #[inline]
    pub fn contains(&self, entry: &Entry<T>) -> bool {
        match &entry.transaction.transaction_id() {
            Ok(transaction_id) => self.transactions.contains_key(&transaction_id.to_vec()),
            Err(_) => false,
        }
    }

    /// Get candidate transactions for a new block.
    #[inline]
    pub fn get_candidates<P: MerkleParameters>(
        &self,
        storage: &Ledger<T, P>,
        max_size: usize,
    ) -> Result<DPCTransactions<T>, ConsensusError> {
        let max_size = max_size - (BLOCK_HEADER_SIZE + COINBASE_TRANSACTION_SIZE);

        let mut block_size = 0;
        let mut transactions = DPCTransactions::new();

        // TODO Change naive transaction selection
        for (_transaction_id, entry) in self.transactions.clone() {
            if block_size + entry.size <= max_size {
                if storage.transcation_conflicts(&entry.transaction)? || transactions.conflicts(&entry.transaction) {
                    continue;
                }

                block_size += entry.size;
                transactions.push(entry.transaction.clone());
            }
        }

        Ok(transactions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use snarkos_dpc::base_dpc::instantiated::Tx;
    use snarkos_objects::Block;
    use snarkos_testing::{consensus::*, storage::*};

    use std::sync::Arc;

    #[test]
    fn push() {
        let blockchain = Arc::new(FIXTURE_VK.ledger());

        let mut mem_pool = MemoryPool::new();
        let transaction = Tx::read(&TRANSACTION_1[..]).unwrap();
        let size = TRANSACTION_1.len();

        mem_pool
            .insert(&blockchain, Entry {
                size,
                transaction: transaction.clone(),
            })
            .unwrap();

        assert_eq!(1889, mem_pool.total_size);
        assert_eq!(1, mem_pool.transactions.len());

        // Duplicate pushes don't do anything

        mem_pool.insert(&blockchain, Entry { size, transaction }).unwrap();

        assert_eq!(1889, mem_pool.total_size);
        assert_eq!(1, mem_pool.transactions.len());

        kill_storage_sync(blockchain);
    }

    #[test]
    fn remove_entry() {
        let blockchain = Arc::new(FIXTURE_VK.ledger());

        let mut mem_pool = MemoryPool::new();
        let transaction = Tx::read(&TRANSACTION_1[..]).unwrap();
        let size = TRANSACTION_1.len();

        let entry = Entry::<Tx> {
            size,
            transaction: transaction.clone(),
        };

        mem_pool.insert(&blockchain, entry.clone()).unwrap();

        assert_eq!(1, mem_pool.transactions.len());
        assert_eq!(size, mem_pool.total_size);

        mem_pool.remove(&entry).unwrap();

        assert_eq!(0, mem_pool.transactions.len());
        assert_eq!(0, mem_pool.total_size);

        kill_storage_sync(blockchain);
    }

    #[test]
    fn remove_transaction_by_hash() {
        let blockchain = Arc::new(FIXTURE_VK.ledger());

        let mut mem_pool = MemoryPool::new();
        let transaction = Tx::read(&TRANSACTION_1[..]).unwrap();
        let size = TRANSACTION_1.len();

        mem_pool
            .insert(&blockchain, Entry {
                size,
                transaction: transaction.clone(),
            })
            .unwrap();

        assert_eq!(1, mem_pool.transactions.len());
        assert_eq!(size, mem_pool.total_size);

        mem_pool
            .remove_by_hash(&transaction.transaction_id().unwrap().to_vec())
            .unwrap();

        assert_eq!(0, mem_pool.transactions.len());
        assert_eq!(0, mem_pool.total_size);

        kill_storage_sync(blockchain);
    }

    #[test]
    fn get_candidates() {
        let blockchain = Arc::new(FIXTURE_VK.ledger());

        let mut mem_pool = MemoryPool::new();
        let mut transaction = Tx::read(&TRANSACTION_1[..]).unwrap();
        // TODO (howardwu): This is not correct usage of transaction, fix me.
        // modify the tx a bit so that it does not conflict with the one already inserted
        // in the chain
        transaction.old_serial_numbers.clear();
        transaction.new_commitments.clear();
        transaction.memorandum = [99; 32];
        let size = to_bytes![transaction].unwrap().len();

        let expected_transaction = transaction.clone();
        mem_pool.insert(&blockchain, Entry { size, transaction }).unwrap();

        let max_block_size = size + BLOCK_HEADER_SIZE + COINBASE_TRANSACTION_SIZE;

        let candidates = mem_pool.get_candidates(&blockchain, max_block_size).unwrap();

        assert!(candidates.contains(&expected_transaction));

        kill_storage_sync(blockchain);
    }

    #[test]
    fn store_memory_pool() {
        let blockchain = Arc::new(FIXTURE_VK.ledger());

        let mut mem_pool = MemoryPool::new();
        let transaction = Tx::read(&TRANSACTION_1[..]).unwrap();
        mem_pool
            .insert(&blockchain, Entry {
                size: TRANSACTION_1.len(),
                transaction: transaction.clone(),
            })
            .unwrap();

        assert_eq!(1, mem_pool.transactions.len());

        mem_pool.store(&blockchain).unwrap();

        let new_mem_pool = MemoryPool::from_storage(&blockchain).unwrap();

        assert_eq!(mem_pool.total_size, new_mem_pool.total_size);

        kill_storage_sync(blockchain);
    }

    #[test]
    fn cleanse_memory_pool() {
        let blockchain = Arc::new(FIXTURE_VK.ledger());

        let mut mem_pool = MemoryPool::new();
        let transaction = Tx::read(&TRANSACTION_1[..]).unwrap();
        mem_pool
            .insert(&blockchain, Entry {
                size: TRANSACTION_1.len(),
                transaction: transaction.clone(),
            })
            .unwrap();

        assert_eq!(1, mem_pool.transactions.len());

        mem_pool.store(&blockchain).unwrap();

        let block = Block::<Tx>::read(&BLOCK_1[..]).unwrap();

        blockchain.insert_and_commit(&block).unwrap();

        mem_pool.cleanse(&blockchain).unwrap();

        assert_eq!(0, mem_pool.transactions.len());
        assert_eq!(0, mem_pool.total_size);

        kill_storage_sync(blockchain);
    }
}
