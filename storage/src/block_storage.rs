use crate::{
    bytes_to_u32,
    BlockPath,
    Key,
    KeyValue,
    SideChainPath,
    Storage,
    TransactionMeta,
    TransactionValue,
    Value,
    KEY_BEST_BLOCK_NUMBER,
    KEY_MEMORY_POOL,
    NUM_COLS,
};
use snarkos_errors::{
    storage::StorageError,
    unwrap_option_or_continue,
    unwrap_result_or_continue,
    unwrap_option_or_error,
};
use snarkos_objects::{create_script_pub_key, Block, BlockHeader, BlockHeaderHash, Outpoint, Transaction, Transactions, TransactionInput};

use parking_lot::RwLock;
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};
use wagyu_bitcoin::{BitcoinAddress, Mainnet};

pub struct BlockStorage {
    pub latest_block_height: RwLock<u32>,
    pub storage: Arc<Storage>,
}

impl BlockStorage {
    /// Create a new storage
    pub fn new() -> Result<Arc<Self>, StorageError> {
        let mut path = std::env::current_dir()?;
        path.push("../../db");

        let genesis = "00000000000000000000000000000000000000000000000000000000000000008c8d4f393f39c063c40a617c6e2584e6726448c4c0f7da7c848bfa573e628388fbf1285e00000000ffffffffff7f00005e4401000101000000010000000000000000000000000000000000000000000000000000000000000000ffffffff04010000000100e1f505000000001976a914ef5392fc02643be8b98f6aaca5c1ffaab238916a88ac".into();

        BlockStorage::open_at_path(path, genesis)
    }

    /// Open the blockchain storage at a particular path
    pub fn open_at_path<P: AsRef<Path>>(path: P, genesis: String) -> Result<Arc<Self>, StorageError> {
        fs::create_dir_all(path.as_ref()).map_err(|err| StorageError::Message(err.to_string()))?;

        match Storage::open_cf(path, NUM_COLS) {
            Ok(storage) => Self::get_latest_state(storage, genesis),
            Err(err) => return Err(err),
        }
    }

    /// Get the latest state of the storage
    pub fn get_latest_state(storage: Storage, genesis: String) -> Result<Arc<Self>, StorageError> {
        let value = storage.get(&Key::Meta(KEY_BEST_BLOCK_NUMBER))?;

        match value {
            Some(val) => Ok(Arc::new(Self {
                latest_block_height: RwLock::new(bytes_to_u32(val)),
                storage: Arc::new(storage),
            })),
            None => {
                // Add genesis block to database

                let block_storage = Self {
                    latest_block_height: RwLock::new(0),
                    storage: Arc::new(storage),
                };

                let genesis_block = Block::deserialize(&hex::decode(genesis)?)?;

                block_storage.insert_and_commit(genesis_block)?;

                Ok(Arc::new(block_storage))
            }
        }
    }

    /// Returns true if there are no blocks in the chain.
    pub fn is_empty(&self) -> bool {//TODO: FIX THIS FUNCTION
        self.get_latest_block().is_err()
    }

    /// Retrieve a value given a key
    pub fn get(&self, key: &Key) -> Result<Value, StorageError> {
        match self.storage.get(key)? {
            Some(data) => Ok(Value::from_bytes(&key, &data)?),
            None => Err(StorageError::MissingValue(key.to_string())),
        }
    }

    /// Get a block header given the block hash
    pub fn get_block_header(&self, block_hash: BlockHeaderHash) -> Result<BlockHeader, StorageError> {
        Ok(unwrap_option_or_error!(
            self.get(&Key::BlockHeaders(block_hash))?.block_header();
            StorageError::MissingValue(block_header_key.to_string())
        ))
    }

    /// Get the block hash given a block number
    pub fn get_block_hash(&self, block_num: u32) -> Result<BlockHeaderHash, StorageError> {
        Ok(unwrap_option_or_error!(
            self.get(&Key::BlockHashes(block_num))?.block_hash();
            StorageError::InvalidBlockNumber(block_num)
        ))
    }

