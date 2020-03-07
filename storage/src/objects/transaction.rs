use crate::BlockStorage;
use snarkos_errors::{objects::transaction::TransactionError, unwrap_option_or_continue, unwrap_result_or_continue};
use snarkos_objects::{create_script_pub_key, BlockHeaderHash, Outpoint, Transaction, Transactions};
use wagyu_bitcoin::{BitcoinAddress, Mainnet};

/// Get a transaction bytes given the transaction id
pub fn get_transaction_bytes(
    storage: &BlockStorage,
    transaction_id: &Vec<u8>,
) -> Result<Transaction, TransactionError> {
    match storage.get_transaction(&transaction_id.clone()) {
        Some(transaction) => Ok(Transaction::deserialize(&transaction.transaction_bytes).unwrap()),
        None => Err(TransactionError::InvalidTransactionIdString(hex::encode(
            &transaction_id,
        ))),
    }
}

/// Find the outpoint given the transaction id and index
pub fn get_outpoint(
    storage: &BlockStorage,
    transaction_id: &Vec<u8>,
    index: usize,
) -> Result<Outpoint, TransactionError> {
    let transaction = get_transaction_bytes(storage, &transaction_id)?;

    if transaction.parameters.outputs.len() < index {
        return Err(TransactionError::InvalidOutpoint(hex::encode(transaction_id), index));
    }
    let output = transaction.parameters.outputs[index].clone();

    Ok(Outpoint {
        transaction_id: transaction_id.clone(),
        index: index as u32,
        script_pub_key: Some(output.script_pub_key),
        address: None,
    })
}

/// Returns true if the given outpoint is already spent.
/// Gets the previous transaction hash from storage and checks the outpoint's index.
pub fn is_spent(storage: &BlockStorage, outpoint: &Outpoint) -> Result<bool, TransactionError> {
    Ok(storage.get_transaction_meta(&outpoint.transaction_id.clone())?.spent[outpoint.index as usize])
}

/// Ensure that all inputs in a single transaction are unspent.
pub fn check_for_double_spend(storage: &BlockStorage, transaction: &Transaction) -> Result<(), TransactionError> {
    for input in &transaction.parameters.inputs {
        // Already spent
        if is_spent(storage, &input.outpoint.clone())? {
            return Err(TransactionError::AlreadySpent(
                input.outpoint.transaction_id.clone(),
                input.outpoint.index,
            ));
        }
    }
    Ok(())
}

/// Ensure that all inputs in all transactions are unspent.
pub fn check_for_double_spends(storage: &BlockStorage, transactions: &Transactions) -> Result<(), TransactionError> {
    let mut new_spends: Vec<(Vec<u8>, u32)> = vec![];
    for transaction in transactions.iter() {
        for input in &transaction.parameters.inputs {
            if input.outpoint.is_coinbase() {
                continue;
            }
            let new_spend = (input.outpoint.transaction_id.clone(), input.outpoint.index);
            // Already spent
            if is_spent(&storage, &input.outpoint.clone())? || new_spends.contains(&new_spend) {
                return Err(TransactionError::AlreadySpent(
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
fn check_single_coinbase(transactions: &Transactions) -> Result<(), TransactionError> {
    let mut coinbase_transaction_count = 0;
    for transaction in transactions.iter() {
        let input_length = transaction.parameters.inputs.len();
        for input in &transaction.parameters.inputs {
            if input.outpoint.is_coinbase() {
                if input_length > 1 {
                    return Err(TransactionError::InvalidCoinbaseTransaction);
                }
                coinbase_transaction_count += 1;
            }
        }
    }
    match coinbase_transaction_count {
        1 => Ok(()),
        _ => Err(TransactionError::MultipleCoinbaseTransactions(
            coinbase_transaction_count,
        )),
    }
}

/// Perform the coinbase and double spend checks on all transactions
pub fn check_block_transactions(storage: &BlockStorage, transactions: &Transactions) -> Result<(), TransactionError> {
    check_single_coinbase(transactions)?;
    check_for_double_spends(storage, transactions)
}

/// Get spendable amount given a list of outpoints
pub fn get_spendable_amount(storage: &BlockStorage, utxos: Vec<&Outpoint>) -> u64 {
    let mut balance: u64 = 0;

    for outpoint in utxos {
        let index = outpoint.index as usize;

        let transaction_value = unwrap_option_or_continue!(storage.get_transaction(&outpoint.transaction_id));
        let transaction: Transaction =
            unwrap_result_or_continue!(Transaction::deserialize(&transaction_value.transaction_bytes));

        let tx_outputs = transaction.parameters.outputs;

        if tx_outputs.len() > index && !unwrap_result_or_continue!(is_spent(storage, &outpoint)) {
            balance += tx_outputs[index].amount;
        }
    }

    balance
}

/// Calculate the miner transaction fees from transactions
pub fn calculate_transaction_fees(
    storage: &BlockStorage,
    transactions: &Transactions,
) -> Result<u64, TransactionError> {
    let mut balance = 0;

    for transaction in transactions.iter() {
        let mut valid_input_amounts = 0;
        let mut non_coinbase_outpoints: Vec<&Outpoint> = vec![];

        for input in &transaction.parameters.inputs {
            let outpoint = &input.outpoint;
            if !outpoint.is_coinbase() {
                non_coinbase_outpoints.push(outpoint);
            }
        }

        valid_input_amounts += get_spendable_amount(storage, non_coinbase_outpoints);

        if !transaction.is_coinbase() {
            balance += transaction.calculate_transaction_fee(valid_input_amounts)?;
        }
    }

    Ok(balance)
}

/// Traverse of blockchain to find the spendable outpoints and balances for an address
pub fn get_spendable_outpoints(storage: &BlockStorage, address: &BitcoinAddress<Mainnet>) -> Vec<(Outpoint, u64)> {
    let script_pub_key = create_script_pub_key(address).unwrap();

    let mut spendable_outpoints: Vec<(Outpoint, u64)> = vec![];

    for block_num in 0..=storage.get_latest_block_height() {
        // Get block header hash
        let block_hash: BlockHeaderHash = unwrap_result_or_continue!(storage.get_block_hash(block_num));

        // Get list of transaction ids
        let block_transactions = unwrap_result_or_continue!(storage.get_block_transactions(&block_hash));

        for transaction_id in block_transactions {
            let transaction = unwrap_result_or_continue!(get_transaction_bytes(storage, &transaction_id));

            for (output_index, output) in transaction.parameters.outputs.iter().enumerate() {
                // Output is spendable by this address
                if output.script_pub_key == script_pub_key {
                    // Get transaction meta
                    let transaction_meta = unwrap_result_or_continue!(storage.get_transaction_meta(&transaction_id));

                    if !transaction_meta.spent[output_index] && output.amount > 0 {
                        spendable_outpoints.push((
                            Outpoint {
                                transaction_id: transaction_id.clone(),
                                index: output_index as u32,
                                address: None,
                                script_pub_key: None,
                            },
                            output.amount,
                        ));
                    }
                }
            }
        }
    }

    spendable_outpoints
}

/// traverse of blockchain to find the balance for an address
pub fn get_balance(storage: &BlockStorage, address: &BitcoinAddress<Mainnet>) -> u64 {
    let mut balance: u64 = 0;

    for (_outpoint, outpoint_amount) in get_spendable_outpoints(&storage, address) {
        balance += outpoint_amount;
    }

    balance
}
