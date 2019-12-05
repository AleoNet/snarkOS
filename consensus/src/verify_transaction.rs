use snarkos_errors::consensus::ConsensusError;
use snarkos_objects::{Transaction, Transactions};
use snarkos_storage::BlockStorage;

pub fn check_for_double_spends(storage: &BlockStorage, transactions: &Transactions) -> Result<(), ConsensusError> {
    let mut new_spends: Vec<(Vec<u8>, u32)> = vec![];
    for transaction in transactions.iter() {
        for input in &transaction.parameters.inputs {
            if input.outpoint.is_coinbase() {
                continue;
            }
            let new_spend = (input.outpoint.transaction_id.clone(), input.outpoint.index);
            // Already spent
            if storage.is_spent(&input.outpoint.clone())? || new_spends.contains(&new_spend) {
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

pub fn check_for_double_spend(storage: &BlockStorage, transaction: &Transaction) -> Result<(), ConsensusError> {
    for input in &transaction.parameters.inputs {
        // Already spent
        if storage.is_spent(&input.outpoint.clone())? {
            return Err(ConsensusError::AlreadySpent(
                input.outpoint.transaction_id.clone(),
                input.outpoint.index,
            ));
        }
    }
    Ok(())
}

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

pub fn check_block_transactions(storage: &BlockStorage, transactions: &Transactions) -> Result<(), ConsensusError> {
    check_single_coinbase(transactions)?;
    check_for_double_spends(storage, transactions)
}
