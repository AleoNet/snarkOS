mod miner_instance_integration {
    use snarkos_consensus::{miner::MemoryPool, test_data::*};
    use snarkos_network::{test_data::*, MinerInstance};

    use serial_test::serial;
    use std::{str::FromStr, sync::Arc};
    use tokio::{runtime, sync::Mutex};
    use wagyu_bitcoin::{BitcoinAddress, Mainnet};

    type N = Mainnet;

    #[test]
    #[serial]
    fn spawn_and_mine() {
        // Initialize the db, lock_1 we pass to the miner, lock_2 we use to check state later
        let (storage, path) = initialize_test_blockchain();
        let storage_ref_1 = Arc::clone(&storage);
        let storage_ref_2 = Arc::clone(&storage);

        // Create a new runtime so we can start, stop, and kill the miner
        let mut rt = runtime::Runtime::new().unwrap();

        // Start the miner in it's own runtime
        rt.block_on(async move {
            let server_address = random_socket_address();
            let coinbase_address = BitcoinAddress::<N>::from_str(TEST_WALLETS[4].address).unwrap();
            let consensus = TEST_CONSENSUS;
            let memory_pool = MemoryPool::new();
            let memory_pool_lock = Arc::new(Mutex::new(memory_pool));

            let miner = MinerInstance::new(
                coinbase_address,
                consensus,
                storage_ref_1,
                memory_pool_lock,
                server_address,
            );

            miner.spawn();

            // Continually check the block height in storage until it increases
            // This blocks the thread until we are guaranteed to have mined a block
            let mut block_height = 0;
            while block_height == 0 {
                block_height = storage_ref_2.get_latest_block_height();
            }

            assert!(block_height > 0);
        });

        // Kill the miner
        drop(rt);
        kill_storage_sync(storage, path);
    }
}
