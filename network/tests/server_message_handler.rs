mod server_message_handler {
    use snarkos_consensus::miner::Entry;
    use snarkos_dpc::base_dpc::instantiated::{CommitmentMerkleParameters, Tx};
    use snarkos_network::{
        message::{types::*, Channel, Message},
        test_data::*,
        PingState,
    };
    use snarkos_objects::{block::Block as BlockStruct, BlockHeaderHash};
    use snarkos_testing::{consensus::*, dpc::load_verifying_parameters, storage::*};
    use snarkos_utilities::{
        bytes::{FromBytes, ToBytes},
        to_bytes,
    };

    use chrono::{DateTime, Utc};
    use serial_test::serial;
    use std::{collections::HashMap, net::SocketAddr, sync::Arc};
    use tokio::{net::TcpListener, runtime::Runtime, sync::oneshot};

    pub const WAIT_PERIOD: u64 = 1000;

    #[test]
    #[serial]
    fn receive_block_message() {
        let mut rt = Runtime::new().unwrap();
        let storage = Arc::new(FIXTURE_VK.ledger());
        let path = storage.storage.db.path().to_owned();
        let parameters = load_verifying_parameters();

        let storage_ref = storage.clone();

        rt.block_on(async move {
            let bootnode_address = random_socket_address();
            let server_address = random_socket_address();
            let peer_address = random_socket_address();

            let server = initialize_test_server(
                server_address,
                bootnode_address,
                storage,
                parameters,
                CONNECTION_FREQUENCY_LONG,
            );
            let mut server_sender = server.sender.clone();

            // 1. Start peer and server

            simulate_active_node(peer_address).await;
            start_test_server(server);
            sleep(WAIT_PERIOD).await; // Sleep to give testing server time to spin up on a new thread

            // 2. Send Block message to server

            let (tx, rx) = oneshot::channel();
            tokio::spawn(async move {
                server_sender
                    .send((
                        tx,
                        Block::name(),
                        Block::new(BLOCK_1.to_vec()).serialize().unwrap(),
                        Arc::new(Channel::new_write_only(peer_address).await.unwrap()),
                    ))
                    .await
                    .unwrap();
            });
            rx.await.unwrap();

            // 3. Check that server inserted block into storage

            let block = BlockStruct::<Tx>::deserialize(&BLOCK_1.to_vec()).unwrap();

            assert!(storage_ref.block_hash_exists(&block.header.get_hash()));
        });

        drop(rt);
        kill_storage_async::<Tx, CommitmentMerkleParameters>(path);
    }

    #[test]
    #[serial]
    fn receive_get_block() {
        let mut rt = Runtime::new().unwrap();
        let storage = Arc::new(FIXTURE_VK.ledger());
        let path = storage.storage.db.path().to_owned();
        let parameters = load_verifying_parameters();

        let genesis_block = storage.get_block_from_block_num(0).unwrap();

        rt.block_on(async move {
            let bootnode_address = random_socket_address();
            let server_address = random_socket_address();
            let peer_address = random_socket_address();

            let mut peer_listener = TcpListener::bind(peer_address).await.unwrap();

            let server = initialize_test_server(
                server_address,
                bootnode_address,
                storage,
                parameters,
                CONNECTION_FREQUENCY_LONG,
            );
            let mut server_sender = server.sender.clone();

            // 1. Start server

            start_test_server(server);
            sleep(WAIT_PERIOD).await; // Sleep to give testing server time to spin up on a new thread

            // 2. Send BlockRequest to server from peer

            let (tx, rx) = oneshot::channel();
            tokio::spawn(async move {
                server_sender
                    .send((
                        tx,
                        GetBlock::name(),
                        GetBlock::new(BlockHeaderHash::new(GENESIS_BLOCK_HEADER_HASH.to_vec()))
                            .serialize()
                            .unwrap(),
                        Arc::new(Channel::new_write_only(peer_address).await.unwrap()),
                    ))
                    .await
                    .unwrap();
            });
            rx.await.unwrap();

            // 3. Check that server correctly sent SyncBlock message

            let channel = accept_channel(&mut peer_listener, server_address).await;
            let (name, bytes) = channel.read().await.unwrap();
            assert_eq!(SyncBlock::name(), name);

            assert_eq!(
                SyncBlock::new(to_bytes![genesis_block].unwrap()).serialize().unwrap(),
                bytes
            );
        });

        drop(rt);
        kill_storage_async::<Tx, CommitmentMerkleParameters>(path);
    }

    #[test]
    #[serial]
    fn receive_sync_block() {
        let mut rt = Runtime::new().unwrap();
        let storage = Arc::new(FIXTURE_VK.ledger());
        let path = storage.storage.db.path().to_owned();
        let parameters = load_verifying_parameters();

        let storage_ref = Arc::clone(&storage);

        rt.block_on(async move {
            let bootnode_address = random_socket_address();
            let mut bootnode_listener = TcpListener::bind(bootnode_address).await.unwrap();

            let server_address = random_socket_address();
            let server = initialize_test_server(
                server_address,
                bootnode_address,
                storage,
                parameters,
                CONNECTION_FREQUENCY_LONG,
            );
            let mut server_sender = server.sender.clone();

            // 1. Start server

            start_test_server(server);
            sleep(WAIT_PERIOD).await; // Sleep to give testing server time to spin up on a new thread

            let channel_server_side = Arc::new(Channel::new_write_only(bootnode_address).await.unwrap());
            accept_channel(&mut bootnode_listener, server_address).await;

            // 2. Send SyncBlock message to server

            let block_bytes = BLOCK_1.to_vec();
            let block_bytes_ref = block_bytes.clone();
            let (tx, rx) = oneshot::channel();
            tokio::spawn(async move {
                server_sender
                    .send((
                        tx,
                        SyncBlock::name(),
                        SyncBlock::new(block_bytes_ref).serialize().unwrap(),
                        channel_server_side,
                    ))
                    .await
                    .unwrap()
            });
            rx.await.unwrap();

            // 3. Check that server inserted block into storage

            let block = BlockStruct::<Tx>::deserialize(&block_bytes).unwrap();
            assert!(storage_ref.block_hash_exists(&block.header.get_hash()));
        });

        drop(rt);
        kill_storage_async::<Tx, CommitmentMerkleParameters>(path);
    }

    #[test]
    #[serial]
    fn receive_get_sync() {
        let mut rt = Runtime::new().unwrap();
        let storage = Arc::new(FIXTURE_VK.ledger());
        let path = storage.storage.db.path().to_owned();
        let parameters = load_verifying_parameters();

        rt.block_on(async move {
            let bootnode_address = random_socket_address();
            let server_address = random_socket_address();
            let peer_address = random_socket_address();

            let mut peer_listener = TcpListener::bind(peer_address).await.unwrap();

            let server = initialize_test_server(
                server_address,
                bootnode_address,
                storage,
                parameters,
                CONNECTION_FREQUENCY_LONG,
            );
            let mut server_sender_ref_1 = server.sender.clone();
            let mut server_sender_ref_2 = server.sender.clone();

            // 1. Start server

            simulate_active_node(bootnode_address).await;
            start_test_server(server);
            sleep(WAIT_PERIOD).await; // Sleep to give testing server time to spin up on a new thread

            // 2. Send Block 1 to server from bootnode

            let (tx, rx) = oneshot::channel();
            tokio::spawn(async move {
                server_sender_ref_1
                    .send((
                        tx,
                        Block::name(),
                        Block::new(BLOCK_1.to_vec()).serialize().unwrap(),
                        Arc::new(Channel::new_write_only(bootnode_address).await.unwrap()),
                    ))
                    .await
                    .unwrap()
            });
            rx.await.unwrap();

            // 3. Send GetSync to server from peer

            let (tx, rx) = oneshot::channel();
            tokio::spawn(async move {
                server_sender_ref_2
                    .send((
                        tx,
                        GetSync::name(),
                        GetSync::new(vec![BlockHeaderHash::new(GENESIS_BLOCK_HEADER_HASH.to_vec())])
                            .serialize()
                            .unwrap(),
                        Arc::new(Channel::new_write_only(peer_address).await.unwrap()),
                    ))
                    .await
                    .unwrap()
            });
            rx.await.unwrap();

            // 4. Check that server correctly sent Sync message

            let channel = accept_channel(&mut peer_listener, server_address).await;
            let (name, bytes) = channel.read().await.unwrap();

            assert_eq!(Sync::name(), name);
            assert_eq!(
                Sync::new(vec![BlockHeaderHash::new(BLOCK_1_HEADER_HASH.to_vec())])
                    .serialize()
                    .unwrap(),
                bytes
            );
        });

        drop(rt);
        kill_storage_async::<Tx, CommitmentMerkleParameters>(path);
    }

    #[test]
    #[serial]
    fn receive_sync() {
        let mut rt = Runtime::new().unwrap();
        let storage = Arc::new(FIXTURE_VK.ledger());
        let path = storage.storage.db.path().to_owned();
        let parameters = load_verifying_parameters();

        rt.block_on(async move {
            let bootnode_address = random_socket_address();
            let server_address = random_socket_address();
            let peer_address = random_socket_address();

            let mut bootnode_listener = TcpListener::bind(bootnode_address).await.unwrap();

            let server = initialize_test_server(
                server_address,
                bootnode_address,
                storage,
                parameters,
                CONNECTION_FREQUENCY_LONG,
            );
            let mut server_sender = server.sender.clone();
            let context = server.context.clone();
            context
                .connections
                .write()
                .await
                .store_channel(&Arc::new(Channel::new_write_only(bootnode_address).await.unwrap()));

            let block_hash = BlockHeaderHash::new(BLOCK_1_HEADER_HASH.to_vec());
            let block_hash_clone = block_hash.clone();

            // 1. Start server

            simulate_active_node(peer_address).await;
            start_test_server(server);
            sleep(WAIT_PERIOD).await; // Sleep to give testing server time to spin up on a new thread

            // 2. Send Sync message to server from peer

            let (tx, rx) = oneshot::channel();
            tokio::spawn(async move {
                server_sender
                    .send((
                        tx,
                        Sync::name(),
                        Sync::new(vec![block_hash_clone]).serialize().unwrap(),
                        Arc::new(Channel::new_write_only(peer_address).await.unwrap()),
                    ))
                    .await
                    .unwrap();
            });
            rx.await.unwrap();

            // 3. Check that server sent a BlockRequest message to sync node

            let channel = accept_channel(&mut bootnode_listener, server_address).await;
            let (name, bytes) = channel.read().await.unwrap();

            assert_eq!(GetBlock::name(), name);
            assert_eq!(GetBlock::new(block_hash).serialize().unwrap(), bytes);
        });

        drop(rt);
        kill_storage_async::<Tx, CommitmentMerkleParameters>(path);
    }

    #[test]
    #[serial]
    fn receive_transaction() {
        let mut rt = Runtime::new().unwrap();
        let storage = Arc::new(FIXTURE_VK.ledger());
        let path = storage.storage.db.path().to_owned();
        let parameters = load_verifying_parameters();

        rt.block_on(async move {
            let bootnode_address = random_socket_address();
            let server_address = random_socket_address();
            let peer_address = random_socket_address();
            let server = initialize_test_server(
                server_address,
                bootnode_address,
                storage,
                parameters,
                CONNECTION_FREQUENCY_LONG,
            );

            let mut server_sender = server.sender.clone();
            let memory_pool_lock = server.memory_pool_lock.clone();

            // 1. Start server

            simulate_active_node(peer_address).await;
            start_test_server(server);
            sleep(WAIT_PERIOD).await; // Sleep to give testing server time to spin up on a new thread

            // 2. Send Transaction message to server from peer

            let transaction_bytes = TRANSACTION_1.to_vec();
            let transaction_bytes_clone = transaction_bytes.clone();

            let (tx, rx) = oneshot::channel();
            tokio::spawn(async move {
                server_sender
                    .send((
                        tx,
                        Transaction::name(),
                        Transaction::new(transaction_bytes_clone).serialize().unwrap(),
                        Arc::new(Channel::new_write_only(peer_address).await.unwrap()),
                    ))
                    .await
                    .unwrap()
            });
            rx.await.unwrap();

            // 3. Check that server added transaction to memory pool

            let memory_pool = memory_pool_lock.lock().await;
            assert!(memory_pool.contains(&Entry {
                size: transaction_bytes.len(),
                transaction: Tx::read(&transaction_bytes[..]).unwrap(),
            }));
        });

        drop(rt);
        kill_storage_async::<Tx, CommitmentMerkleParameters>(path);
    }

    #[test]
    #[serial]
    fn receive_get_memory_pool_empty() {
        let mut rt = Runtime::new().unwrap();
        let storage = Arc::new(FIXTURE_VK.ledger());
        let path = storage.storage.db.path().to_owned();
        let parameters = load_verifying_parameters();

        rt.block_on(async move {
            let bootnode_address = random_socket_address();
            let server_address = random_socket_address();
            let peer_address = random_socket_address();

            let mut peer_listener = TcpListener::bind(peer_address).await.unwrap();

            let server = initialize_test_server(
                server_address,
                bootnode_address,
                storage,
                parameters,
                CONNECTION_FREQUENCY_LONG,
            );
            let mut server_sender = server.sender.clone();

            // 1. Start server

            start_test_server(server);
            sleep(WAIT_PERIOD).await; // Sleep to give testing server time to spin up on a new thread

            // 2. Send GetMemoryPool to server from peer

            let (tx, rx) = oneshot::channel();
            tokio::spawn(async move {
                server_sender
                    .send((
                        tx,
                        GetMemoryPool::name(),
                        GetMemoryPool.serialize().unwrap(),
                        Arc::new(Channel::new_write_only(peer_address).await.unwrap()),
                    ))
                    .await
                    .unwrap();
            });
            rx.await.unwrap();
            accept_channel(&mut peer_listener, server_address).await;
        });

        drop(rt);
        kill_storage_async::<Tx, CommitmentMerkleParameters>(path);
    }

    #[test]
    #[serial]
    fn receive_get_memory_pool_normal() {
        let mut rt = Runtime::new().unwrap();
        let storage = Arc::new(FIXTURE_VK.ledger());
        let path = storage.storage.db.path().to_owned();
        let parameters = load_verifying_parameters();

        rt.block_on(async move {
            let bootnode_address = random_socket_address();
            let server_address = random_socket_address();
            let peer_address = random_socket_address();

            let mut peer_listener = TcpListener::bind(peer_address).await.unwrap();

            let server = initialize_test_server(
                server_address,
                bootnode_address,
                storage,
                parameters,
                CONNECTION_FREQUENCY_LONG,
            );
            let mut server_sender = server.sender.clone();

            // 1. Insert transaction into server memory pool

            let transaction_bytes = TRANSACTION_1.to_vec();
            let entry = Entry {
                size: transaction_bytes.len(),
                transaction: Tx::read(&transaction_bytes[..]).unwrap(),
            };
            let mut memory_pool = server.memory_pool_lock.lock().await;

            assert!(memory_pool.insert(&server.storage, entry).is_ok());

            drop(memory_pool);

            // 2. Start server

            start_test_server(server);
            sleep(WAIT_PERIOD).await; // Sleep to give testing server time to spin up on a new thread

            // 3. Send GetMemoryPool to server from peer

            let (tx, rx) = oneshot::channel();
            tokio::spawn(async move {
                server_sender
                    .send((
                        tx,
                        GetMemoryPool::name(),
                        GetMemoryPool.serialize().unwrap(),
                        Arc::new(Channel::new_write_only(peer_address).await.unwrap()),
                    ))
                    .await
                    .unwrap()
            });
            rx.await.unwrap();

            // 4. Check that server correctly responded with MemoryPool

            let channel = accept_channel(&mut peer_listener, server_address).await;
            let (name, bytes) = channel.read().await.unwrap();

            assert_eq!(MemoryPool::name(), name);
            assert_eq!(
                MemoryPool::new(vec![TRANSACTION_1.to_vec()]).serialize().unwrap(),
                bytes
            )
        });

        drop(rt);
        kill_storage_async::<Tx, CommitmentMerkleParameters>(path);
    }

    #[test]
    #[serial]
    fn receive_memory_pool() {
        let mut rt = Runtime::new().unwrap();
        let storage = Arc::new(FIXTURE_VK.ledger());
        let path = storage.storage.db.path().to_owned();
        let parameters = load_verifying_parameters();

        rt.block_on(async move {
            let bootnode_address = random_socket_address();
            let server_address = random_socket_address();
            let peer_address = random_socket_address();

            let server = initialize_test_server(
                server_address,
                bootnode_address,
                storage,
                parameters,
                CONNECTION_FREQUENCY_LONG,
            );
            let mut server_sender = server.sender.clone();
            let memory_pool_lock = Arc::clone(&server.memory_pool_lock);

            // 1. Start server

            simulate_active_node(peer_address).await;
            start_test_server(server);

            // 2. Send MemoryPoolResponse to server from peer

            let (tx, rx) = oneshot::channel();
            tokio::spawn(async move {
                server_sender
                    .send((
                        tx,
                        MemoryPool::name(),
                        MemoryPool::new(vec![TRANSACTION_1.to_vec()]).serialize().unwrap(),
                        Arc::new(Channel::new_write_only(peer_address).await.unwrap()),
                    ))
                    .await
                    .unwrap()
            });
            rx.await.unwrap();

            // 3. Check that server correctly added transaction to memory pool

            let transaction_bytes = TRANSACTION_1.to_vec();
            let memory_pool = memory_pool_lock.lock().await;

            assert!(memory_pool.contains(&Entry {
                size: transaction_bytes.len(),
                transaction: Tx::read(&transaction_bytes[..]).unwrap(),
            }));
        });

        drop(rt);
        kill_storage_async::<Tx, CommitmentMerkleParameters>(path);
    }

    #[test]
    #[serial]
    fn receive_get_peers() {
        let mut rt = Runtime::new().unwrap();
        let storage = Arc::new(FIXTURE_VK.ledger());
        let path = storage.storage.db.path().to_owned();
        let parameters = load_verifying_parameters();

        rt.block_on(async move {
            let bootnode_address = random_socket_address();
            let server_address = random_socket_address();
            let peer_address = random_socket_address();

            let mut peer_listener = TcpListener::bind(peer_address).await.unwrap();

            let server = initialize_test_server(
                server_address,
                bootnode_address,
                storage,
                parameters,
                CONNECTION_FREQUENCY_LONG,
            );
            let mut server_sender = server.sender.clone();

            // 1. Start server and bootnode

            start_test_server(server);
            simulate_active_node(bootnode_address).await;
            sleep(WAIT_PERIOD).await; // Sleep to give testing server time to spin up on a new thread

            // 2. Send GetPeers message to server from peer

            let (tx, rx) = oneshot::channel();
            tokio::spawn(async move {
                server_sender
                    .send((
                        tx,
                        GetPeers::name(),
                        GetPeers.serialize().unwrap(),
                        Arc::new(Channel::new_write_only(peer_address).await.unwrap()),
                    ))
                    .await
                    .unwrap();
            });
            rx.await.unwrap();

            // 3. Check that server correctly responded with PeersResponse message

            let channel = accept_channel(&mut peer_listener, server_address).await;
            let (name, bytes) = channel.read().await.unwrap();

            assert_eq!(Peers::name(), name);
            assert_eq!(
                Peers::new(HashMap::<SocketAddr, DateTime<Utc>>::new())
                    .serialize()
                    .unwrap(),
                bytes
            )
        });

        drop(rt);
        kill_storage_async::<Tx, CommitmentMerkleParameters>(path);
    }

    #[test]
    #[serial]
    fn receive_peers() {
        let mut rt = Runtime::new().unwrap();
        let storage = Arc::new(FIXTURE_VK.ledger());
        let path = storage.storage.db.path().to_owned();
        let parameters = load_verifying_parameters();

        rt.block_on(async move {
            let bootnode_address = random_socket_address();
            let server_address = random_socket_address();
            let peer_address = random_socket_address();

            let server = initialize_test_server(
                server_address,
                bootnode_address,
                storage,
                parameters,
                CONNECTION_FREQUENCY_LONG,
            );
            let mut server_sender = server.sender.clone();
            let server_context = Arc::clone(&server.context);

            // 1. Start peer and server

            simulate_active_node(bootnode_address).await;
            start_test_server(server);

            // 2. Send Peers message to server with new peer address form bootnode

            let mut addresses = HashMap::<SocketAddr, DateTime<Utc>>::new();
            addresses.insert(peer_address, Utc::now());

            let (tx, rx) = oneshot::channel();
            tokio::spawn(async move {
                server_sender
                    .send((
                        tx,
                        Peers::name(),
                        Peers::new(addresses).serialize().unwrap(),
                        Arc::new(Channel::new_write_only(bootnode_address).await.unwrap()),
                    ))
                    .await
                    .unwrap()
            });
            rx.await.unwrap();

            // 3. Check that new peer address was added correctly

            assert!(server_context.peer_book.read().await.gossiped_contains(&peer_address));
        });

        drop(rt);
        kill_storage_async::<Tx, CommitmentMerkleParameters>(path);
    }

    #[test]
    #[serial]
    fn receive_ping() {
        let mut rt = Runtime::new().unwrap();
        let storage = Arc::new(FIXTURE_VK.ledger());
        let path = storage.storage.db.path().to_owned();
        let parameters = load_verifying_parameters();

        rt.block_on(async move {
            let bootnode_address = random_socket_address();
            let server_address = random_socket_address();
            let peer_address = random_socket_address();

            let mut peer_listener = TcpListener::bind(peer_address).await.unwrap();

            let server = initialize_test_server(
                server_address,
                bootnode_address,
                storage,
                parameters,
                CONNECTION_FREQUENCY_LONG,
            );
            let mut server_sender = server.sender.clone();

            // 1. Start server

            start_test_server(server);
            sleep(WAIT_PERIOD).await; // Sleep to give testing server time to spin up on a new thread

            // 2. Send ping request to server from peer

            let ping = Ping::new();
            let ping_bytes = ping.serialize().unwrap();
            let (tx, rx) = oneshot::channel();
            tokio::spawn(async move {
                server_sender
                    .send((
                        tx,
                        Ping::name(),
                        ping_bytes,
                        Arc::new(Channel::new_write_only(peer_address).await.unwrap()),
                    ))
                    .await
                    .unwrap()
            });
            rx.await.unwrap();

            // 3. Check that peer received pong

            let channel = accept_channel(&mut peer_listener, server_address).await;
            let (name, bytes) = channel.read().await.unwrap();

            assert_eq!(Pong::name(), name);
            assert_eq!(Pong::new(ping).serialize().unwrap(), bytes);
        });

        drop(rt);
        kill_storage_async::<Tx, CommitmentMerkleParameters>(path);
    }

    #[test]
    #[serial]
    fn receive_pong_unknown() {
        let mut rt = Runtime::new().unwrap();
        let storage = Arc::new(FIXTURE_VK.ledger());
        let path = storage.storage.db.path().to_owned();
        let parameters = load_verifying_parameters();

        rt.block_on(async move {
            let bootnode_address = random_socket_address();
            let server_address = random_socket_address();
            let peer_address = random_socket_address();

            let server = initialize_test_server(
                server_address,
                bootnode_address,
                storage,
                parameters,
                CONNECTION_FREQUENCY_LONG,
            );
            let mut server_sender = server.sender.clone();
            let context = Arc::clone(&server.context);

            // 1. Start peer and server

            simulate_active_node(peer_address).await;
            start_test_server(server);

            // 2. Send pong response to server from peer

            let (tx, rx) = oneshot::channel();
            tokio::spawn(async move {
                server_sender
                    .send((
                        tx,
                        Pong::name(),
                        Pong::new(Ping::new()).serialize().unwrap(),
                        Arc::new(Channel::new_write_only(peer_address).await.unwrap()),
                    ))
                    .await
                    .unwrap()
            });
            rx.await.unwrap();

            // 3. Check that server updated peer

            let peer_book = context.peer_book.read().await;
            assert!(!peer_book.connected_contains(&peer_address));
        });

        drop(rt);
        kill_storage_async::<Tx, CommitmentMerkleParameters>(path);
    }

    #[test]
    #[serial]
    fn receive_pong_rejected() {
        let mut rt = Runtime::new().unwrap();
        let storage = Arc::new(FIXTURE_VK.ledger());
        let path = storage.storage.db.path().to_owned();
        let parameters = load_verifying_parameters();

        rt.block_on(async move {
            let bootnode_address = random_socket_address();
            let server_address = random_socket_address();
            let peer_address = random_socket_address();

            let mut peer_listener = TcpListener::bind(peer_address).await.unwrap();

            let server = initialize_test_server(
                server_address,
                bootnode_address,
                storage,
                parameters,
                CONNECTION_FREQUENCY_LONG,
            );
            let mut server_sender = server.sender.clone();
            let context = Arc::clone(&server.context);

            // 1. Start server

            start_test_server(server);
            sleep(WAIT_PERIOD).await; // Sleep to give testing server time to spin up on a new thread

            let channel_server_side = Arc::new(Channel::new_write_only(peer_address).await.unwrap());
            let channel_peer_side = accept_channel(&mut peer_listener, server_address).await;

            // 2. Add peer to pings

            context.connections.write().await.store_channel(&channel_server_side);

            // 3. Send ping request from server to peer

            context
                .pings
                .write()
                .await
                .send_ping(channel_server_side.clone())
                .await
                .unwrap();

            // 4. Accept server ping request

            let (name, _bytes) = channel_peer_side.read().await.unwrap();
            assert_eq!(Ping::name(), name);

            // 5. Send invalid pong response to server from peer

            let (tx, rx) = oneshot::channel();
            tokio::spawn(async move {
                server_sender
                    .send((
                        tx,
                        Pong::name(),
                        Pong::new(Ping::new()).serialize().unwrap(),
                        channel_server_side,
                    ))
                    .await
                    .unwrap()
            });
            rx.await.unwrap();

            // 6. Check that server did not add peer to peerlist

            let pings = context.pings.read().await;
            let peer_book = context.peer_book.read().await;

            assert_eq!(PingState::Rejected, pings.get_state(peer_address).unwrap());
            assert!(!peer_book.connected_contains(&peer_address));
        });

        drop(rt);
        kill_storage_async::<Tx, CommitmentMerkleParameters>(path);
    }

    #[test]
    #[serial]
    fn receive_pong_accepted() {
        let mut rt = Runtime::new().unwrap();
        let storage = Arc::new(FIXTURE_VK.ledger());
        let path = storage.storage.db.path().to_owned();
        let parameters = load_verifying_parameters();

        rt.block_on(async move {
            let bootnode_address = random_socket_address();
            let server_address = random_socket_address();
            let peer_address = random_socket_address();

            let mut peer_listener = TcpListener::bind(peer_address).await.unwrap();

            let server = initialize_test_server(
                server_address,
                bootnode_address,
                storage,
                parameters,
                CONNECTION_FREQUENCY_LONG,
            );
            let mut server_sender = server.sender.clone();
            let context = Arc::clone(&server.context);

            // 1. Start server

            start_test_server(server);
            sleep(WAIT_PERIOD).await; // Sleep to give testing server time to spin up on a new thread

            let channel_server_side = Arc::new(Channel::new_write_only(peer_address).await.unwrap());
            let channel_peer_side = accept_channel(&mut peer_listener, server_address).await;

            // 2. Add peer to pings

            context.connections.write().await.store_channel(&channel_server_side);

            // 3. Send ping request from server to peer

            context
                .pings
                .write()
                .await
                .send_ping(channel_server_side.clone())
                .await
                .unwrap();

            // 4. Accept server ping request

            let (name, bytes) = channel_peer_side.read().await.unwrap();
            assert_eq!(Ping::name(), name);

            let ping_message = Ping::deserialize(bytes).unwrap();

            // 5. Send pong response to server from peer

            let (tx, rx) = oneshot::channel();
            tokio::spawn(async move {
                server_sender
                    .send((
                        tx,
                        Pong::name(),
                        Pong::new(ping_message).serialize().unwrap(),
                        channel_server_side,
                    ))
                    .await
                    .unwrap()
            });
            rx.await.unwrap();

            // 6. Check that server did not add peer to peerlist

            let pings = context.pings.read().await;
            let peer_book = context.peer_book.read().await;

            assert_eq!(PingState::Accepted, pings.get_state(peer_address).unwrap());
            assert!(peer_book.connected_contains(&peer_address));
        });

        drop(rt);
        kill_storage_async::<Tx, CommitmentMerkleParameters>(path);
    }
}
