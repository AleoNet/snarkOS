mod miner_instance_integration {
    use snarkos_consensus::{miner::MemoryPool, test_data::*};
    use snarkos_network::{context::Context, server::MinerInstance, test_data::*};

    use serial_test::serial;
    use std::{str::FromStr, sync::Arc};
    use tokio::{runtime, sync::Mutex};
    use wagyu_bitcoin::{BitcoinAddress, Mainnet};

    //    use tokio::sync::RwLock;

    type N = Mainnet;

    #[test]
    #[serial]
    fn spawn_and_mine() {
        // Initialize the db, lock_1 we pass to the miner, lock_2 we use to check state later
        let (storage, path) = initialize_test_blockchain();
        //        let lock = Arc::new(RwLock::new(0));
        //        let lock_ref = lock.clone();

        // Create a new runtime so we can start, stop, and kill the miner

        let mut rt = runtime::Runtime::new().unwrap();

        rt.block_on(async move {
            //
            //            let lock_ref_2 = lock_ref.clone();
            //
            //
            //            tokio::spawn(async move {
            //                println!("miner thread change value");
            ////                loop { //Todo: find way around locking in a loop
            ////                    println!("acquire lock");
            //                    let mut w = lock_ref.write();
            //                    *w += 1;
            ////                    println!("dropping lock");
            ////                    parking_lot::RwLockWriteGuard::unlock_fair(w);
            ////                    drop(w);
            ////                }
            //            });
            //
            //            println!("test thread waiting for change");
            //
            //            let mut num = 0;
            //            while num == 0 {
            //                println!("test attempt read");
            //                let lock = lock_ref_2.read();
            //                num = *lock;
            //                println!("test found num {:?}", num);
            //                parking_lot::RwLockReadGuard::unlock_fair(lock);
            //            }
            //            println!("test thread done");

            //            let storage_ref = storage.clone();
            //            let (tx, rx) = oneshot::channel();
            //            tokio::spawn(async move {
            //                let mut num = 0;
            //                while num == 0 {
            //                    num = storage_ref.get_latest_block_height();
            //                }
            //                tx.send(()).unwrap();
            //            });
            //
            //            tokio::spawn(async move {
            //                loop {
            //                    let mut w = storage.latest_block_height.write();
            //                    *w +=1;
            //                    drop(w);
            //                }
            //            });
            //            rx.await.unwrap();

            let server_address = aleo_socket_address();
            //            let peer_address = random_socket_address();
            //            let mut peer_listener = TcpListener::bind(peer_address).await.unwrap();

            // 1. Store peer in the peer list of the miner's server_context

            //            let server_context_ref = server_context.clone();
            //            let (tx, rx) = oneshot::channel();
            //            tokio::spawn(async move {
            //                let channel = Channel::connect(peer_address).await.unwrap();
            //
            //                server_context_ref.peer_book.write().await.peers.update(peer_address, Utc::now());
            //                server_context_ref.connections.write().await.store(peer_address, channel);
            //                tx.send(()).unwrap();
            //            });
            //            rx.await.unwrap();
            //            let channel = get_next_channel(&mut peer_listener).await;

            // 2. Start the miner

            let miner = MinerInstance::new(
                BitcoinAddress::<N>::from_str(TEST_WALLETS[4].address).unwrap(),
                TEST_CONSENSUS,
                storage,
                Arc::new(Mutex::new(MemoryPool::new())),
                Arc::new(Context::new(server_address, 5, 0, 10, true, vec![])),
            );

            miner.spawn();

            //            storage.clone().get_latest_block_height();
            //            let mut  block_height = 0;
            //            while block_height == 0 {
            //                block_height = storage.get_latest_block_height();
            //            }
            //            assert_eq!(block_height, 1);
            //            println!("a");

            // 3. Wait for the miner to mine a block and send it to the peer
            //            let (name, bytes) = channel.read().await.unwrap();

            //            assert_eq!(MessageName::from("block"), name);
        });

        //        println!("try to kill runtime");
        //        let lock = lock.write();
        //        parking_lot::RwLockWriteGuard::unlock_fair(lock);

        // Kill the miner
        drop(rt);
        kill_storage_async(path);
    }
}
