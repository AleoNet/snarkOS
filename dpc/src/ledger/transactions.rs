use crate::dpc::base_dpc::{transaction::DPCTransaction, BaseDPCComponents};

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
pub struct Transactions<C: BaseDPCComponents>(pub Vec<DPCTransaction<C>>);

impl<C: BaseDPCComponents> Transactions<C> {
    /// Initializes an empty list of transactions.
    pub fn new() -> Self {
        Self(vec![])
    }

    /// Initializes from a given list of transactions.
    pub fn from(transactions: &[DPCTransaction<C>]) -> Self {
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
            .map(|transaction| -> Result<Vec<u8>, TransactionError> { Ok(to_bytes![transaction]?) })
            .collect::<Result<Vec<Vec<u8>>, TransactionError>>()
    }
}

impl<C: BaseDPCComponents> ToBytes for Transactions<C> {
    #[inline]
    fn write<W: Write>(&self, mut writer: W) -> IoResult<()> {
        variable_length_integer(self.0.len() as u64).write(&mut writer)?;

        for transaction in &self.0 {
            transaction.write(&mut writer)?;
        }

        Ok(())
    }
}

impl<C: BaseDPCComponents> FromBytes for Transactions<C> {
    #[inline]
    fn read<R: Read>(mut reader: R) -> IoResult<Self> {
        let num_transactions = read_variable_length_integer(&mut reader)?;
        let mut transactions = vec![];
        for _ in 0..num_transactions {
            let transaction: DPCTransaction<C> = FromBytes::read(&mut reader)?;
            transactions.push(transaction);
        }

        Ok(Self(transactions))
    }
}

impl<C: BaseDPCComponents> Default for Transactions<C> {
    fn default() -> Self {
        Self::new()
    }
}

impl<C: BaseDPCComponents> Deref for Transactions<C> {
    type Target = Vec<DPCTransaction<C>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<C: BaseDPCComponents> DerefMut for Transactions<C> {
    fn deref_mut(&mut self) -> &mut Vec<DPCTransaction<C>> {
        &mut self.0
    }
}
