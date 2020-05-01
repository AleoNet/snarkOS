mod server_connection_handler {
    use snarkos_consensus::test_data::*;
    use snarkos_dpc::{
        base_dpc::{instantiated::Components, parameters::PublicParameters},
        test_data::setup_or_load_parameters,
    };
    use snarkos_network::{
        message::{types::GetMemoryPool, Message},
        test_data::*,
        Channel,
    };

    use chrono::{Duration, Utc};
    use rand::thread_rng;
    use serial_test::serial;
    use std::sync::Arc;
    use tokio::{net::TcpListener, runtime::Runtime};

    fn peer_connect(parameters: PublicParameters<Components>) {
        let mut rt = Runtime::new().unwrap();
        let (storage, path) = initialize_test_blockchain();

        rt.block_on(async move {
            let bootnode_address = random_socket_address();
            let server_address = random_socket_address();
            let peer_address = random_socket_address();

            let peer_listener = TcpListener::bind(peer_address).await.unwrap();

            let server = initialize_test_server(
                server_address,
                bootnode_address,
                storage,
                parameters,
                CONNECTION_FREQUENCY_SHORT,
            );
            let context = Arc::clone(&server.context);

            // 1. Add peer to connected in peer_book

            let mut peer_book = context.peer_book.write().await;
            peer_book.update_connected(peer_address, Utc::now());
            drop(peer_book);

            // 2. Start server

            start_test_server(server);

            // 3. Check that peer received server connect

            accept_all_messages(peer_listener);

            // 4. Check that the server did not move the peer

            let peer_book = context.peer_book.read().await;

            assert!(peer_book.connected_contains(&peer_address));
            assert!(!peer_book.gossiped_contains(&peer_address));
            assert!(!peer_book.disconnected_contains(&peer_address));
        });

        drop(rt);
        kill_storage_async(path);
    }

    fn peer_disconnect(parameters: PublicParameters<Components>) {
        let mut rt = Runtime::new().unwrap();
        let (storage, path) = initialize_test_blockchain();

        rt.block_on(async move {
            let bootnode_address = random_socket_address();
            let server_address = random_socket_address();
            let peer_address = random_socket_address();

            let server = initialize_test_server(
                server_address,
                bootnode_address,
                storage,
                parameters,
                CONNECTION_FREQUENCY_SHORT,
            );
            let context = Arc::clone(&server.context);

            // 1. Add peer with old date to connected in peer_book

            let mut peer_book = context.peer_book.write().await;
            peer_book.update_connected(peer_address, Utc::now() - Duration::minutes(1));
            drop(peer_book);

            // 2. Start server

            start_test_server(server);

            // 3. Wait for connection handler loop

            sleep(CONNECTION_FREQUENCY_SHORT_TIMEOUT).await;

            // 4. Check that the server moved peer from peers to disconnected

            let peer_book = context.peer_book.read().await;

            assert!(!peer_book.connected_contains(&peer_address));
            assert!(!peer_book.gossiped_contains(&peer_address));
            assert!(peer_book.disconnected_contains(&peer_address));
        });

        drop(rt);
        kill_storage_async(path);
    }

    fn gossiped_peer_connect(parameters: PublicParameters<Components>) {
        let mut rt = Runtime::new().unwrap();
        let (storage, path) = initialize_test_blockchain();

        rt.block_on(async move {
            let bootnode_address = random_socket_address();
            let server_address = random_socket_address();
            let peer_address = random_socket_address();

            let peer_listener = TcpListener::bind(peer_address).await.unwrap();

            let server = initialize_test_server(
                server_address,
                bootnode_address,
                storage,
                parameters,
                CONNECTION_FREQUENCY_SHORT,
            );
            let context = Arc::clone(&server.context);

            // 1. Add peer to gossiped in peer_book

            let mut peer_book = context.peer_book.write().await;
            peer_book.update_gossiped(peer_address, Utc::now());
            drop(peer_book);

            // 2. Start server

            start_test_server(server);

            // 3. Check that peer received server connect

            accept_all_messages(peer_listener);

            // 4. Check that the server did not move the peer from gossiped

            let peer_book = context.peer_book.read().await;

            assert!(!peer_book.connected_contains(&peer_address));
            assert!(peer_book.gossiped_contains(&peer_address));
            assert!(!peer_book.disconnected_contains(&peer_address));
        });

        drop(rt);
        kill_storage_async(path);
    }

    fn gossiped_peer_disconnect(parameters: PublicParameters<Components>) {
        let mut rt = Runtime::new().unwrap();
        let (storage, path) = initialize_test_blockchain();

        rt.block_on(async move {
            let bootnode_address = random_socket_address();
            let server_address = random_socket_address();
            let peer_address = random_socket_address();

            let server = initialize_test_server(
                server_address,
                bootnode_address,
                storage,
                parameters,
                CONNECTION_FREQUENCY_SHORT,
            );
            let context = Arc::clone(&server.context);

            let mut peer_book = context.peer_book.write().await;

            // 1. Add the maximum number of connected to the server peer book

            for _x in 0..10 {
                peer_book.update_connected(random_socket_address(), Utc::now());
            }

            // 2. Add peer with old date to gossiped in peer_book

            peer_book.update_gossiped(peer_address, Utc::now() - Duration::minutes(1));
            drop(peer_book);

            // 3. Start server

            start_test_server(server);

            // 4. Wait for connection handler loop

            sleep(CONNECTION_FREQUENCY_SHORT_TIMEOUT).await;

            // 5. Check that the server did not move peer from gossiped

            let peer_book = context.peer_book.read().await;

            assert!(!peer_book.connected_contains(&peer_address));
            assert!(peer_book.gossiped_contains(&peer_address));
            assert!(!peer_book.disconnected_contains(&peer_address));
        });

        drop(rt);
        kill_storage_async(path);
    }

    fn sync_node_disconnect(parameters: PublicParameters<Components>) {
        let mut rt = Runtime::new().unwrap();
        let (storage, path) = initialize_test_blockchain();

        rt.block_on(async move {
            let bootnode_address = random_socket_address();
            let server_address = random_socket_address();
            let peer_address = random_socket_address();

            let peer_listener = TcpListener::bind(peer_address).await.unwrap();

            let server = initialize_test_server(
                server_address,
                bootnode_address,
                storage,
                parameters,
                CONNECTION_FREQUENCY_SHORT,
            );
            let context = Arc::clone(&server.context);
            let sync_handler_lock = Arc::clone(&server.sync_handler_lock);

            // 1. Add peer to peers

            let mut peer_book = context.peer_book.write().await;
            peer_book.update_connected(peer_address, Utc::now());
            drop(peer_book);

            // 2. Start peer_listener

            accept_all_messages(peer_listener);

            // 2. Start server

            start_test_server(server);

            // 4. Add sync_handler to disconnected

            peer_book = context.peer_book.write().await;
            peer_book.disconnect_peer(bootnode_address);
            drop(peer_book);

            // 5. Wait for connection handler loop

            sleep(CONNECTION_FREQUENCY_SHORT_TIMEOUT).await;

            // 6. Check that the server set sync_node to peer

            assert_eq!(sync_handler_lock.lock().await.sync_node, peer_address);
        });

        drop(rt);
        kill_storage_async(path);
    }

    fn memory_pool_interval(parameters: PublicParameters<Components>) {
        let mut rt = Runtime::new().unwrap();

        let (storage, path) = initialize_test_blockchain();

        rt.block_on(async move {
            let bootnode_address = random_socket_address();
            let server_address = random_socket_address();

            let mut sync_node_listener = TcpListener::bind(bootnode_address).await.unwrap();

            let server = initialize_test_server(
                server_address,
                bootnode_address,
                storage,
                parameters,
                CONNECTION_FREQUENCY_SHORT,
            );
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

    #[test]
    #[serial]
    fn test_peer_searching() {
        let (_, parameters) = setup_or_load_parameters(true, &mut thread_rng());

        {
            println!("test peer connect");
            peer_connect(parameters.clone());
        }

        {
            println!("test peer disconnect");
            peer_disconnect(parameters.clone());
        }

        {
            println!("test gossiped peer connect");
            gossiped_peer_connect(parameters.clone());
        }

        {
            println!("test gossiped peer disconnect");
            gossiped_peer_disconnect(parameters.clone());
        }

        {
            println!("test sync node disconnect");
            sync_node_disconnect(parameters.clone());
        }

        {
            println!("test memory pool interval");
            memory_pool_interval(parameters);
        }
    }
}