    /// Get the block num given a block hash
    pub fn get_block_num(&self, block_hash: &BlockHeaderHash) -> Result<u32, StorageError> {
        Ok(unwrap_option_or_error!(
            self.get(&Key::BlockNumbers(block_hash.clone()))?.block_number();
            StorageError::InvalidBlockHash(hex::encode(block_hash.0))
        ))
    }

    /// Get a transaction given the transaction id
    pub fn get_transaction(&self, transaction_id: &Vec<u8>) -> Option<TransactionValue> {
        match self.get(&Key::Transactions(transaction_id.clone())) {
            Ok(value) => match value.transactions() {
                Some(transaction_value) => Some(transaction_value),
                None => None,
            },
            Err(_) => None,
        }
    }

    pub fn get_transaction_meta(&self, input: TransactionInput) -> Result<TransactionMeta, StorageError>{
        Ok(unwrap_option_or_error!(
            self.get(&Key::TransactionMeta(input.outpoint.transaction_id.clone()))?.transaction_meta();
            StorageError::InvalidTransactionMeta(hex::encode(&input.outpoint.transaction_id))
        ))
    }

    /// Get a transaction bytes given the transaction id
    pub fn get_transaction_bytes(&self, transaction_id: &Vec<u8>) -> Result<Transaction, StorageError> {//TODO: MOVE
        let transaction_value: TransactionValue = unwrap_option_or_error!(
            self.get(&Key::Transactions(transaction_id.clone()))?.transactions();
            StorageError::InvalidTransactionId(hex::encode(transaction_id))
        );

        Ok(Transaction::deserialize(&transaction_value.transaction_bytes)?)
    }

    /// Get a block given the block hash
    pub fn get_block(&self, block_hash: BlockHeaderHash) -> Result<Block, StorageError> {//TODO: MOVE
        let block_transactions: Vec<Vec<u8>> = unwrap_option_or_error!(
            self.get(&Key::BlockTransactions(block_hash.clone()))?.block_transaction();
            StorageError::MissingValue(block_transactions_key.to_string())
        );

        let mut transactions = vec![];
        for block_transaction_id in block_transactions {
            transactions.push(self.get_transaction_bytes(&block_transaction_id)?);
        }

        Ok(Block {
            header: self.get_block_header(block_hash)?,
            transactions: Transactions::from(&transactions),
        })
    }

    /// Find the potential child block given a parent block header
    pub fn find_child_block(&self, parent_header: &BlockHeaderHash) -> Result<BlockHeaderHash, StorageError> {
        Ok(unwrap_option_or_error!(
            self.get(&Key::ChildHashes(parent_header.clone()))?.child_hashes();
            StorageError::InvalidParentHash(hex::encode(parent_header.0))
        ))
    }

    /// Get the latest block height of the chain
    pub fn get_latest_block_height(&self) -> u32 {
        *self.latest_block_height.read()
    }

    /// Get the latest number of blocks in the chain
    pub fn get_block_count(&self) -> u32 {
        *self.latest_block_height.read() + 1
    }

    // OBJECT TRANSACTION METHODS ==================================================================

    /// Returns true if the given outpoint is already spent.
    pub fn is_spent(&self, outpoint: &Outpoint) -> Result<bool, StorageError> {//TODO: MOVE
        let transaction_meta: TransactionMeta = unwrap_option_or_error!(
             self.get(&Key::TransactionMeta(outpoint.transaction_id.clone()))?.transaction_meta();
            StorageError::InvalidTransactionId(hex::encode(&outpoint.transaction_id))
        );

        Ok(transaction_meta.spent[outpoint.index as usize])
    }

    /// Find the outpoint given the transaction id and index
    pub fn get_outpoint(&self, transaction_id: &Vec<u8>, index: usize) -> Result<Outpoint, StorageError> {//TODO: MOVE
        let transaction = self.get_transaction_bytes(&transaction_id)?;

        if transaction.parameters.outputs.len() < index {
            return Err(StorageError::InvalidOutpoint(hex::encode(transaction_id), index));
        }
        let output = transaction.parameters.outputs[index].clone();

        Ok(Outpoint {
            transaction_id: transaction_id.clone(),
            index: index as u32,
            script_pub_key: Some(output.script_pub_key),
            address: None,
        })
    }

