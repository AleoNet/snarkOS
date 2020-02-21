mod server_listen {
    use snarkos_consensus::{miner::MemoryPool, test_data::*};
    use snarkos_network::{
        base::{handshake_response, Context, SyncHandler},
        message::{
            types::{GetPeers, Verack, Version},
            Message,
            MessageName,
        },
        test_data::*,
        Server,
    };
    use snarkos_storage::BlockStorage;

    use serial_test::serial;
    use std::{net::SocketAddr, sync::Arc};
    use tokio::{
        net::TcpListener,
        runtime::Runtime,
        sync::{oneshot, oneshot::Sender, Mutex},
    };
    use tokio_test::assert_err;

    async fn start_server(
        tx: Sender<()>,
        server_address: SocketAddr,
        bootnode_address: SocketAddr,
        storage: Arc<BlockStorage>,
        is_bootnode: bool,
    ) {
        let memory_pool = MemoryPool::new();
        let memory_pool_lock = Arc::new(Mutex::new(memory_pool));

        let consensus = TEST_CONSENSUS;

        let sync_handler = SyncHandler::new(bootnode_address);
        let sync_handler_lock = Arc::new(Mutex::new(sync_handler));

        let server = Server::new(
            Context::new(server_address, 5, 0, 10, is_bootnode, vec![
                bootnode_address.to_string(),
            ]),
            consensus,
            storage,
            memory_pool_lock,
            sync_handler_lock,
            10000,
        );

        tx.send(()).unwrap();

        server.listen().await.unwrap();
    }

    #[test]
    #[serial]
    fn bind_to_port() {
        let (storage, path) = initialize_test_blockchain();

        // Create a new runtime so we can spawn and block_on threads

        let mut rt = Runtime::new().unwrap();

        rt.block_on(async move {
            let bootnode_address = random_socket_address();
            let server_address = random_socket_address();

            let (tx, rx) = oneshot::channel();

            // 1. Simulate server

            tokio::spawn(async move {
                start_server(tx, server_address, bootnode_address, storage, true).await;
            });
            rx.await.unwrap();

            // 2. Try and bind to server listener port

            sleep(100).await;
            assert_err!(TcpListener::bind(server_address).await);
        });

        drop(rt);
        kill_storage_async(path);
    }

    #[test]
    #[serial]
    fn startup_handshake_bootnode() {
        let (storage, path) = initialize_test_blockchain();

        let mut rt = Runtime::new().unwrap();

        rt.block_on(async move {
            let server_address = random_socket_address();
            let bootnode_address = random_socket_address();

            // 1. Start bootnode

            let mut bootnode_listener = TcpListener::bind(bootnode_address).await.unwrap();

            // 2. Start server

            let (tx, rx) = oneshot::channel();

            tokio::spawn(async move { start_server(tx, server_address, bootnode_address, storage, false).await });

            rx.await.unwrap();

            // 3. Check that bootnode received Version message

            let channel = get_next_channel(&mut bootnode_listener).await;
            let (name, bytes) = channel.read().await.unwrap();

            assert_eq!(MessageName::from("version"), name);
            assert!(Version::deserialize(bytes).is_ok());

            // 4. Check that bootnode received GetPeers message

            let (name, bytes) = channel.read().await.unwrap();

            assert_eq!(MessageName::from("getpeers"), name);
            assert!(GetPeers::deserialize(bytes).is_ok());

            // 5. Send handshake response from bootnode to server

            let channel = Arc::new(channel.update_address(server_address).unwrap());
            handshake_response(channel.clone(), true, 1u64, 0u32, bootnode_address)
                .await
                .unwrap();

            // 6. Check that bootnode received Verack message

            let (name, bytes) = channel.read().await.unwrap();

            assert_eq!(MessageName::from("verack"), name);
            assert!(Verack::deserialize(bytes).is_ok());
        });

        drop(rt);
        kill_storage_async(path);
    }
}
