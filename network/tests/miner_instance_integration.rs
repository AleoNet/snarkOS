mod miner_instance_integration {
    use snarkos_consensus::test_data::*;
    use snarkos_network::{server::MinerInstance, test_data::*, Message};

    use chrono::Utc;
    use serial_test::serial;
    use snarkos_network::message::types::Block;
    use std::str::FromStr;
    use tokio::{net::TcpListener, runtime, sync::oneshot};
    use wagyu_bitcoin::{BitcoinAddress, Mainnet};

    //    use tokio::sync::RwLock;

    type N = Mainnet;

    #[test]
    #[serial]
    fn spawn_and_mine() {
        let (storage, path) = initialize_test_blockchain();

        let mut rt = runtime::Runtime::new().unwrap();

        rt.block_on(async move {
            let bootnode_address = random_socket_address();
            let server_address = aleo_socket_address();
            let peer_address = random_socket_address();

            // 1. Bind to peer address

            let mut peer_listener = TcpListener::bind(peer_address).await.unwrap();

            let (tx, rx) = oneshot::channel();
            tokio::spawn(async move {
                // 4. Accept server connection

                let channel_peer_side = get_next_channel(&mut peer_listener).await;

                // 7. Peer receives block

                let (name, _bytes) = channel_peer_side.read().await.unwrap();

                assert_eq!(Block::name(), name);
                tx.send(()).unwrap();
            });

            let server = initialize_test_server(server_address, bootnode_address, storage, CONNECTION_FREQUENCY_LONG);

            // 2. Store peer in peers list

            let mut peer_book = server.context.peer_book.write().await;
            peer_book.peers.update(peer_address, Utc::now());
            drop(peer_book);

            // 3. Server connects to peer

            let mut connections = server.context.connections.write().await;
            connections.connect_and_store(peer_address).await.unwrap();
            drop(connections);

            let miner = MinerInstance::new(
                BitcoinAddress::<N>::from_str(TEST_WALLETS[4].address).unwrap(),
                server.consensus.clone(),
                server.storage.clone(),
                server.memory_pool_lock.clone(),
                server.context.clone(),
            );
            drop(server);

            // 5. Miner starts mining

            miner.spawn();

            // 6. Wait for peer to receive block

            rx.await.unwrap();
        });

        // Kill the miner
        drop(rt);
        kill_storage_async(path);
    }
}
