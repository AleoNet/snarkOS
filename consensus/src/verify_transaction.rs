use snarkos_errors::{
    consensus::ConsensusError,
    storage::StorageError,
    unwrap_option_or_error,
};
use snarkos_objects::{Transaction, Transactions, Outpoint};
use snarkos_storage::{BlockStorage, Key, TransactionMeta};

/// Returns true if the given outpoint is already spent.
fn is_spent(storage: &BlockStorage, outpoint: &Outpoint) -> Result<bool, ConsensusError> {
    let transaction_id_key = Key::TransactionMeta(outpoint.transaction_id.clone());

    let transaction_meta: TransactionMeta = unwrap_option_or_error!(
             storage.get(&transaction_id_key)?.transaction_meta();
                ConsensusError::StorageError(StorageError::InvalidTransactionId(hex::encode(&outpoint.transaction_id)))
        );
    Ok(transaction_meta.spent[outpoint.index as usize])
}

/// Ensure that all inputs in a single transaction are unspent.
pub fn check_for_double_spend(storage: &BlockStorage, transaction: &Transaction) -> Result<(), ConsensusError> {
    for input in &transaction.parameters.inputs {
        // Already spent
        if is_spent(storage, &input.outpoint.clone())? {
            return Err(ConsensusError::AlreadySpent(
                input.outpoint.transaction_id.clone(),
                input.outpoint.index,
            ));
        }
    }
    Ok(())
}

/// Ensure that all inputs in all transactions are unspent.
pub fn check_for_double_spends(storage: &BlockStorage, transactions: &Transactions) -> Result<(), ConsensusError> {
    let mut new_spends: Vec<(Vec<u8>, u32)> = vec![];
    for transaction in transactions.iter() {
        for input in &transaction.parameters.inputs {
            if input.outpoint.is_coinbase() {
                continue;
            }
            let new_spend = (input.outpoint.transaction_id.clone(), input.outpoint.index);
            // Already spent
            if is_spent(&storage, &input.outpoint.clone())? || new_spends.contains(&new_spend) {
                return Err(ConsensusError::AlreadySpent(
                    input.outpoint.transaction_id.clone(),
                    input.outpoint.index,
                ));
            }
            new_spends.push(new_spend);
        }
    }
    Ok(())
}


/// Ensure that only one coinbase transaction exists for all transactions.
pub fn check_single_coinbase(transactions: &Transactions) -> Result<(), ConsensusError> {
    let mut coinbase_transaction_count = 0;
    for transaction in transactions.iter() {
        let input_length = transaction.parameters.inputs.len();
        for input in &transaction.parameters.inputs {
            if input.outpoint.is_coinbase() {
                if input_length > 1 {
                    return Err(ConsensusError::InvalidCoinbaseTransaction);
                }
                coinbase_transaction_count += 1;
            }
        }
    }
    match coinbase_transaction_count {
        1 => Ok(()),
        _ => Err(ConsensusError::MultipleCoinbaseTransactions(coinbase_transaction_count)),
    }
}

/// Perform the coinbase and double spend checks on all transactions
pub fn check_block_transactions(storage: &BlockStorage, transactions: &Transactions) -> Result<(), ConsensusError> {
    check_single_coinbase(transactions)?;
    check_for_double_spends(storage, transactions)
}
