use crate::dpc::Transaction;

use snarkos_errors::objects::TransactionError;
use snarkos_utilities::{
    bytes::{FromBytes, ToBytes},
    to_bytes,
    variable_length_integer::{read_variable_length_integer, variable_length_integer},
};

use std::{
    io::{Read, Result as IoResult, Write},
    ops::{Deref, DerefMut},
};

#[derive(Clone, Eq, PartialEq)]
pub struct DPCTransactions<T: Transaction>(pub Vec<T>);

impl<T: Transaction> DPCTransactions<T> {
    /// Initializes an empty list of transactions.
    pub fn new() -> Self {
        Self(vec![])
    }

    /// Initializes from a given list of transactions.
    pub fn from(transactions: &[T]) -> Self {
        Self(transactions.to_vec())
    }

    /// Initializes an empty list of transactions.
    pub fn push(&mut self, transaction: T) {
        self.0.push(transaction);
    }

    /// Returns the transaction ids.
    pub fn to_transaction_ids(&self) -> Result<Vec<Vec<u8>>, TransactionError> {
        self.0
            .iter()
            .map(|transaction| -> Result<Vec<u8>, TransactionError> {
                transaction.transaction_id().map(|tx_id| tx_id.to_vec())
            })
            .collect::<Result<Vec<Vec<u8>>, TransactionError>>()
    }

    /// Serializes the transactions into byte vectors.
    pub fn serialize(&self) -> Result<Vec<Vec<u8>>, TransactionError> {
        self.0
            .iter()
            .map(|transaction| -> Result<Vec<u8>, TransactionError> { Ok(to_bytes![transaction]?) })
            .collect::<Result<Vec<Vec<u8>>, TransactionError>>()
    }
}

impl<T: Transaction> ToBytes for DPCTransactions<T> {
    #[inline]
    fn write<W: Write>(&self, mut writer: W) -> IoResult<()> {
        variable_length_integer(self.0.len() as u64).write(&mut writer)?;

        for transaction in &self.0 {
            transaction.write(&mut writer)?;
        }

        Ok(())
    }
}

impl<T: Transaction> FromBytes for DPCTransactions<T> {
    #[inline]
    fn read<R: Read>(mut reader: R) -> IoResult<Self> {
        let num_transactions = read_variable_length_integer(&mut reader)?;
        let mut transactions = vec![];
        for _ in 0..num_transactions {
            let transaction: T = FromBytes::read(&mut reader)?;
            transactions.push(transaction);
        }

        Ok(Self(transactions))
    }
}

impl<T: Transaction> Default for DPCTransactions<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Transaction> Deref for DPCTransactions<T> {
    type Target = Vec<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: Transaction> DerefMut for DPCTransactions<T> {
    fn deref_mut(&mut self) -> &mut Vec<T> {
        &mut self.0
    }
}
