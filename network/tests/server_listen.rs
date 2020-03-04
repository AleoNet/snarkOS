mod server_listen {
    use snarkos_consensus::{miner::MemoryPool, test_data::*};
    use snarkos_network::{
        context::Context,
        message::{
            types::{GetPeers, GetSync, Verack},
            Message,
        },
        protocol::SyncHandler,
        server::Server,
        test_data::*,
        Handshakes,
    };
    use snarkos_storage::BlockStorage;

    use serial_test::serial;
    use snarkos_network::message::types::Ping;
    use std::{net::SocketAddr, sync::Arc};
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::{TcpListener, TcpStream},
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

            let (reader, _peer) = bootnode_listener.accept().await.unwrap();

            // 4. Send handshake response from bootnode to server

            let mut bootnode_handshakes = Handshakes::new();
            let mut bootnode_hand = bootnode_handshakes
                .receive_any(1u64, 1u32, bootnode_address, server_address, reader)
                .await
                .unwrap();

            // 5. Check that bootnode received a GetPeers message

            let (name, _bytes) = bootnode_hand.channel.read().await.unwrap();

            assert_eq!(GetPeers::name(), name);

            // 6. Check that bootnode received Verack message

            let (name, bytes) = bootnode_hand.channel.read().await.unwrap();

            assert_eq!(Verack::name(), name);
            let verack_message = Verack::deserialize(bytes).unwrap();
            bootnode_hand.accept(verack_message).await.unwrap();

            // 7. Check that bootnode received GetSync message

            let (name, _bytes) = bootnode_hand.channel.read().await.unwrap();
            assert_eq!(GetSync::name(), name);
        });

        drop(rt);
        kill_storage_async(path);
    }
    //
    //
    // #[test]
    // #[serial]
    // fn test_max_peers() {
    //     let (storage, path) = initialize_test_blockchain();
    //
    //     let mut rt = Runtime::new().unwrap();
    //
    //     rt.block_on(async move {
    //         let server_address = random_socket_address();
    //         let bootnode_address = random_socket_address();
    //
    //         // Maximum peers is initialized to 10.
    //         let server = initialize_test_server(server_address, bootnode_address, storage, CONNECTION_FREQUENCY_LONG);
    //
    //         let context = server.context.clone();
    //
    //         // Add 10 connected peers.
    //         let mut peer_book = context.peer_book.write().await;
    //
    //         for _x in 0..10 {
    //             peer_book.add_connected(random_socket_address());
    //         }
    //
    //         assert_eq!(peer_book.connected_total(), context.max_peers);
    //
    //         rt.spawn(async move {
    //             server.listen().await.unwrap()
    //         });
    //
    //         sleep(100).await;
    //
    //         let (tx, rx) = oneshot::channel();
    //         tokio::spawn(async move {
    //             // Should fail
    //             let mut stream = TcpStream::connect(server_address).await.unwrap();
    //             println!("connected");
    //             sleep(2000).await;
    //
    //             stream.read(&mut vec![1u8; 1]).await.unwrap();
    //
    //             // println!("writing");
    //             // assert_err!(stream.write_all(&Ping::new().serialize().unwrap()).await);
    //             // assert_err!();
    //             tx.send(()).unwrap();
    //         });
    //         rx.await.unwrap()
    //     });
    //
    //     drop(rt);
    //     kill_storage_async(path);
    // }
}
