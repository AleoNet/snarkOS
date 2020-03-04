mod server_connection_handler {
    use snarkos_consensus::test_data::*;
    use snarkos_network::{
        message::{types::GetMemoryPool, Message},
        test_data::*,
        Channel,
    };

    use chrono::{Duration, Utc};
    use serial_test::serial;
    use std::sync::Arc;
    use tokio::{net::TcpListener, runtime::Runtime};

    mod peer_searching {
        use super::*;

        #[test]
        #[serial]
        fn peer_connect() {
            let mut rt = Runtime::new().unwrap();
            let (storage, path) = initialize_test_blockchain();

            rt.block_on(async move {
                let bootnode_address = random_socket_address();
                let server_address = random_socket_address();
                let peer_address = random_socket_address();

                let peer_listener = TcpListener::bind(peer_address).await.unwrap();

                let server =
                    initialize_test_server(server_address, bootnode_address, storage, CONNECTION_FREQUENCY_SHORT);
                let context = Arc::clone(&server.context);

                // 1. Add peer to peers in peer_book

                let mut peer_book = context.peer_book.write().await;
                peer_book.peers.update(peer_address, Utc::now());
                drop(peer_book);

                // 2. Start server

                start_test_server(server);

                // 3. Check that peer received server connect

                accept_all_messages(peer_listener);

                // 4. Check that the server did not move the peer

                let peer_book = context.peer_book.read().await;

                assert!(peer_book.peers.addresses.contains_key(&peer_address));
                assert!(!peer_book.gossiped.addresses.contains_key(&peer_address));
                assert!(!peer_book.disconnected.addresses.contains_key(&peer_address));
            });

            drop(rt);
            kill_storage_async(path);
        }

        #[test]
        #[serial]
        fn peer_disconnect() {
            let mut rt = Runtime::new().unwrap();
            let (storage, path) = initialize_test_blockchain();

            rt.block_on(async move {
                let bootnode_address = random_socket_address();
                let server_address = random_socket_address();
                let peer_address = random_socket_address();

                let server =
                    initialize_test_server(server_address, bootnode_address, storage, CONNECTION_FREQUENCY_SHORT);
                let context = Arc::clone(&server.context);

                // 1. Add peer with old date to peers in peer_book

                let mut peer_book = context.peer_book.write().await;
                peer_book.peers.update(peer_address, Utc::now() - Duration::minutes(1));
                drop(peer_book);

                // 2. Start server

                start_test_server(server);

                // 3. Wait for connection handler loop

                sleep(CONNECTION_FREQUENCY_SHORT_TIMEOUT).await;

                // 4. Check that the server moved peer from peers to disconnected

                let peer_book = context.peer_book.read().await;

                assert!(!peer_book.peers.addresses.contains_key(&peer_address));
                assert!(!peer_book.gossiped.addresses.contains_key(&peer_address));
                assert!(peer_book.disconnected.addresses.contains_key(&peer_address));
            });

            drop(rt);
            kill_storage_async(path);
        }

        #[test]
        #[serial]
        fn gossiped_peer_connect() {
            let mut rt = Runtime::new().unwrap();
            let (storage, path) = initialize_test_blockchain();

            rt.block_on(async move {
                let bootnode_address = random_socket_address();
                let server_address = random_socket_address();
                let peer_address = random_socket_address();

                let peer_listener = TcpListener::bind(peer_address).await.unwrap();

                let server =
                    initialize_test_server(server_address, bootnode_address, storage, CONNECTION_FREQUENCY_SHORT);
                let context = Arc::clone(&server.context);

                // 1. Add peer to gossiped in peer_book

                let mut peer_book = context.peer_book.write().await;
                peer_book.peers.remove(&peer_address);
                peer_book.disconnected.remove(&peer_address);
                peer_book.gossiped.update(peer_address, Utc::now());
                drop(peer_book);

                // 2. Start server

                start_test_server(server);

                // 3. Check that peer received server connect

                accept_all_messages(peer_listener);

                // 4. Check that the server did not move the peer from gossiped

                let peer_book = context.peer_book.read().await;

                assert!(!peer_book.peers.addresses.contains_key(&peer_address));
                assert!(peer_book.gossiped.addresses.contains_key(&peer_address));
                assert!(!peer_book.disconnected.addresses.contains_key(&peer_address));
            });

            drop(rt);
            kill_storage_async(path);
        }

        #[test]
        #[serial]
        fn gossiped_peer_disconnect() {
            let mut rt = Runtime::new().unwrap();
            let (storage, path) = initialize_test_blockchain();

            rt.block_on(async move {
                let bootnode_address = random_socket_address();
                let server_address = random_socket_address();
                let peer_address = random_socket_address();

                let server =
                    initialize_test_server(server_address, bootnode_address, storage, CONNECTION_FREQUENCY_SHORT);
                let context = Arc::clone(&server.context);

                // 1. Add peer with old date to gossiped in peer_book

                let mut peer_book = context.peer_book.write().await;
                peer_book
                    .gossiped
                    .update(peer_address, Utc::now() - Duration::minutes(1));
                drop(peer_book);

                // 2. Start server

                start_test_server(server);

                // 3. Wait for connection handler loop

                sleep(CONNECTION_FREQUENCY_SHORT_TIMEOUT).await;

                // 4. Check that the server did not move peer from gossiped

                let peer_book = context.peer_book.read().await;

                assert!(!peer_book.peers.addresses.contains_key(&peer_address));
                assert!(peer_book.gossiped.addresses.contains_key(&peer_address));
                assert!(!peer_book.disconnected.addresses.contains_key(&peer_address));
            });

            drop(rt);
            kill_storage_async(path);
        }
    }

    #[test]
    #[serial]
    fn sync_node_disconnect() {
        let mut rt = Runtime::new().unwrap();
        let (storage, path) = initialize_test_blockchain();

        rt.block_on(async move {
            let bootnode_address = random_socket_address();
            let server_address = random_socket_address();
            let peer_address = random_socket_address();

            let peer_listener = TcpListener::bind(peer_address).await.unwrap();

            let server = initialize_test_server(server_address, bootnode_address, storage, CONNECTION_FREQUENCY_SHORT);
            let context = Arc::clone(&server.context);
            let sync_handler_lock = Arc::clone(&server.sync_handler_lock);

            // 1. Add peer to peers

            let mut peer_book = context.peer_book.write().await;
            peer_book.peers.update(peer_address, Utc::now());
            drop(peer_book);

            // 2. Start peer_listener

            accept_all_messages(peer_listener);

            // 2. Start server

            start_test_server(server);

            // 4. Add sync_handler to disconnected

            peer_book = context.peer_book.write().await;
            peer_book.disconnected.update(bootnode_address, Utc::now());
            drop(peer_book);

            // 5. Wait for connection handler loop

            sleep(CONNECTION_FREQUENCY_SHORT_TIMEOUT).await;

            // 6. Check that the server set sync_node to peer

            assert_eq!(sync_handler_lock.lock().await.sync_node, peer_address);
        });

        drop(rt);
        kill_storage_async(path);
    }

    #[test]
    #[serial]
    fn memory_pool_interval() {
        let mut rt = Runtime::new().unwrap();

        let (storage, path) = initialize_test_blockchain();

        rt.block_on(async move {
            let bootnode_address = random_socket_address();
            let server_address = random_socket_address();

            let mut sync_node_listener = TcpListener::bind(bootnode_address).await.unwrap();

            let server = initialize_test_server(server_address, bootnode_address, storage, CONNECTION_FREQUENCY_SHORT);
            let context = server.context.clone();

            // 1. Start server

            start_test_server(server);

            // 2. Add sync handler to connections

            context
                .connections
                .write()
                .await
                .store_channel(&Arc::new(Channel::new_write_only(bootnode_address).await.unwrap()));

            let channel_sync_side = accept_channel(&mut sync_node_listener, server_address).await;

            // 3. Wait for memory pool interval

            let (name, bytes) = channel_sync_side.read().await.unwrap();

            assert_eq!(GetMemoryPool::name(), name);
            assert!(GetMemoryPool::deserialize(bytes).is_ok());
        });

        drop(rt);
        kill_storage_async(path);
    }
}
