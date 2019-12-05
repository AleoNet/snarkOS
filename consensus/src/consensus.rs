use crate::{bitcoin_retarget, check_block_transactions, miner::MemoryPool};
use snarkos_errors::consensus::ConsensusError;
use snarkos_objects::{merkle_root, Block, BlockHeader, BlockHeaderHash, MerkleRootHash, Transaction, Transactions};
use snarkos_storage::{BlockPath, BlockStorage};

use chrono::Utc;
use wagyu_bitcoin::{BitcoinAddress, Mainnet};

pub const TWO_HOURS_UNIX: i64 = 7200;

/// Parameters for a proof of work blockchain
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConsensusParameters {
    /// maximum block size in bytes
    pub max_block_size: usize,

    /// maximum nonce value allowed
    pub max_nonce: u32,

    /// the amount of time it should take to find a block
    pub target_block_time: i64,

    /// maximum transaction size in bytes
    pub transaction_size: usize,
    //    /// mainnet or testnet
    //    network: Network
}

/// Calculate a block reward that halves every 1000 blocks
pub fn block_reward(block_num: u32) -> u64 {
    100_000_000u64 / (2_u64.pow(block_num / 1000))
}

impl ConsensusParameters {
    /// Calculate the difficulty for the next block
    pub fn get_block_difficulty(&self, prev_header: &BlockHeader, block_timestamp: i64) -> u64 {
        //        let time_elapsed = block_timestamp - prev_header.time;
        //        println!("block_time {:?}   target {:?}", time_elapsed, self.target_block_time);

        //        naive_retarget(block_timestamp, prev_header.time, self.target_block_time, prev_header.difficulty_target)

        bitcoin_retarget(
            block_timestamp,
            prev_header.time,
            self.target_block_time,
            prev_header.difficulty_target,
        )

        //        ethereum_retarget(block_timestamp, prev_header.time, prev_header.difficulty_target)
    }

    /// Run proof of work to find genesis block (parent header is 0)
    pub fn find_genesis(&self, transactions: &Transactions) -> Result<BlockHeader, ConsensusError> {
        let mut merkle_root_bytes = [0u8; 32];
        merkle_root_bytes[..].copy_from_slice(&merkle_root(&transactions.to_transaction_ids()?));

        let time = Utc::now().timestamp();

        let header = BlockHeader {
            merkle_root_hash: MerkleRootHash(merkle_root_bytes),
            previous_block_hash: BlockHeaderHash([0u8; 32]),
            time,
            difficulty_target: 0x0000_7FFF_FFFF_FFFF_u64,
            nonce: 0u32,
        };

        let mut hash_input = header.serialize();

        for nonce in 0..self.max_nonce {
            hash_input[80..84].copy_from_slice(&nonce.to_le_bytes());
            let header = BlockHeader::deserialize(&hash_input);
            let hash_result = header.to_difficulty_hash();

            if hash_result <= header.difficulty_target {
                return Ok(BlockHeader::deserialize(&hash_input));
            }
        }

        Err(ConsensusError::NonceLimitError)
    }

    /// Mine and return genesis block
    pub fn genesis_block(
        &self,
        receiving_address: &BitcoinAddress<Mainnet>,
        storage: &BlockStorage,
    ) -> Result<Block, ConsensusError> {
        // 1. create genesis transaction
        // 2. add transaction to block
        // 3. mine block

        let genesis_transaction = Transaction::create_coinbase_transaction(
            storage.get_latest_block_height() + 1,
            block_reward(storage.get_latest_block_height() + 1),
            0u64,
            receiving_address,
        )?;

        let transactions = Transactions::from(&[genesis_transaction]);
        let genesis_block_header = self.find_genesis(&transactions)?;

        let genesis_block = Block {
            header: genesis_block_header,
            transactions,
        };
        Ok(genesis_block)
    }

    pub fn is_genesis(block: &Block) -> bool {
        block.header.previous_block_hash == BlockHeaderHash([0u8; 32])
    }

    /// Verify all fields in a block header
    /// 1. the parent hash points to the tip of the chain
    /// 2. transactions hash to merkle root
    /// 3. timestamp is less than 2 hours into the future
    /// 4. timestamp is greater than parent timestamp
    /// 5. header is greater than or equal to target difficulty
    /// 6. nonce is within limit
    pub fn verify_header(
        &self,
        header: &BlockHeader,
        parent_header: &BlockHeader,
        merkle_root_hash: &MerkleRootHash,
    ) -> Result<(), ConsensusError> {
        let hash_result = header.to_difficulty_hash();
        let future_timelimit: i64 = Utc::now().timestamp() + TWO_HOURS_UNIX;

        if parent_header.get_hash() != header.previous_block_hash {
            Err(ConsensusError::NoParent(
                parent_header.get_hash().to_string(),
                header.previous_block_hash.to_string(),
            ))
        } else if header.merkle_root_hash != *merkle_root_hash {
            Err(ConsensusError::MerkleRoot(header.merkle_root_hash.to_string()))
        } else if header.time > future_timelimit {
            Err(ConsensusError::FuturisticTimestamp(future_timelimit, header.time))
        } else if header.time < parent_header.time {
            Err(ConsensusError::TimestampInvalid(header.time, parent_header.time))
        } else if hash_result > header.difficulty_target {
            Err(ConsensusError::PowInvalid(header.difficulty_target, hash_result))
        } else if header.nonce >= self.max_nonce {
            Err(ConsensusError::NonceInvalid(header.nonce, self.max_nonce))
        } else {
            Ok(())
        }
    }

