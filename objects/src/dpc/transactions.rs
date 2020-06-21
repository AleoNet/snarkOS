use snarkos_errors::objects::TransactionError;
use snarkos_models::objects::Transaction;
use snarkos_utilities::{
    bytes::{FromBytes, ToBytes},
    has_duplicates,
    to_bytes,
    variable_length_integer::{read_variable_length_integer, variable_length_integer},
};

use std::{
    io::{Read, Result as IoResult, Write},
    ops::{Deref, DerefMut},
};

#[derive(Debug, Clone, Eq, PartialEq)]
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

    /// Serializes the transactions into strings.
    pub fn serialize_as_str(&self) -> Result<Vec<String>, TransactionError> {
        self.0
            .iter()
            .map(|transaction| -> Result<String, TransactionError> { Ok(hex::encode(to_bytes![transaction]?)) })
            .collect::<Result<Vec<String>, TransactionError>>()
    }

    pub fn conflicts(&self, transaction: &T) -> bool {
        let mut holding_serial_numbers = vec![];
        let mut holding_commitments = vec![];
        let mut holding_memos = vec![];

        for tx in &self.0 {
            if tx.network_id() != transaction.network_id() {
                return true;
            };

            holding_serial_numbers.extend(tx.old_serial_numbers());
            holding_commitments.extend(tx.new_commitments());
            holding_memos.push(tx.memorandum());
        }

        let transaction_serial_numbers = transaction.old_serial_numbers();
        let transaction_commitments = transaction.new_commitments();
        let transaction_memo = transaction.memorandum();

        // Check if the transactions in the block have duplicate serial numbers
        if has_duplicates(transaction_serial_numbers) {
            return true;
        }

        // Check if the transactions in the block have duplicate commitments
        if has_duplicates(transaction_commitments) {
            return true;
        }

        if holding_memos.contains(&transaction_memo) {
            return true;
        }

        for sn in transaction_serial_numbers {
            if holding_serial_numbers.contains(&sn) {
                return true;
            }
        }

        for cm in transaction_commitments {
            if holding_commitments.contains(&cm) {
                return true;
            }
        }

        false
    }

    /// Calculate the miner transaction fees from transactions.
    pub fn calculate_transaction_fees(&self) -> u64 {
        let mut balance = 0;

        for transaction in self.0.iter() {
            let value_balance = transaction.value_balance();

            // Only add to the transaction fee if the transaction is not a coinbase transaction
            if !value_balance.is_negative() {
                balance += value_balance as u64;
            }
        }

        balance
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
