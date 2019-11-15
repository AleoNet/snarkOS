use crate::transaction::{Transaction, TransactionParameters, Vector};
use snarkos_errors::objects::TransactionError;

use std::ops::{Deref, DerefMut};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Transactions(Vec<Transaction>);

impl Transactions {
    /// Initializes an empty list of transactions.
    pub fn new() -> Self {
        Self(vec![])
    }

    /// Initializes from a given list of transactions.
    pub fn from(transactions: &[Transaction]) -> Self {
        Self(transactions.to_vec())
    }

    /// Returns the transaction ids.
    pub fn to_transaction_ids(&self) -> Result<Vec<Vec<u8>>, TransactionError> {
        self.0
            .iter()
            .map(|transaction| -> Result<Vec<u8>, TransactionError> { transaction.to_transaction_id() })
            .collect::<Result<Vec<Vec<u8>>, TransactionError>>()
    }

    /// Serializes the transactions into byte vectors.
    pub fn serialize(&self) -> Result<Vec<Vec<u8>>, TransactionError> {
        self.0
            .iter()
            .map(|transaction| -> Result<Vec<u8>, TransactionError> { transaction.serialize() })
            .collect::<Result<Vec<Vec<u8>>, TransactionError>>()
    }

    /// Deserializes the given byte vectors into transactions.
    pub fn deserialize(bytes: &[u8]) -> Result<Self, TransactionError> {
        Ok(Transactions(
            Vector::read(&mut bytes.clone(), TransactionParameters::read)?
                .iter()
                .map(|parameters| Transaction {
                    parameters: parameters.clone(),
                })
                .collect::<Vec<Transaction>>(),
        ))
    }
}

impl Default for Transactions {
    fn default() -> Self {
        Self::new()
    }
}

impl Deref for Transactions {
    type Target = Vec<Transaction>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Transactions {
    fn deref_mut(&mut self) -> &mut Vec<Transaction> {
        &mut self.0
    }
}
