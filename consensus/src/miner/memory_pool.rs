//! Transactions memory pool
//!
//! `MemoryPool` keeps a vector of transactions seen by the miner.

use snarkos_errors::consensus::ConsensusError;
use snarkos_objects::{transaction::Transaction, Outpoint, Transactions};
use snarkos_storage::BlockStorage;

use std::collections::HashMap;

/// Stores a transaction and it's size in the memory pool.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Entry {
    pub size: usize,
    pub transaction: Transaction,
}

/// Stores transactions received by the server.
/// Transaction entries will eventually be fetched by the miner and assembled into blocks.
#[derive(Debug, Clone)]
pub struct MemoryPool {
    pub total_size: usize,

    // Hashmap transaction_id -> Entry
    pub transactions: HashMap<Vec<u8>, Entry>,

    // Hashmap outpoint -> transaction_id of the transaction spending the outpoint
    pub transaction_from_outpoint: HashMap<Outpoint, Vec<u8>>,
}

const BLOCK_HEADER_SIZE: usize = 84;
const COINBASE_TRANSACTION_SIZE: usize = 81;

impl MemoryPool {
    #[inline]
    pub fn new() -> Self {
        Self {
            total_size: 0,
            transactions: HashMap::<Vec<u8>, Entry>::new(),
            transaction_from_outpoint: HashMap::<Outpoint, Vec<u8>>::new(),
        }
    }

    #[inline]
    pub fn from_storage(storage: &BlockStorage) -> Result<Self, ConsensusError> {
        let mut memory_pool = Self::new();

        match storage.get_memory_pool_transactions() {
            Ok(serialized_transactions_option) => {
                if let Some(serialized_transactions) = serialized_transactions_option {
                    let transaction_bytes: Vec<Vec<u8>> = bincode::deserialize(&serialized_transactions)?;

                    for tx_bytes in transaction_bytes {
                        let transaction = Transaction::deserialize(&tx_bytes)?;
                        let size = tx_bytes.len();
                        let entry = Entry { transaction, size };
                        memory_pool.insert(storage, entry)?;
                    }
                }
            }
            Err(_) => {}
        };

        Ok(memory_pool)
    }

    #[inline]
    pub fn store(&self, storage: &BlockStorage) -> Result<(), ConsensusError> {
        // TODO (howardwu): Convert this to imperative logic.
        let mut transactions = vec![];

        for (_transaction_id, entry) in self.transactions.iter() {
            transactions.push(entry.transaction.serialize()?)
        }

        let serialized_transactions = bincode::serialize(&transactions)?;

        storage.store_to_memory_pool(serialized_transactions)?;

        Ok(())
    }

    /// Adds entry to memory pool if valid in the current blockchain.
    #[inline]
    pub fn insert(&mut self, storage: &BlockStorage, entry: Entry) -> Result<Option<Vec<u8>>, ConsensusError> {
        match storage.check_for_double_spend(&entry.transaction.clone()) {
            Ok(_) => {
                let transaction_id = entry.transaction.to_transaction_id()?;

                let mut outpoints_to_add: Vec<(Outpoint, Vec<u8>)> = vec![];
                // Make sure an outpoint can only have 1 corresponding tx in the memory pool
                for input in entry.transaction.parameters.inputs.clone() {
                    let store_outpoint =
                        Outpoint::new(input.outpoint.transaction_id, input.outpoint.index, None, None)?;

                    if self.transaction_from_outpoint.get(&store_outpoint).is_some() {
                        // There already exists a transaction spending this outpoint in the memory pool
                        return Ok(None);
                    } else {
                        outpoints_to_add.push((store_outpoint, transaction_id.clone()));
                    }
                }

                self.total_size += entry.size;
                self.transactions.insert(transaction_id.clone(), entry);

                for (outpoint, entry_hash) in outpoints_to_add {
                    let input_check = self.transaction_from_outpoint.insert(outpoint, entry_hash);
                    assert_eq!(input_check, None);
                }

                Ok(Some(transaction_id))
            }
            Err(_) => Ok(None),
        }
    }

    /// Cleanse the memory pool of outdated transactions.
    #[inline]
    pub fn cleanse(&mut self, storage: &BlockStorage) -> Result<(), ConsensusError> {
        let mut new_memory_pool = Self::new();

        for (_, entry) in self.clone().transactions.iter() {
            new_memory_pool.insert(&storage, entry.clone())?;
        }

        self.total_size = new_memory_pool.total_size;
        self.transactions = new_memory_pool.transactions;
        self.transaction_from_outpoint = new_memory_pool.transaction_from_outpoint;

        Ok(())
    }

    /// Removes transaction from memory pool or error.
    #[inline]
    pub fn remove(&mut self, entry: &Entry) -> Result<Option<Vec<u8>>, ConsensusError> {
        if self.contains(entry) {
            self.total_size -= entry.size;

            let transaction_id = entry.transaction.to_transaction_id()?;

            self.transactions.remove(&transaction_id);

            return Ok(Some(transaction_id));
        }

        Ok(None)
    }

