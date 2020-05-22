mod server_listen {
    use snarkos_consensus::{miner::MemoryPool, test_data::*};
    use snarkos_dpc::{
        base_dpc::{
            instantiated::{CommitmentMerkleParameters, Components, MerkleTreeLedger, Tx},
            parameters::PublicParameters,
        },
        test_data::load_verifying_parameters,
    };
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
    use snarkos_storage::test_data::*;

    use chrono::{DateTime, Utc};
    use serial_test::serial;
    use std::{collections::HashMap, net::SocketAddr, sync::Arc};
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
        storage: Arc<MerkleTreeLedger>,
        parameters: PublicParameters<Components>,
        is_bootnode: bool,
    ) {
        let memory_pool = MemoryPool::new();
        let memory_pool_lock = Arc::new(Mutex::new(memory_pool));

        let consensus = TEST_CONSENSUS.clone();

        let sync_handler = SyncHandler::new(bootnode_address);
        let sync_handler_lock = Arc::new(Mutex::new(sync_handler));

        let server = Server::new(
            Context::new(server_address, 5, 0, 10, is_bootnode, vec![
                bootnode_address.to_string(),
            ]),
            consensus,
            storage,
            parameters,
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
        let storage = Arc::new(FIXTURE_VK.ledger());
        let path = storage.storage.storage.path().to_owned();
        let parameters = load_verifying_parameters();

        // Create a new runtime so we can spawn and block_on threads

        let mut rt = Runtime::new().unwrap();

        rt.block_on(async move {
            let bootnode_address = random_socket_address();
            let server_address = random_socket_address();

            let (tx, rx) = oneshot::channel();

            // 1. Simulate server

            tokio::spawn(async move {
                start_server(tx, server_address, bootnode_address, storage, parameters, true).await;
                sleep(5000).await;
            });
            rx.await.unwrap();

            // 2. Try and bind to server listener port

            sleep(100).await;
            assert_err!(TcpListener::bind(server_address).await);
        });

        drop(rt);
        kill_storage_async::<Tx, CommitmentMerkleParameters>(path);
    }

    #[test]
    #[serial]
    fn startup_handshake_bootnode() {
        let storage = Arc::new(FIXTURE_VK.ledger());
        let path = storage.storage.storage.path().to_owned();
        let parameters = load_verifying_parameters();

        let mut rt = Runtime::new().unwrap();

        rt.block_on(async move {
            let server_address = random_socket_address();
            let bootnode_address = random_socket_address();

            // 1. Start bootnode

            let mut bootnode_listener = TcpListener::bind(bootnode_address).await.unwrap();

            // 2. Start server

            let (tx, rx) = oneshot::channel();

            tokio::spawn(async move {
                start_server(tx, server_address, bootnode_address, storage, parameters, false).await;
                sleep(5000).await;
            });

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
        kill_storage_async::<Tx, CommitmentMerkleParameters>(path);
    }

    #[test]
    #[serial]
    fn startup_handshake_stored_peers() {
        let storage = Arc::new(FIXTURE_VK.ledger());
        let path = storage.storage.storage.path().to_owned();
        let parameters = load_verifying_parameters();

        let mut rt = Runtime::new().unwrap();

        rt.block_on(async move {
            let server_address = random_socket_address();
            let peer_address = random_socket_address();

            // 1. Start peer

            let mut peer_listener = TcpListener::bind(peer_address).await.unwrap();

            // 2. Add peer to storage

            let mut connected_peers = HashMap::<SocketAddr, DateTime<Utc>>::new();
            connected_peers.insert(peer_address, Utc::now());
            storage
                .store_to_peer_book(bincode::serialize(&connected_peers).unwrap())
                .unwrap();

            // 3. Start server

            let (tx, rx) = oneshot::channel();

            tokio::spawn(async move {
                start_server(tx, server_address, peer_address, storage, parameters, false).await;
                sleep(5000).await;
            });

            rx.await.unwrap();

            // 4. Check that peer received Version message

            let (reader, _peer) = peer_listener.accept().await.unwrap();
            sleep(1000).await;

            // 5. Send handshake response from peer to server

            let mut peer_handshakes = Handshakes::new();
            peer_handshakes
                .receive_any(1u64, 1u32, peer_address, server_address, reader)
                .await
                .unwrap();
        });

        drop(rt);
        kill_storage_async::<Tx, CommitmentMerkleParameters>(path);
    }
}
