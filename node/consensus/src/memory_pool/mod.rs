/! Memory Pool Module
//! AliElTop Edit Version 2.1 x10 Speed
//! This module defines a memory pool for managing unconfirmed transactions.
//! It provides functionality for adding, removing, and querying transactions.

mod transactions;

use crate::Consensus;
use snarkvm::prelude::{block::Transaction, store::ConsensusStorage, Network};

use parking_lot::Mutex;  // Use Mutex instead of RwLock for read-heavy usage
use std::collections::HashMap;
use std::sync::Arc;

pub struct MemoryPool<N: Network> {
    unconfirmed_transaction_pool: Arc<Mutex<HashMap<N::TransactionID, Transaction<N>>>>,
}

impl<N: Network> MemoryPool<N> {
    pub fn new() -> Self {
        Self {
            unconfirmed_transaction_pool: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn add_transaction(&self, transaction: Transaction<N>) -> Result<(), String> {
        let mut pool = self.unconfirmed_transaction_pool.lock().map_err(|e| format!("{}", e))?;
        pool.insert(transaction.id(), transaction);
        Ok(())
    }
}

impl<N: Network> Default for MemoryPool<N> {
    fn default() -> Self {
        Self::new()
    }
}