    /// Removes transaction from memory pool based on the transaction id.
    #[inline]
    pub fn remove_by_hash(&mut self, transaction_id: &Vec<u8>) -> Result<Option<Entry>, ConsensusError> {
        match self.transactions.clone().get(transaction_id) {
            Some(entry) => {
                self.total_size -= entry.size;
                self.remove_outpoint_references(&entry.clone().transaction)?;
                self.transactions.remove(transaction_id);

                Ok(Some(entry.clone()))
            }
            None => Ok(None),
        }
    }

    #[inline]
    fn remove_outpoint_references(
        &mut self,
        transaction: &Transaction,
    ) -> Result<Option<Vec<Outpoint>>, ConsensusError> {
        let mut removed_outpoints = vec![];
        for input in transaction.parameters.inputs.clone() {
            let input_outpoint = Outpoint::new(input.outpoint.transaction_id, input.outpoint.index, None, None)?;
            let removed_entry_hash = self.transaction_from_outpoint.remove(&input_outpoint);
            assert_eq!(removed_entry_hash, Some(transaction.to_transaction_id()?));
            removed_outpoints.push(input_outpoint);
        }

        match removed_outpoints.len() {
            0 => Ok(None),
            _ => Ok(Some(removed_outpoints)),
        }
    }

    /// Removes transaction from memory pool based on outpoints (when transactions are spent).
    #[inline]
    pub fn remove_by_outpoint(&mut self, outpoint: &Outpoint) -> Result<Vec<Entry>, ConsensusError> {
        let outpoint = Outpoint::new(outpoint.transaction_id.clone(), outpoint.index, None, None)?;
        let mut removed: Vec<Entry> = vec![];

        if let Some(entry_hash) = self.transaction_from_outpoint.clone().get(&outpoint) {
            if let Some(removed_entry) = self.transactions.remove(&entry_hash.clone()) {
                self.remove_outpoint_references(&removed_entry.transaction)?;

                for index in 0..removed_entry.transaction.parameters.outputs.len() {
                    let output_outpoint = Outpoint::new(entry_hash.clone(), index as u32, None, None)?;
                    removed.extend(self.remove_by_outpoint(&output_outpoint)?);
                }
            }
        }

        Ok(removed)
    }

    /// Returns whether or not the memory pool contains the entry.
    #[inline]
    pub fn contains(&self, entry: &Entry) -> bool {
        match &entry.transaction.to_transaction_id() {
            Ok(transaction_id) => self.transactions.contains_key(transaction_id),
            Err(_) => false,
        }
    }

    /// Get candidate transactions for a new block.
    #[inline]
    pub fn get_candidates(&self, storage: &BlockStorage, max_size: usize) -> Result<Transactions, ConsensusError> {
        let max_size = max_size - (BLOCK_HEADER_SIZE + COINBASE_TRANSACTION_SIZE);

        let mut block_size = 0;
        let mut transactions = Transactions::new();

        let mut spent_outpoints: Vec<Outpoint> = vec![];

        // TODO Change naive transaction selection
        'outer: for (_transaction_id, entry) in self.transactions.clone() {
            let mut temp_spent_outpoints: Vec<Outpoint> = spent_outpoints.clone();
            if block_size + entry.size <= max_size {
                storage.check_for_double_spend(&entry.transaction.clone())?;

                for input in entry.transaction.parameters.inputs.clone() {
                    let outpoint = Outpoint::new(input.outpoint.transaction_id, input.outpoint.index, None, None)?;
                    if spent_outpoints.contains(&outpoint) || temp_spent_outpoints.contains(&outpoint) {
                        continue 'outer;
                    }

                    temp_spent_outpoints.push(outpoint)
                }

                spent_outpoints.extend(temp_spent_outpoints);
                block_size += entry.size;
                transactions.push(entry.transaction.clone());
            }
        }

        storage.check_for_double_spends(&transactions)?;