    /// Verify that a block's transactions are valid in the context of the current blockchain
    pub fn verify_transactions(
        &self,
        storage: &BlockStorage,
        transactions: &Transactions,
    ) -> Result<(), ConsensusError> {
        check_block_transactions(storage, transactions)?;

        for transaction in transactions.iter() {
            let mut transaction = transaction.clone();
            for input in transaction.parameters.inputs.clone() {
                if !input.outpoint.is_coinbase() && input.outpoint.script_pub_key.is_none() {
                    transaction = transaction.update_outpoint(
                        storage.get_outpoint(&input.outpoint.transaction_id, input.outpoint.index as usize)?,
                    );
                }
            }
            transaction.verify_signatures()?;
        }

        // Check that transactions have sufficient input balance
        if storage.calculate_transaction_fees(transactions).is_ok() {
            Ok(())
        } else {
            Err(ConsensusError::TransactionOverspending)
        }
    }

    pub fn verify_hash(block_header: &BlockHeader, hash_result: u64) -> bool {
        hash_result == block_header.to_difficulty_hash()
    }

    /// Verifies that the block header is valid
    pub fn valid_block_header(&self, block: &Block, storage: &BlockStorage) -> Result<(), ConsensusError> {
        let mut merkle_root_slice = [0u8; 32];
        merkle_root_slice.copy_from_slice(&merkle_root(&block.transactions.to_transaction_ids()?));
        let merkle_root_hash = &MerkleRootHash(merkle_root_slice);

        // Do not verify headers of genesis blocks
        if !Self::is_genesis(block) {
            let parent_block = storage.get_latest_block()?;
            self.verify_header(&block.header, &parent_block.header, merkle_root_hash)?;
        }

        Ok(())
    }

    /// Returns whether or not the given block is valid and insert it
    pub fn process_block(
        &self,
        storage: &BlockStorage,
        memory_pool: &mut MemoryPool,
        block: &Block,
    ) -> Result<u32, ConsensusError> {
        let mut num_canonized = 0;

        // upon retrieval of a new block,
        // 1. verify that the block header is valid
        self.valid_block_header(block, storage)?;

        // 2. verify that the transactions are valid
        self.verify_transactions(storage, &block.transactions)?;

        // 3. Insert/canonize block
        // this check also handles duplicate blocks so we don't need to check
        storage.insert_and_commit(block.clone())?;
        num_canonized += 1;

        info!(
            "new block accepted {:?}.\n Current chain length: {}",
            hex::encode(&block.header.get_hash().0),
            storage.get_latest_block_height()
        );

        for transaction_id in block.transactions.to_transaction_ids()? {
            memory_pool.remove_by_hash(&transaction_id)?;
        }

        // 4. Check cached blocks to insert/canonize
        if let Ok(child_header_hash) = storage.find_child_block(&block.header.get_hash()) {
            // There exists a cached child block that we can add to our chain
            if let Ok(child_block) = storage.get_block(child_header_hash) {
                // process it
                num_canonized += self.process_block(&storage, memory_pool, &child_block)?;
                info!("1 new block accepted from cache");
            }
        }

        Ok(num_canonized)
    }

    /// Receives blocks from an external source
    pub fn receive_block(
        &self,
        storage: &BlockStorage,
        memory_pool: &mut MemoryPool,
        block: &Block,
    ) -> Result<(), ConsensusError> {
        let block_size = block.serialize()?.len();
        if block_size > self.max_block_size {
            return Err(ConsensusError::BlockTooLarge(block_size, self.max_block_size));
        }

        // Block is an unknown orphan
        if !storage.is_previous_block_exist(block) && !storage.is_previous_block_canon(block) {
            if Self::is_genesis(&block) && storage.is_empty() {
                self.process_block(&storage, memory_pool, &block)?;
            } else {
                storage.insert_only(block.clone())?;
            }
        } else {
            // Find the origin of the block
            match storage.get_block_path(&block.header)? {
                BlockPath::ExistingBlock => {}
                BlockPath::CanonChain(_) => {
                    self.process_block(&storage, memory_pool, block)?;
                }
                BlockPath::SideChain(side_chain_path) => {
                    if side_chain_path.new_block_number > storage.get_latest_block_height() {
                        storage.revert_for_fork(&side_chain_path)?;

                        if !side_chain_path.path.is_empty() {
                            let parent_block = storage.get_block(side_chain_path.path[0].clone())?;
                            let num_canonized = self.process_block(&storage, memory_pool, &parent_block)?;
                            assert_eq!(side_chain_path.path.len(), num_canonized as usize);
                        }

                        self.process_block(&storage, memory_pool, block)?;
                        info!("Forked to superior side chain");
                    } else {
                        storage.insert_only(block.clone())?;
                    }
                }
            };
        }

        Ok(())
    }
}
