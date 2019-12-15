mod request_all {
    use snarkos_consensus::test_data::*;
    use snarkos_network::{
        base::{
            handshake_request,
            handshake_response,
            send_block,
            send_block_request,
            send_memory_pool_request,
            send_memory_pool_response,
            send_propagate_block,
            send_sync_block,
            send_sync_request,
            send_sync_response,
            send_transaction,
            Message,
        },
        test_data::*,
        AddressBook,
    };
    use snarkos_objects::BlockHeaderHash;

    use serial_test::serial;
    use snarkos_network::base::{send_peers_request, send_peers_response, send_ping, send_pong};
    use tokio::net::TcpListener;

    #[tokio::test]
    #[serial]
    async fn test_block() {
        let peer_address = random_socket_address();

        let mut peer_listener = TcpListener::bind(peer_address).await.unwrap();

        let block_serialized = hex::decode(BLOCK_1).unwrap();

        // 1. Send block to peer

        send_block(peer_address, block_serialized.clone()).await.unwrap();

        // 2. Check that peer received Block message

        let expected = Message::Block { block_serialized };
        let actual = get_next_message(&mut peer_listener).await;

        assert_eq!(actual, expected);
    }

    #[tokio::test]
    #[serial]
    async fn test_block_request() {
        let peer_address = random_socket_address();

        let mut peer_listener = TcpListener::bind(peer_address).await.unwrap();

        let block_hash = BlockHeaderHash::new(hex::decode(GENESIS_BLOCK_HEADER_HASH).unwrap());

        // 1. Request block from peer

        send_block_request(peer_address, block_hash.clone()).await.unwrap();

        // 2. Check that peer received BlockRequest Message
        let expected = Message::BlockRequest { block_hash };
        let actual = get_next_message(&mut peer_listener).await;

        assert_eq!(actual, expected);
    }

    #[tokio::test]
    #[serial]
    async fn test_memory_pool_request() {
        let peer_address = random_socket_address();
        let mut peer_listener = TcpListener::bind(peer_address).await.unwrap();

        // 1. Send memory pool request to peer

        send_memory_pool_request(peer_address).await.unwrap();

        let expected = Message::MemoryPoolRequest;
        let actual = get_next_message(&mut peer_listener).await;

        assert_eq!(actual, expected);
    }

    #[tokio::test]
    #[serial]
    async fn test_memory_pool_response() {
        let peer_address = random_socket_address();
        let mut peer_listener = TcpListener::bind(peer_address).await.unwrap();

        let memory_pool_transactions = vec![hex::decode(TRANSACTION).unwrap()];

        // 1. Send memory pool response to peer

        send_memory_pool_response(peer_address, memory_pool_transactions.clone())
            .await
            .unwrap();

        let expected = Message::MemoryPoolResponse {
            memory_pool_transactions,
        };
        let actual = get_next_message(&mut peer_listener).await;

        assert_eq!(actual, expected);
    }

    #[tokio::test]
    #[serial]
    async fn test_peers_request() {
        let peer_address = random_socket_address();
        let mut peer_listener = TcpListener::bind(peer_address).await.unwrap();

        send_peers_request(peer_address).await.unwrap();

        let expected = Message::PeersRequest;
        let actual = get_next_message(&mut peer_listener).await;

        assert_eq!(actual, expected);
    }

    #[tokio::test]
    #[serial]
    async fn test_peers_response() {
        let peer_address = random_socket_address();
        let mut peer_listener = TcpListener::bind(peer_address).await.unwrap();

        let addresses = AddressBook::new().addresses;

        send_peers_response(peer_address, addresses.clone()).await.unwrap();

        let expected = Message::PeersResponse { addresses };
        let actual = get_next_message(&mut peer_listener).await;

        assert_eq!(actual, expected);
    }

    #[tokio::test]
    #[serial]
    async fn test_ping() {
        let peer_address = random_socket_address();
        let mut peer_listener = TcpListener::bind(peer_address).await.unwrap();

        send_ping(peer_address).await.unwrap();

        let expected = Message::Ping;
        let actual = get_next_message(&mut peer_listener).await;

        assert_eq!(actual, expected);
    }

    #[tokio::test]
    #[serial]
    async fn test_pong() {
        let peer_address = random_socket_address();
        let mut peer_listener = TcpListener::bind(peer_address).await.unwrap();

        send_pong(peer_address).await.unwrap();

        let expected = Message::Pong;
        let actual = get_next_message(&mut peer_listener).await;

        assert_eq!(actual, expected);
    }

    #[tokio::test]
    #[serial]
    async fn test_propagate_block() {
        let server_address = random_socket_address();

        let mut server_listener = TcpListener::bind(server_address).await.unwrap();

        let block_serialized = hex::decode(BLOCK_1).unwrap();

        // 1. Propagate block internally from miner to server

        send_propagate_block(server_address, block_serialized.clone())
            .await
            .unwrap();

        // 2. Check that server received PropagateBlock message

        let expected = Message::PropagateBlock {
            block_serialized: block_serialized.clone(),
        };
        let actual = get_next_message(&mut server_listener).await;

        assert_eq!(actual, expected);
    }

    #[tokio::test]
    #[serial]
    async fn test_sync_block() {
        let peer_address = random_socket_address();

        let mut peer_listener = TcpListener::bind(peer_address).await.unwrap();

        let block_serialized = hex::decode(BLOCK_1).unwrap();

        // 1. Send block to syncing peer

        send_sync_block(peer_address, block_serialized.clone()).await.unwrap();

        // 2. Check that peer received SyncBlock message

        let expected = Message::SyncBlock { block_serialized };
        let actual = get_next_message(&mut peer_listener).await;

        assert_eq!(actual, expected);
    }

    #[tokio::test]
    #[serial]
    async fn test_sync_request() {
        let bootnode_address = random_socket_address();

        let mut bootnode_listener = TcpListener::bind(bootnode_address).await.unwrap();

        let block_locator_hashes = vec![BlockHeaderHash::new(hex::decode(GENESIS_BLOCK_HEADER_HASH).unwrap())];

        // 1. Send sync block request to bootnode

        send_sync_request(bootnode_address, block_locator_hashes.clone())
            .await
            .unwrap();

        let expected = Message::SyncRequest { block_locator_hashes };
        let actual = get_next_message(&mut bootnode_listener).await;

        assert_eq!(actual, expected);
    }

    #[tokio::test]
    #[serial]
    async fn test_sync_response() {
        let peer_address = random_socket_address();
        let mut peer_listener = TcpListener::bind(peer_address).await.unwrap();

        let block_hashes = vec![BlockHeaderHash::new(hex::decode(GENESIS_BLOCK_HEADER_HASH).unwrap())];

        // 1. Send blocks to peer

        send_sync_response(peer_address, block_hashes.clone()).await.unwrap();

        let expected = Message::SyncResponse { block_hashes };
        let actual = get_next_message(&mut peer_listener).await;

        assert_eq!(actual, expected);
    }

    #[tokio::test]
    #[serial]
    async fn test_transaction() {
        let server_address = random_socket_address();

        let mut server_listener = TcpListener::bind(server_address).await.unwrap();

        let transaction_bytes = hex::decode(TRANSACTION).unwrap();

        // 1. Send transaction from client_address to server

        send_transaction(server_address, transaction_bytes.clone())
            .await
            .unwrap();

        // 2. Check that server received Transaction message

        let expected = Message::Transaction {
            transaction_bytes: transaction_bytes.clone(),
        };
        let actual = get_next_message(&mut server_listener).await;

        assert_eq!(actual, expected);
    }

    mod handshake {
        use super::*;

        #[tokio::test]
        #[serial]
        async fn test_handshake_request() {
            let expected_height = 1;

            let peer_address = random_socket_address();
            let mut peer_listener = TcpListener::bind(peer_address).await.unwrap();

            // 1. Initiate handshake request from server to peer

            handshake_request(expected_height, peer_address).await.unwrap();

            // 2. Check that peer received the Version message

            let actual = get_next_message(&mut peer_listener).await;

            if let Message::Version {
                version,
                timestamp: _,
                height,
                address_receiver,
            } = actual
            {
                assert_eq!(version, 1);
                assert_eq!(height, expected_height);
                assert_eq!(address_receiver, peer_address);
            } else {
                panic!();
            }
        }

        #[tokio::test]
        #[serial]
        async fn test_handshake_response_new_peer() {
            let expected_height = 1;
            //        let server_address = random_socket_address();
            let peer_address = random_socket_address();
            let new_peer = true;
            let mut peer_listener = TcpListener::bind(peer_address).await.unwrap();

            // 1. Send response from server to peer

            handshake_response(expected_height, peer_address, new_peer)
                .await
                .unwrap();

            // 2. Check that peer received a Verack message

            let expected_verack = Message::Verack;
            let actual_verack = get_next_message(&mut peer_listener).await;

            assert_eq!(actual_verack, expected_verack);

            // 3. Check that peer received a Version message

            let actual_version = get_next_message(&mut peer_listener).await;

            if let Message::Version {
                version,
                timestamp: _,
                height,
                address_receiver,
            } = actual_version
            {
                assert_eq!(version, 1);
                assert_eq!(height, expected_height);
                assert_eq!(address_receiver, peer_address);
            } else {
                panic!();
            }
        }

        #[tokio::test]
        #[serial]
        async fn test_handshake_response_old_peer() {
            let expected_height = 1;
            //        let server_address = random_socket_address();
            let peer_address = random_socket_address();
            let new_peer = false;
            let mut peer_listener = TcpListener::bind(peer_address).await.unwrap();

            // 1. Send response from server to peer

            handshake_response(expected_height, peer_address, new_peer)
                .await
                .unwrap();

            // 2. Check that peer received a Verack message

            let expected = Message::Verack;
            let actual = get_next_message(&mut peer_listener).await;

            assert_eq!(actual, expected);

            // 3. Ping peer and make sure no more messages were received

            ping(peer_address, peer_listener).await;
        }
    }
}