    /// Get spendable amount given a list of outpoints
    pub fn get_spendable_amount(&self, utxos: Vec<&Outpoint>) -> u64 {//TODO: MOVE
        let mut balance: u64 = 0;

        for outpoint in utxos {
            let index = outpoint.index as usize;

            let transaction_value: Value =
                unwrap_result_or_continue!(self.get(&Key::Transactions(outpoint.transaction_id.clone())));
            let transaction_bytes: Vec<u8> =
                unwrap_option_or_continue!(transaction_value.transactions()).transaction_bytes;

            let transaction: Transaction = unwrap_result_or_continue!(Transaction::deserialize(&transaction_bytes));

            let tx_outputs = transaction.parameters.outputs;

            if tx_outputs.len() > index && !unwrap_result_or_continue!(self.is_spent(&outpoint)) {
                balance += tx_outputs[index].amount;
            }
        }

        balance
    }

    /// Traverse of blockchain to find the spendable outpoints and balances for an address
    pub fn get_spendable_outpoints(&self, address: &BitcoinAddress<Mainnet>) -> Vec<(Outpoint, u64)> {//TODO: MOVE
        let script_pub_key = create_script_pub_key(address).unwrap();

        let mut spendable_outpoints: Vec<(Outpoint, u64)> = vec![];

        for block_num in 0..=self.get_latest_block_height() {
            // Get block header hash
            let block_hash_value: Value = unwrap_result_or_continue!(self.get(&Key::BlockHashes(block_num)));
            let block_hash: BlockHeaderHash = unwrap_option_or_continue!(block_hash_value.block_hash());

            // Get list of transaction ids
            let block_transactions_value =
                unwrap_result_or_continue!(self.get(&Key::BlockTransactions(block_hash)));
            let block_transactions: Vec<Vec<u8>> =
                unwrap_option_or_continue!(block_transactions_value.block_transaction());

            for transaction_id in block_transactions {
                // Get transaction bytes
                let transaction_value: Value =
                    unwrap_result_or_continue!(self.get(&Key::Transactions(transaction_id.clone())));
                let transaction_bytes: Vec<u8> =
                    unwrap_option_or_continue!(transaction_value.transactions()).transaction_bytes;

                let transaction: Transaction = unwrap_result_or_continue!(Transaction::deserialize(&transaction_bytes));

                for (output_index, output) in transaction.parameters.outputs.iter().enumerate() {
                    // Output is spendable by this address
                    if output.script_pub_key == script_pub_key {
                        // Get transaction meta
                        let transaction_meta_value: Value =
                            unwrap_result_or_continue!(self.get(&Key::TransactionMeta(transaction_id.clone())));
                        let transaction_meta: TransactionMeta =
                            unwrap_option_or_continue!(transaction_meta_value.transaction_meta());

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

        for (_outpoint, outpoint_amount) in self.get_spendable_outpoints(address) {//TODO: MOVE
            balance += outpoint_amount;
        }

        balance
    }

    /// Calculate the miner transaction fees from transactions
    pub fn calculate_transaction_fees(&self, transactions: &Transactions) -> Result<u64, StorageError> {//TODO: MOVE
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

    // OBJECT BLOCK HEADER METHODS =================================================================

    /// Returns true if the block for the given block header hash exists.
    pub fn is_exist(&self, block_hash: &BlockHeaderHash) -> bool {//TODO: MOVE
        if self.is_empty() {
            return false;
        }

        match self.get(&Key::BlockHeaders(block_hash.clone())) {
            Ok(block_header) => block_header.block_header().is_some(),
            Err(_) => return false,
        }
    }

    /// Returns the latest shared block header hash.
    /// If the block locator hashes are for a side chain, returns the common point of fork.
    /// If the block locator hashes are for the canon chain, returns the latest block header hash.
    pub fn get_latest_shared_hash(
        &self,
        block_locator_hashes: Vec<BlockHeaderHash>,
    ) -> Result<BlockHeaderHash, StorageError> {//TODO: MOVE
        for block_hash in block_locator_hashes {
            if self.is_canon(&block_hash) {
                return Ok(block_hash);
            }
        }

        self.get_block_hash(0)
    }

    /// Get the list of block locator hashes (Bitcoin protocol)
    pub fn get_block_locator_hashes(&self) -> Result<Vec<BlockHeaderHash>, StorageError> {//TODO: MOVE
        let mut step = 1;
        let mut index = self.get_latest_block_height();
        let mut block_locator_hashes = vec![];

        while index > 0 {
            block_locator_hashes.push(self.get_block_hash(index)?);

            if block_locator_hashes.len() >= 10 {
                step *= 2;
            }

            if index < step {
                if index != 1 {
                    block_locator_hashes.push(self.get_block_hash(0)?);
                }

                break;
            }

            index -= step;
        }

        Ok(block_locator_hashes)
    }

    // OBJECT BLOCK METHODS ========================================================================

    /// Get the latest block in the chain
    pub fn get_latest_block(&self) -> Result<Block, StorageError> {//TODO: MOVE
        self.get_block_from_block_num(self.get_latest_block_height())
    }

    /// Find the potential parent block given a block header
    pub fn find_parent_block(&self, block_header: &BlockHeader) -> Result<Block, StorageError> {//TODO: MOVE/DELETE
        self.get_block(block_header.previous_block_hash.clone())
    }

    /// Returns true if the block exists in the canon chain.
    pub fn is_canon(&self, block_hash: &BlockHeaderHash) -> bool {//TODO: MOVE
        self.is_exist(block_hash) && self.get(&Key::BlockNumbers(block_hash.clone())).is_ok()
    }

    /// Returns true if the block corresponding to this block's previous_block_hash exists.
    pub fn is_previous_block_exist(&self, block: &Block) -> bool {//TODO: MOVE
        self.is_exist(&block.header.previous_block_hash)
    }

    /// Returns true if the block corresponding to this block's previous_block_hash is in the canon chain.
    pub fn is_previous_block_canon(&self, block: &Block) -> bool {//TODO MOVE
        self.is_canon(&block.header.previous_block_hash)
    }

    /// Get a block given the block number
    pub fn get_block_from_block_num(&self, block_num: u32) -> Result<Block, StorageError> {//TODO: MOVE
        if block_num > self.get_latest_block_height() {
            return Err(StorageError::InvalidBlockNumber(block_num));
        }

        let block_hash: BlockHeaderHash = unwrap_option_or_error!(
            self.get(&Key::BlockHashes(block_num))?.block_hash();
            StorageError::MissingValue(block_hash_key.to_string())
        );

        self.get_block(block_hash)
    }

    /// Returns the block number of a conflicting block that has already been mined
    pub fn already_mined(&self, block: &Block) -> Result<Option<u32>, StorageError> {//TODO: MOVE
        // look up new block's previous block by hash
        // if the block after previous_block_number exists, then someone has already mined this new block
        let previous_block_number: u32 = unwrap_option_or_error!(
            self.get(&Key::BlockNumbers(block.header.previous_block_hash.clone()))?.block_number();
            StorageError::MissingValue(previous_block_number_key.to_string())
        );

        let existing_block_number = previous_block_number + 1;

        if self.get_block_from_block_num(existing_block_number).is_ok() {
            // the storage has a conflicting block with the same previous_block_hash
            Ok(Some(existing_block_number))
        } else {
            // the new block has no conflicts
            Ok(None)
        }
    }

    // BLOCK PATH METHOD ===========================================================================

    /// Get the block's path/origin
    pub fn get_block_path(&self, block_header: &BlockHeader) -> Result<BlockPath, StorageError> {//TODO: MOVE
        let block_hash = block_header.get_hash();
        if self.is_exist(&block_hash) {
            return Ok(BlockPath::ExistingBlock);
        }

        if &self.get_latest_block()?.header.get_hash() == &block_header.previous_block_hash {
            return Ok(BlockPath::CanonChain(self.get_latest_block_height() + 1));
        }

        const OLDEST_FORK_THRESHOLD: u32 = 1024;
        let mut side_chain_path = vec![];
        let mut parent_hash = block_header.previous_block_hash.clone();

        for _ in 0..=OLDEST_FORK_THRESHOLD {
            // check if the part is part of the canon chain
            match &self.get_block_num(&parent_hash) {
                // This is a canon parent
                Ok(block_num) => {
                    return Ok(BlockPath::SideChain(SideChainPath {
                        shared_block_number: *block_num,
                        new_block_number: block_num + side_chain_path.len() as u32 + 1,
                        path: side_chain_path,
                    }));
                }
                // Add to the side_chain_path
                Err(_) => {
                    side_chain_path.insert(0, parent_hash.clone());
                    parent_hash = self.get_block_header(parent_hash)?.previous_block_hash;
                }
            }
        }

        Err(StorageError::IrrelevantBlock)
    }

    // OBJECT MEMORY POOL METHODS ==================================================================

    /// Get the stored memory pool transactions
    pub fn get_memory_pool_transactions(&self) -> Result<Option<Vec<u8>>, StorageError> {//TODO: EDIT
        Ok(self.get(&Key::Meta(KEY_MEMORY_POOL))?.meta())
    }

    /// Store the memory pool transactions
    pub fn store_to_memory_pool(&self, transactions_serialized: Vec<u8>) -> Result<(), StorageError> {
        self.storage
            .insert(KeyValue::Meta(KEY_MEMORY_POOL, transactions_serialized))
    }

    // INSERT COMMIT =================================================================================

    /// Insert a block into the storage but do not commit
    pub fn insert_only(&self, block: Block) -> Result<(), StorageError> {
        // Verify that the block does not already exist in storage.
        if self.is_exist(&block.header.get_hash()) {
            return Err(StorageError::BlockExists(block.header.get_hash().0));
        }

        let transaction_ids: Vec<Vec<u8>> = block.transactions.to_transaction_ids()?;
        let transaction_bytes: Vec<Vec<u8>> = block.transactions.serialize()?;

        let mut transactions_to_store = vec![];
        for (index, tx_bytes) in transaction_bytes.iter().enumerate() {
            let transaction_value = match self.get_transaction(&transaction_ids[index]) {
                Some(transaction_value) => transaction_value.increment(),
                None => TransactionValue::new(tx_bytes.clone()),
            };

            transactions_to_store.push(KeyValue::Transactions(
                transaction_ids[index].clone(),
                transaction_value,
            ));

            transactions_to_store.push(KeyValue::TransactionMeta(
                transaction_ids[index].clone(),
                TransactionMeta {
                    spent: vec![false; block.transactions[index].parameters.outputs.len()],
                },
            ));
        }

        let block_header_hash = block.header.get_hash();
        let block_transactions = KeyValue::BlockTransactions(block_header_hash.clone(), transaction_ids);
        let child_hashes = KeyValue::ChildHashes(block.header.previous_block_hash.clone(), block_header_hash.clone());
        let block_header = KeyValue::BlockHeaders(block_header_hash, block.header);

        self.storage
            .insert_batch(vec![block_header, block_transactions, child_hashes])?;
        self.storage.insert_batch(transactions_to_store)?;

        Ok(())
    }

    /// Insert a block into the storage and commit as part of the longest chain
    pub fn insert_and_commit(&self, block: Block) -> Result<(), StorageError> {
        let block_hash = block.header.get_hash();

        // If the block does not exist in the storage
        if !self.is_exist(&block_hash) {
            // Insert it first
            self.insert_only(block)?;
        }
        // Commit it
        self.commit(block_hash)
    }

    /// Commit/canonize a particular block
    pub fn commit(&self, block_header_hash: BlockHeaderHash) -> Result<(), StorageError> {
        let block = self.get_block(block_header_hash.clone())?;

        let is_genesis = block.header.previous_block_hash == BlockHeaderHash([0u8; 32])
            && self.get_latest_block_height() == 0
            && self.is_empty();

        if !is_genesis {
            let latest_block = self.get_latest_block()?;

            if latest_block.header.get_hash() != block.header.previous_block_hash {
                return Err(StorageError::InvalidNextBlock(
                    latest_block.header.get_hash().to_string(),
                    block.header.previous_block_hash.to_string(),
                ));
            }
        }

        // Update transaction spent status

        let mut transaction_meta_updates: HashMap<Vec<u8>, TransactionMeta> = HashMap::new();
        for transaction in block.transactions.iter() {
            for input in &transaction.parameters.inputs {
                if input.outpoint.is_coinbase() {
                    continue;
                }

                let mut new_transaction_meta = match transaction_meta_updates.get(&input.outpoint.transaction_id) {
                    Some(transaction_meta) => transaction_meta.clone(),
                    None => {
                        unwrap_option_or_error!(
                            self.get(&Key::TransactionMeta(input.outpoint.transaction_id.clone()))?.transaction_meta();
                            StorageError::InvalidTransactionMeta(hex::encode(&input.outpoint.transaction_id))
                        )
                    }
                };

                if new_transaction_meta.spent[input.outpoint.index as usize] {
                    return Err(StorageError::DoubleSpend(hex::encode(&input.outpoint.transaction_id)));
                }

                new_transaction_meta.spent[input.outpoint.index as usize] = true;
                transaction_meta_updates.insert(input.outpoint.transaction_id.clone(), new_transaction_meta);
            }
        }

        let mut update_spent_transactions = vec![];
        for (txid, transaction_meta) in transaction_meta_updates {
            update_spent_transactions.push(KeyValue::TransactionMeta(txid, transaction_meta));
        }

        // Handle storage inserts and height update

        let mut height = self.latest_block_height.write();
        let mut new_best_block_number = 0;
        if !is_genesis {
            new_best_block_number = *height + 1;
        }

        let best_block_number = KeyValue::Meta(KEY_BEST_BLOCK_NUMBER, new_best_block_number.to_le_bytes().to_vec());
        let block_hash = KeyValue::BlockHashes(new_best_block_number, block_header_hash.clone());
        let block_numbers = KeyValue::BlockNumbers(block_header_hash, new_best_block_number);

        self.storage
            .insert_batch(vec![best_block_number, block_hash, block_numbers])?;
        self.storage.insert_batch(update_spent_transactions)?;

        if !is_genesis {
            *height += 1;
        }

        Ok(())
    }

    /// Remove a block and it's related data from the storage
    pub fn remove_block(&self, block_hash: BlockHeaderHash) -> Result<(), StorageError> {
        let block_header_key = Key::BlockHeaders(block_hash.clone());
        let block_transactions_key = Key::BlockTransactions(block_hash);

        let block_transactions: Vec<Vec<u8>> = unwrap_option_or_error!(
           self.get(&block_transactions_key)?.block_transaction();
           StorageError::MissingValue(block_transactions_key.to_string())
        );

        for block_transaction_id in block_transactions {
            self.decrement_transaction_value(&block_transaction_id)?;
        }

        self.storage
            .remove_batch(vec![block_header_key, block_transactions_key])?;

        Ok(())
    }

    /// Remove the latest block
    pub fn remove_latest_block(&self) -> Result<(), StorageError> {
        // De-commit the block from the valid chain

        let latest_block_height = self.get_latest_block_height();
        if latest_block_height == 0 {
            return Err(StorageError::InvalidBlockRemovalNum(0, 0));
        }
        let block_hash_key = Key::BlockHashes(latest_block_height);

        let block_hash: BlockHeaderHash = unwrap_option_or_error!(
            self.get(&block_hash_key)?.block_hash();
            StorageError::MissingValue(block_hash_key.to_string())
        );

        let block_numbers_key = Key::BlockNumbers(block_hash.clone());
        let block_transactions_key = Key::BlockTransactions(block_hash.clone());

        let block_transactions: Vec<Vec<u8>> = unwrap_option_or_error!(
            self.get(&block_transactions_key)?.block_transaction();
            StorageError::MissingValue(block_numbers_key.to_string())
        );

        let mut transaction_meta_updates: HashMap<Vec<u8>, TransactionMeta> = HashMap::new();

        for block_transaction_id in block_transactions {
            // Update transaction meta spends

            for input in self.get_transaction_bytes(&block_transaction_id)?.parameters.inputs {
                if input.outpoint.is_coinbase() {
                    continue;
                }

                let mut new_transaction_meta = match transaction_meta_updates.get(&input.outpoint.transaction_id) {
                    Some(transaction_meta) => transaction_meta.clone(),
                    None => {
                        unwrap_option_or_continue!(
                        self.get(&Key::TransactionMeta(input.outpoint.transaction_id.clone()))?.transaction_meta();)
                    }
                };

                new_transaction_meta.spent[input.outpoint.index as usize] = false;
                transaction_meta_updates.insert(input.outpoint.transaction_id.clone(), new_transaction_meta);
            }
        }

        // Update spent status of relevant utxos

        let mut update_spent_transactions = vec![];
        for (txid, transaction_meta) in transaction_meta_updates {
            update_spent_transactions.push(KeyValue::TransactionMeta(txid, transaction_meta));
        }

        let update_best_block_num = latest_block_height - 1;
        let best_block_number = KeyValue::Meta(KEY_BEST_BLOCK_NUMBER, (update_best_block_num).to_le_bytes().to_vec());

        let mut storage_inserts = vec![best_block_number];
        storage_inserts.extend(update_spent_transactions);

        self.storage.insert_batch(storage_inserts)?;
        self.storage.remove_batch(vec![block_hash_key, block_numbers_key])?;

        let mut latest_block_height = self.latest_block_height.write();
        *latest_block_height -= 1;

        // Remove the block structure

        self.remove_block(block_hash)?;

        Ok(())
    }

    /// Remove the latest `num_blocks` blocks.
    pub fn remove_latest_blocks(&self, num_blocks: u32) -> Result<(), StorageError> {
        let latest_block_height = self.get_latest_block_height();
        if num_blocks > latest_block_height {
            return Err(StorageError::InvalidBlockRemovalNum(num_blocks, latest_block_height));
        }

        for _ in 0..num_blocks {
            self.remove_latest_block()?;
        }
        Ok(())
    }

    /// Revert the chain to the state before the fork
    pub fn revert_for_fork(&self, side_chain_path: &SideChainPath) -> Result<(), StorageError> {
        let latest_block_height = self.get_latest_block_height();

        if side_chain_path.new_block_number > latest_block_height {
            for _ in (side_chain_path.shared_block_number)..latest_block_height {
                self.remove_latest_block()?;
            }
        }

        Ok(())
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

    /// Destroy the storage given a path
    pub fn destroy_storage(path: PathBuf) -> Result<(), StorageError> {
        Storage::destroy_storage(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use hex;
    use std::str::FromStr;
    use wagyu_bitcoin::{BitcoinAddress, Mainnet};

    const TEST_DB_PATH: &str = "../test_db";

    pub struct Wallet {
        pub private_key: &'static str,
        pub address: &'static str,
    }

    const TEST_WALLETS: [Wallet; 5] = [
        Wallet {
            private_key: "KzW6KyJ1s4mp3CFDUzCXFh4r2xzyd2rkMwcbeP5T2T2iMvepkAwS",
            address: "1NpScgYSLW4WcvmZM55EY5cziEiqZx3wJu",
        },
        Wallet {
            private_key: "L2tBggaVMYPghRB6LR2ThY5Er1Rc284T3vgiK274JpaFsj1tVSsT",
            address: "167CPx9Ae96iVQCrwoq17jwKmmvr9RTyM7",
        },
        Wallet {
            private_key: "KwrJGqYZVj3m2WyimxdLBNrdwQZBVnHhw78c73xuLSWkjFBiqq3P",
            address: "1Dy6XpKrNRDw9SewppvYpGHSMbBExVmZsU",
        },
        Wallet {
            private_key: "KwwZ97gYoBBf6cGLp33qD8v4pEKj89Yir65vUA3N5Y1AtWbLzqED",
            address: "1CL1zq3kLK3TFNLdTk4HtuguT7JMdD5vi5",
        },
        Wallet {
            private_key: "L4cR7BQfvj6CPdbaTvRKHJXB4LjaUHJxtrDqNzkkyRXqrqUxLQTS",
            address: "1Hz8RzEXYPF6z8o7z5SHVnjzmhqS5At5kU",
        },
    ];

    const GENESIS_BLOCK: &str = "0000000000000000000000000000000000000000000000000000000000000000b3d9ad9de8e21b2b3a9ffb40bae6fefa852026e7fb2e279322cd7589a20ee35592ec145e00000000ffffffffff7f000030d901000101000000010000000000000000000000000000000000000000000000000000000000000000ffffffff04080000000100e1f505000000001976a914ef5392fc02643be8b98f6aaca5c1ffaab238916a88ac";

    pub fn random_storage_path() -> String {
        let ptr = Box::into_raw(Box::new(123));
        format!("{}{}", TEST_DB_PATH, ptr as usize)
    }

    pub fn kill_storage(storage: Arc<BlockStorage>, path: PathBuf) {
        drop(storage);
        BlockStorage::destroy_storage(path).unwrap();
    }

    #[test]
    pub fn test_initialize_blockchain() {
        let mut path = std::env::current_dir().unwrap();
        path.push(random_storage_path());

        BlockStorage::destroy_storage(path.clone()).unwrap();

        let blockchain = BlockStorage::open_at_path(path.clone(), GENESIS_BLOCK.into()).unwrap();

        assert_eq!(blockchain.get_latest_block_height(), 0);

        let latest_block = blockchain.get_latest_block().unwrap();

        let genesis_block = Block::deserialize(&hex::decode(&GENESIS_BLOCK).unwrap()).unwrap();

        assert_eq!(genesis_block, latest_block);

        let address = BitcoinAddress::<Mainnet>::from_str(TEST_WALLETS[0].address).unwrap();

        assert_eq!(blockchain.get_balance(&address), 100000000);
        assert!(blockchain.remove_latest_block().is_err());

        kill_storage(blockchain, path);
    }

    #[test]
    pub fn test_storage() {
        let mut path = std::env::current_dir().unwrap();
        path.push(random_storage_path());

        let blockchain = BlockStorage::open_at_path(path.clone(), GENESIS_BLOCK.into()).unwrap();

        blockchain.storage.storage.put(b"my key", b"my value").unwrap();

        match blockchain.storage.storage.get(b"my key") {
            Ok(Some(value)) => println!("retrieved value {}", String::from_utf8(value).unwrap()),
            Ok(None) => println!("value not found"),
            Err(e) => println!("operational problem encountered: {}", e),
        }

        assert!(blockchain.storage.storage.get(b"my key").is_ok());

        kill_storage(blockchain, path);
    }

    #[test]
    pub fn test_destroy_storage() {
        let mut path = std::env::current_dir().unwrap();
        path.push(random_storage_path());

        BlockStorage::destroy_storage(path).unwrap();
    }

    mod test_invalid {
        use super::*;
        use snarkos_objects::{BlockHeader, MerkleRootHash};

        #[test]
        pub fn test_invalid_block_addition() {
            let mut path = std::env::current_dir().unwrap();
            path.push(random_storage_path());

            BlockStorage::destroy_storage(path.clone()).unwrap();

            let blockchain = BlockStorage::open_at_path(path.clone(), GENESIS_BLOCK.into()).unwrap();

            let random_block_header = BlockHeader {
                previous_block_hash: BlockHeaderHash([0u8; 32]),
                merkle_root_hash: MerkleRootHash([0u8; 32]),
                time: 0,
                difficulty_target: u64::max_value(),
                nonce: 0,
            };

            let random_block = Block {
                header: random_block_header,
                transactions: Transactions::new(),
            };

            assert!(blockchain.insert_and_commit(random_block.clone()).is_err());

            kill_storage(blockchain, path);
        }

        #[test]
        pub fn test_invalid_block_removal() {
            let mut path = std::env::current_dir().unwrap();
            path.push(random_storage_path());

            BlockStorage::destroy_storage(path.clone()).unwrap();

            let blockchain = BlockStorage::open_at_path(path.clone(), GENESIS_BLOCK.into()).unwrap();

            assert!(blockchain.remove_latest_block().is_err());
            assert!(blockchain.remove_latest_blocks(5).is_err());

            kill_storage(blockchain, path);
        }

        #[test]
        pub fn test_invalid_block_retrieval() {
            let mut path = std::env::current_dir().unwrap();
            path.push(random_storage_path());

            BlockStorage::destroy_storage(path.clone()).unwrap();

            let blockchain = BlockStorage::open_at_path(path.clone(), GENESIS_BLOCK.into()).unwrap();

            assert_eq!(
                blockchain.get_latest_block().unwrap(),
                blockchain.get_block_from_block_num(0).unwrap()
            );

            assert!(blockchain.get_block_from_block_num(2).is_err());
            assert!(blockchain.get_block_from_block_num(10).is_err());

            kill_storage(blockchain, path);
        }
    }
}
