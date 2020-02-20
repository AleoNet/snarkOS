//mod miner_instance_integration {
//    use snarkos_consensus::{miner::MemoryPool, test_data::*};
//    use snarkos_network::{test_data::*, MinerInstance};
//
//    use serial_test::serial;
//    use std::{str::FromStr, sync::Arc};
//    use tokio::{runtime, sync::Mutex};
//    use wagyu_bitcoin::{BitcoinAddress, Mainnet};
//    use snarkos_network::base::Context;
//    use chrono::Utc;
//    use tokio::net::TcpListener;
//
//    type N = Mainnet;
//
//    #[test]
//    #[serial]
//    fn spawn_and_mine() {
//        // Initialize the db, lock_1 we pass to the miner, lock_2 we use to check state later
//        let (storage, path) = initialize_test_blockchain();
////        let storage_ref_2 = &storage_ref_1;
//
//        // Create a new runtime so we can start, stop, and kill the miner
//        let mut rt = runtime::Runtime::new().unwrap();
//
//        // Start the miner in it's own runtime
//        rt.block_on(async move {
//            let peer_address = random_socket_address();
//            let server_context = Arc::new(Context::new(random_socket_address(), 5, 0, 10, true, vec![]));
//            server_context.peer_book.write().await.peers.update(peer_address, Utc::now());
//            server_context.connections.write().await.connect_and_store(peer_address).await.unwrap();
//
////            let peer_listener = TcpListener::bind(peer_address).await.unwrap();
////
////            let miner = MinerInstance::new(
////                BitcoinAddress::<N>::from_str(TEST_WALLETS[4].address).unwrap(),
////                TEST_CONSENSUS,
////                storage,
////                Arc::new(Mutex::new(MemoryPool::new())),
////                server_context,
////            );
////
////            miner.spawn();
//
//
//
//            // Continually check the block height in storage until it increases
//            // This blocks the thread until we are guaranteed to have mined a block
//
////            let mut block_height = 0;
////            while block_height == 0 {
////                block_height = storage.get_latest_block_height();
////            }
//
////            assert!(block_height > 0);
//            println!("test complete");
//        });
//        println!("dropping");
//        // Kill the miner
//        drop(rt);
//        println!("killing");
//        kill_storage_sync(storage, path);
//    }
//}
