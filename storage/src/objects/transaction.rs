use crate::{BlockStorage, Key, KeyValue};
use snarkos_errors::{
    objects::transaction::TransactionError,
    storage::StorageError,
    unwrap_option_or_continue,
    unwrap_result_or_continue,
};
use snarkos_objects::{create_script_pub_key, BlockHeaderHash, Outpoint, Transaction, Transactions};
use wagyu_bitcoin::{BitcoinAddress, Mainnet};

impl BlockStorage {
    /// Get a transaction bytes given the transaction id.
    pub fn get_transaction_bytes(&self, transaction_id: &Vec<u8>) -> Result<Transaction, TransactionError> {
        match self.get_transaction(&transaction_id.clone()) {
            Some(transaction) => Ok(Transaction::deserialize(&transaction.transaction_bytes).unwrap()),
            None => Err(TransactionError::InvalidTransactionIdString(hex::encode(
                &transaction_id,
            ))),
        }
    }

    /// Find the outpoint given the transaction id and index.
    pub fn get_outpoint(&self, transaction_id: &Vec<u8>, index: usize) -> Result<Outpoint, TransactionError> {
        let transaction = self.get_transaction_bytes(&transaction_id)?;

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
    pub fn is_spent(&self, outpoint: &Outpoint) -> Result<bool, TransactionError> {
        Ok(self.get_transaction_meta(&outpoint.transaction_id.clone())?.spent[outpoint.index as usize])
    }

    /// Ensure that all inputs in a single transaction are unspent.
    pub fn check_for_double_spend(&self, transaction: &Transaction) -> Result<(), TransactionError> {
        for input in &transaction.parameters.inputs {
            // Already spent
            if self.is_spent(&input.outpoint.clone())? {
                return Err(TransactionError::AlreadySpent(
                    input.outpoint.transaction_id.clone(),
                    input.outpoint.index,
                ));
            }
        }
        Ok(())
    }

    /// Ensure that all inputs in all transactions are unspent.
    pub fn check_for_double_spends(&self, transactions: &Transactions) -> Result<(), TransactionError> {
        let mut new_spends: Vec<(Vec<u8>, u32)> = vec![];
        for transaction in transactions.iter() {
            for input in &transaction.parameters.inputs {
                if input.outpoint.is_coinbase() {
                    continue;
                }
                let new_spend = (input.outpoint.transaction_id.clone(), input.outpoint.index);
                // Already spent
                if self.is_spent(&input.outpoint.clone())? || new_spends.contains(&new_spend) {
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
    pub fn check_block_transactions(&self, transactions: &Transactions) -> Result<(), TransactionError> {
        BlockStorage::check_single_coinbase(transactions)?;
        self.check_for_double_spends(transactions)
    }

    /// Get spendable amount given a list of outpoints.
    fn get_spendable_amount(&self, utxos: Vec<&Outpoint>) -> u64 {
        let mut balance: u64 = 0;

        for outpoint in utxos {
            let index = outpoint.index as usize;

            let transaction_value = unwrap_option_or_continue!(self.get_transaction(&outpoint.transaction_id));
            let transaction: Transaction =
                unwrap_result_or_continue!(Transaction::deserialize(&transaction_value.transaction_bytes));

            let tx_outputs = transaction.parameters.outputs;

            if tx_outputs.len() > index && !unwrap_result_or_continue!(self.is_spent(&outpoint)) {
                balance += tx_outputs[index].amount;
            }
        }

        balance
    }

    /// Calculate the miner transaction fees from transactions.
    pub fn calculate_transaction_fees(&self, transactions: &Transactions) -> Result<u64, TransactionError> {
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

            valid_input_amounts += self.get_spendable_amount(non_coinbase_outpoints);

            if !transaction.is_coinbase() {
                balance += transaction.calculate_transaction_fee(valid_input_amounts)?;
            }
        }

        Ok(balance)
    }

    /// Traverse of blockchain to find the spendable outpoints and balances for an address.
    pub fn get_spendable_outpoints(&self, address: &BitcoinAddress<Mainnet>) -> Vec<(Outpoint, u64)> {
        let script_pub_key = create_script_pub_key(address).unwrap();

        let mut spendable_outpoints: Vec<(Outpoint, u64)> = vec![];

        for block_num in 0..=self.get_latest_block_height() {
            // Get block header hash
            let block_hash: BlockHeaderHash = unwrap_result_or_continue!(self.get_block_hash(block_num));

            // Get list of transaction ids
            let block_transactions = unwrap_result_or_continue!(self.get_block_transactions(&block_hash));

            for transaction_id in block_transactions {
                let transaction = unwrap_result_or_continue!(self.get_transaction_bytes(&transaction_id));

                for (output_index, output) in transaction.parameters.outputs.iter().enumerate() {
                    // Output is spendable by this address
                    if output.script_pub_key == script_pub_key {
                        // Get transaction meta
                        let transaction_meta = unwrap_result_or_continue!(self.get_transaction_meta(&transaction_id));

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
    pub fn get_balance(&self, address: &BitcoinAddress<Mainnet>) -> u64 {
        let mut balance: u64 = 0;

        for (_outpoint, outpoint_amount) in self.get_spendable_outpoints(address) {
            balance += outpoint_amount;
        }

        balance
    }

    /// Decrement or remove the transaction_value
    pub fn decrement_transaction_value(&self, transaction_id: &Vec<u8>) -> Result<(), StorageError> {
        let transaction_id_key = Key::Transactions(transaction_id.clone());

        match self.get(&transaction_id_key) {
            Ok(value) => match value.transactions() {
                Some(transaction_value) => {
                    let tx_value = transaction_value.decrement();

                    if tx_value.count > 0 {
                        // Update
                        let update_transaction = KeyValue::Transactions(transaction_id.clone(), tx_value);
                        self.storage.insert(update_transaction)
                    } else {
                        // Remove
                        self.storage.remove(&transaction_id_key)?;
                        self.storage.remove(&Key::TransactionMeta(transaction_id.clone()))
                    }
                }
                None => Ok(()),
            },
            Err(_) => Ok(()),
        }
    }
}
