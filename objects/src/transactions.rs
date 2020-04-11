use crate::transaction::{Transaction, TransactionParameters, Vector};
use snarkos_errors::objects::TransactionError;
use snarkos_utilities::{
    bytes::{FromBytes, ToBytes},
    variable_length_integer::{read_variable_length_integer, variable_length_integer},
};

use std::{
    io::{Read, Result as IoResult, Write},
    ops::{Deref, DerefMut},
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Transactions(pub Vec<Transaction>);

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

impl ToBytes for Transactions {
    #[inline]
    fn write<W: Write>(&self, mut writer: W) -> IoResult<()> {
        variable_length_integer(self.0.len() as u64).write(&mut writer)?;

        for transaction in &self.0 {
            transaction.serialize().unwrap().write(&mut writer)?;
        }

        Ok(())
    }
}

impl FromBytes for Transactions {
    #[inline]
    fn read<R: Read>(mut reader: R) -> IoResult<Self> {
        let num_transactions = read_variable_length_integer(&mut reader)?;
        let mut transactions = vec![];
        for _ in 0..num_transactions {
            let parameters: TransactionParameters = TransactionParameters::read(&mut reader).unwrap();

            let transaction = Transaction { parameters };
            transactions.push(transaction);
        }

        Ok(Self(transactions))
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