        Ok(transactions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_data::*;

    use hex;

    const TRANSACTION_BYTES: &str = "0100000001b3d9ad9de8e21b2b3a9ffb40bae6fefa852026e7fb2e279322cd7589a20ee355000000006a473045022100a3a47eb43eed300927ac841483eaae4f7a886f9fcd7316dae83c9e73d0b4b11802206e948f63dccd11d01cd59b8bbee0251bc16122e3e7b1559f34a3f958534e45342103ca64499d857698431e999035fd22d97896b1dff672739ad9acb8643cdd2be95102f8dcfa02000000001976a9143804a328df69bc873f96c63b3e3218bc2602283088acf8dcfa02000000001976a9148e3d6baa7c1a0a927ea69108503fb5b55e9a71eb88ac";
    const DOUBLE_SPEND_TRANSACTION_BYTES: &str = "0100000001b3d9ad9de8e21b2b3a9ffb40bae6fefa852026e7fb2e279322cd7589a20ee355000000006a473045022100fcf14fbcafed480a8f6f032bec1564c20543362b7032e4c31442dcf702aa5c2b022020d778121387da0ba8f2229a6dbc22bffa6982d9561834531cc3b2b0188c4a852103ca64499d857698431e999035fd22d97896b1dff672739ad9acb8643cdd2be95102f8dcfa02000000001976a9147c421e82d4b2a9c77300f8a8c38f42fd30296f4a88acf8dcfa02000000001976a9148e3d6baa7c1a0a927ea69108503fb5b55e9a71eb88ac";

    #[test]
    fn push() {
        let (blockchain, path) = initialize_test_blockchain();

        let mut mem_pool = MemoryPool::new();
        mem_pool
            .insert(&blockchain, Entry {
                size: TRANSACTION_BYTES.len(),
                transaction: Transaction::deserialize(&hex::decode(TRANSACTION_BYTES).unwrap()).unwrap(),
            })
            .unwrap();

        assert_eq!(434, mem_pool.total_size);
        assert_eq!(1, mem_pool.transactions.len());

        kill_storage_sync(blockchain, path);
    }

    #[test]
    fn remove_transaction_by_hash() {
        let (blockchain, path) = initialize_test_blockchain();

        let mut mem_pool = MemoryPool::new();
        let transaction = Transaction::deserialize(&hex::decode(TRANSACTION_BYTES).unwrap()).unwrap();
        mem_pool
            .insert(&blockchain, Entry {
                size: TRANSACTION_BYTES.len(),
                transaction: transaction.clone(),
            })
            .unwrap();

        assert_eq!(1, mem_pool.transactions.len());
        assert_eq!(
            transaction.parameters.inputs.len(),
            mem_pool.transaction_from_outpoint.len()
        );

        mem_pool
            .remove_by_hash(&transaction.to_transaction_id().unwrap())
            .unwrap();

        assert_eq!(0, mem_pool.transactions.len());
        assert_eq!(0, mem_pool.transaction_from_outpoint.len());

        kill_storage_sync(blockchain, path);
    }

    #[test]
    fn remove_transaction_by_outpoint() {
        let (blockchain, path) = initialize_test_blockchain();

        let mut mem_pool = MemoryPool::new();
        let transaction = Transaction::deserialize(&hex::decode(TRANSACTION_BYTES).unwrap()).unwrap();
        mem_pool
            .insert(&blockchain, Entry {
                size: TRANSACTION_BYTES.len(),
                transaction: transaction.clone(),
            })
            .unwrap();

        assert_eq!(1, mem_pool.transactions.len());
        assert_eq!(
            transaction.parameters.inputs.len(),
            mem_pool.transaction_from_outpoint.len()
        );

        let outpoint = transaction.parameters.inputs[0].outpoint.clone();
        mem_pool.remove_by_outpoint(&outpoint).unwrap();

        assert_eq!(0, mem_pool.transactions.len());
        assert_eq!(0, mem_pool.transaction_from_outpoint.len());

        kill_storage_sync(blockchain, path);
    }

    #[test]
    fn get_candidates() {
        let (blockchain, path) = initialize_test_blockchain();

        let mut mem_pool = MemoryPool::new();
        let transaction = Transaction::deserialize(&hex::decode(TRANSACTION_BYTES).unwrap()).unwrap();
        let expected_transaction = transaction.clone();
        mem_pool
            .insert(&blockchain, Entry {
                size: TRANSACTION_BYTES.len(),
                transaction,
            })
            .unwrap();

        let max_block_size = TRANSACTION_BYTES.len() + BLOCK_HEADER_SIZE + COINBASE_TRANSACTION_SIZE;

        let candidates = mem_pool.get_candidates(&blockchain, max_block_size).unwrap();

        assert!(candidates.contains(&expected_transaction));

        kill_storage_sync(blockchain, path);
    }

    #[test]
    fn get_candidates_handle_double_spends() {
        let (blockchain, path) = initialize_test_blockchain();

        let mut mem_pool = MemoryPool::new();
        let transaction = Transaction::deserialize(&hex::decode(TRANSACTION_BYTES).unwrap()).unwrap();
        let double_spend_transaction =
            Transaction::deserialize(&hex::decode(DOUBLE_SPEND_TRANSACTION_BYTES).unwrap()).unwrap();

        mem_pool
            .insert(&blockchain, Entry {
                size: TRANSACTION_BYTES.len(),
                transaction: transaction.clone(),
            })
            .unwrap();

        mem_pool
            .insert(&blockchain, Entry {
                size: TRANSACTION_BYTES.len(),
                transaction: transaction.clone(),
            })
            .unwrap();

        mem_pool
            .insert(&blockchain, Entry {
                size: DOUBLE_SPEND_TRANSACTION_BYTES.len(),
                transaction: double_spend_transaction,
            })
            .unwrap();

        assert_eq!(1, mem_pool.transactions.len());

        println!("{}", TRANSACTION_BYTES.len());

        let max_block_size = TRANSACTION_BYTES.len() + BLOCK_HEADER_SIZE + COINBASE_TRANSACTION_SIZE;

        let candidates = mem_pool.get_candidates(&blockchain, max_block_size).unwrap();

        assert!(candidates.contains(&transaction));
        assert_eq!(1, candidates.len());

        kill_storage_sync(blockchain, path);
    }
}
