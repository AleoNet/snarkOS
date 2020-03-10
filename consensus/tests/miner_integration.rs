mod miner_integration {
    use snarkos_consensus::{
        miner::{MemoryPool, Miner},
        test_data::*,
    };
    use snarkos_objects::{
        Block,
        Transaction,
        TransactionInput,
        TransactionOutput,
        TransactionParameters,
        Transactions,
    };
    use snarkos_storage::{test_data::*, BlockStorage};

    use futures_await_test::async_test;
    use std::{
        str::FromStr,
        sync::{Arc, Mutex},
    };
    use tokio::sync::Mutex;
    use wagyu_bitcoin::{BitcoinAddress, BitcoinPrivateKey, Mainnet};

    type N = Mainnet;

    pub async fn block_1(blockchain: &Arc<BlockStorage>, miner: &Miner) -> Block {
        let genesis_miner_address = BitcoinAddress::<N>::from_str(TEST_WALLETS[0].address).unwrap();
        let recipient_1 = BitcoinAddress::<Mainnet>::from_str(TEST_WALLETS[1].address).unwrap();
        let recipient_2 = BitcoinAddress::<Mainnet>::from_str(TEST_WALLETS[2].address).unwrap();

        let previous_block = &blockchain.get_latest_block().unwrap();
        let tx_to_spend = &previous_block.clone().transactions[0];
        let starting_balance = blockchain.get_balance(&genesis_miner_address);

        let input = TransactionInput::new(
            tx_to_spend.to_transaction_id().unwrap(),
            0,
            Some(genesis_miner_address.clone()),
        )
        .unwrap();
        let outputs = vec![
            TransactionOutput::new(&recipient_1, (starting_balance - STANDARD_TX_FEE) / 2).unwrap(),
            TransactionOutput::new(&recipient_2, (starting_balance - STANDARD_TX_FEE) / 2).unwrap(),
        ];

        let transaction_parameters = TransactionParameters {
            version: 1,
            inputs: vec![input],
            outputs,
        };

        let transaction = Transaction::new(&transaction_parameters).unwrap();
        let signed_transaction = transaction
            .sign(&BitcoinPrivateKey::<N>::from_str(TEST_WALLETS[0].private_key).unwrap())
            .unwrap();

        let (parent_header, transactions) = miner
            .establish_block(&blockchain, &Transactions::from(&[signed_transaction]))
            .await
            .unwrap();
        let header = miner.find_block(&transactions, &parent_header).unwrap();

        Block { header, transactions }
    }

    pub async fn block_2(blockchain: &Arc<BlockStorage>, miner: &Miner) -> Block {
        let recipient_1 = BitcoinAddress::<Mainnet>::from_str(TEST_WALLETS[1].address).unwrap();
        let recipient_2 = BitcoinAddress::<Mainnet>::from_str(TEST_WALLETS[2].address).unwrap();
        let recipient_3 = BitcoinAddress::<Mainnet>::from_str(TEST_WALLETS[3].address).unwrap();

        let previous_block = &blockchain.get_latest_block().unwrap();
        let tx_to_spend = &previous_block.clone().transactions[1];

        let input_1 =
            TransactionInput::new(tx_to_spend.to_transaction_id().unwrap(), 0, Some(recipient_1.clone())).unwrap();
        let output_1 =
            TransactionOutput::new(&recipient_3, blockchain.get_balance(&recipient_1) - STANDARD_TX_FEE).unwrap();

        let transaction_parameters_1 = TransactionParameters {
            version: 1,
            inputs: vec![input_1],
            outputs: vec![output_1],
        };

        let transaction_1 = Transaction::new(&transaction_parameters_1).unwrap();
        let signed_transaction_1 = transaction_1
            .sign(&BitcoinPrivateKey::<N>::from_str(TEST_WALLETS[1].private_key).unwrap())
            .unwrap();

        let input_2 =
            TransactionInput::new(tx_to_spend.to_transaction_id().unwrap(), 1, Some(recipient_2.clone())).unwrap();
        let output_2 =
            TransactionOutput::new(&recipient_3, blockchain.get_balance(&recipient_2) - STANDARD_TX_FEE).unwrap();

        let transaction_parameters_2 = TransactionParameters {
            version: 1,
            inputs: vec![input_2],
            outputs: vec![output_2],
        };

        let transaction_2 = Transaction::new(&transaction_parameters_2).unwrap();
        let signed_transaction_2 = transaction_2
            .sign(&BitcoinPrivateKey::<N>::from_str(TEST_WALLETS[2].private_key).unwrap())
            .unwrap();

        let (parent_header, transactions) = miner
            .establish_block(
                blockchain,
                &Transactions::from(&[signed_transaction_1, signed_transaction_2]),
            )
            .await
            .unwrap();
        let header = miner.find_block(&transactions, &parent_header).unwrap();

        Block { header, transactions }
    }

    #[async_test]
    async fn mine_block() {
        let (blockchain, path) = initialize_test_blockchain();

        let memory_pool = MemoryPool::new();
        let memory_pool_lock = Arc::new(Mutex::new(memory_pool));

        let miner_address = BitcoinAddress::<N>::from_str(TEST_WALLETS[0].address).unwrap();
        let consensus = TEST_CONSENSUS;
        let miner = Miner::new(miner_address.clone(), consensus.clone());

        miner.mine_block(&blockchain, &memory_pool_lock).await.unwrap();

        kill_storage_sync(blockchain, path);
    }

    #[async_test]
    async fn test_block_and_transactions() {
        let (mut blockchain, path) = initialize_test_blockchain();
        let mut memory_pool = MemoryPool::new();

        let consensus = TEST_CONSENSUS;
        let miner_address = BitcoinAddress::<N>::from_str(TEST_WALLETS[4].address).unwrap();
        let miner = Miner::new(miner_address.clone(), consensus.clone());

        // Find first block

        let block_1 = block_1(&blockchain, &miner).await;

        consensus
            .process_block(&mut blockchain, &mut memory_pool, &block_1)
            .unwrap();

        // Check balances after 1st block

        check_block_1_balances(&blockchain);

        // Find second block

        let block_2 = block_2(&blockchain, &miner).await;

        consensus
            .process_block(&mut blockchain, &mut memory_pool, &block_2)
            .unwrap();

        // Check balances after the 2nd block

        check_block_2_balances(&blockchain);

        kill_storage_sync(blockchain, path);
    }

    #[async_test]
    async fn test_invalid_spends() {
        let (mut blockchain, path) = initialize_test_blockchain();
        let mut memory_pool = MemoryPool::new();

        let genesis_miner_address = BitcoinAddress::<N>::from_str(TEST_WALLETS[0].address).unwrap();
        let miner_address = BitcoinAddress::<N>::from_str(TEST_WALLETS[4].address).unwrap();
        let recipient_1 = BitcoinAddress::<Mainnet>::from_str(TEST_WALLETS[1].address).unwrap();
        let recipient_2 = BitcoinAddress::<Mainnet>::from_str(TEST_WALLETS[2].address).unwrap();

        let consensus = TEST_CONSENSUS;
        let miner = Miner::new(miner_address.clone(), consensus);

        let previous_block = &blockchain.get_latest_block().unwrap();
        let tx_to_spend = &previous_block.clone().transactions[0];
        let starting_balance = blockchain.get_balance(&genesis_miner_address);
        let tx_fee = 10000;

        let input = TransactionInput::new(
            tx_to_spend.to_transaction_id().unwrap(),
            0,
            Some(genesis_miner_address.clone()),
        )
        .unwrap();
        let output_1 = vec![TransactionOutput::new(&recipient_1, starting_balance - tx_fee).unwrap()];
        let output_2 = vec![TransactionOutput::new(&recipient_2, starting_balance - tx_fee).unwrap()];

        let transaction_parameters_1 = TransactionParameters {
            version: 1,
            inputs: vec![input.clone()],
            outputs: output_1,
        };

        let transaction_parameters_2 = TransactionParameters {
            version: 1,
            inputs: vec![input.clone()],
            outputs: output_2,
        };

        let transaction_1 = Transaction::new(&transaction_parameters_1).unwrap();
        let transaction_2 = Transaction::new(&transaction_parameters_2).unwrap();
        let signed_transaction_1 = transaction_1
            .sign(&BitcoinPrivateKey::<N>::from_str(TEST_WALLETS[0].private_key).unwrap())
            .unwrap();
        let signed_transaction_2 = transaction_2
            .sign(&BitcoinPrivateKey::<N>::from_str(TEST_WALLETS[0].private_key).unwrap())
            .unwrap();

        assert!(
            miner
                .establish_block(
                    &blockchain,
                    &Transactions::from(&[signed_transaction_1.clone(), signed_transaction_2.clone()])
                )
                .await
                .is_err()
        );

        let (parent_header, transactions) = miner
            .establish_block(&blockchain, &Transactions::from(&[signed_transaction_1.clone()]))
            .await
            .unwrap();
        let header = miner.find_block(&transactions, &parent_header).unwrap();
        let new_block_1 = Block {
            header: header.clone(),
            transactions,
        };
        let invalid_block = Block {
            header,
            transactions: Transactions::from(&[signed_transaction_1.clone(), signed_transaction_2.clone()]),
        };

        assert!(
            miner
                .consensus
                .process_block(&mut blockchain, &mut memory_pool, &invalid_block)
                .is_err()
        );
        miner
            .consensus
            .process_block(&mut blockchain, &mut memory_pool, &new_block_1)
            .unwrap();
        assert!(
            miner
                .establish_block(&blockchain, &Transactions::from(&[signed_transaction_1]))
                .await
                .is_err()
        );
        assert!(
            miner
                .establish_block(&blockchain, &Transactions::from(&[signed_transaction_2]))
                .await
                .is_err()
        );

        kill_storage_sync(blockchain, path);
    }
}
