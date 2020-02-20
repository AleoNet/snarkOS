//mod sync_integration {
//    use snarkos_consensus::test_data::*;
//    use snarkos_network::{base::Message, increment_sync_handler, test_data::*, SyncHandler};
//    use snarkos_objects::BlockHeaderHash;
//
//    use serial_test::serial;
//    use std::sync::Arc;
//    use tokio::{net::TcpListener, sync::Mutex};
//
//    // TODO handle the transfer of sync handler queue and list and also send the actual block requests
//
//    //    mod handle_peer_block {
//    //        use super::*;
//    //
//    //        #[tokio::test]
//    //        #[serial]
//    //        async fn valid() {
//    //            let (storage, path) = initialize_test_blockchain();
//    //            let storage_ref = Arc::clone(&storage);
//    //
//    //            let memory_pool = MemoryPool::new();
//    //            let memory_pool_lock = Arc::new(Mutex::new(memory_pool));
//    //
//    //            let block_1_serialized = hex::decode(&BLOCK_1).unwrap();
//    //            let block_1 = Block::deserialize(&block_1_serialized).unwrap();
//    //
//    //            let consensus = TEST_CONSENSUS;
//    //            let address_server = random_socket_address();
//    //            let mut listener = TcpListener::bind(address_server).await.unwrap();
//    //
//    //            let propagate = true;
//    //
//    //            handle_peer_block(
//    //                address_server,
//    //                block_1,
//    //                &consensus,
//    //                storage_ref,
//    //                memory_pool_lock.clone(),
//    //                propagate,
//    //            )
//    //            .await;
//    //
//    //            // Check that the valid block was added to the storage
//    //            let new_block_height = storage.latest_block_height();
//    //
//    //            assert_eq!(new_block_height, 1);
//    //
//    //            // Check that a PropagateBlock message was sent to the server
//    //            let message = Message::PropagateBlock {
//    //                block_serialized: block_1_serialized,
//    //            };
//    //            let message_serialized = bincode::serialize(&message).unwrap();
//    //
//    //            let (mut stream, _) = listener.accept().await.unwrap();
//    //            let (mut rd, _) = stream.split();
//    //            let mut wr: Vec<u8> = vec![];
//    //            let n = io::copy(&mut rd, &mut wr).await.unwrap();
//    //
//    //            assert_eq!(n, message_serialized.len() as u64);
//    //            assert_eq!(wr, message_serialized);
//    //
//    //            drop(memory_pool_lock);
//    //            kill_storage_sync(storage, path);
//    //        }
//    //
//    //        #[tokio::test]
//    //        #[serial]
//    //        async fn invalid() {
//    //            let (storage, path) = initialize_test_blockchain();
//    //            let storage_ref = Arc::clone(&storage);
//    //
//    //            let memory_pool = MemoryPool::new();
//    //            let memory_pool_lock = Arc::new(Mutex::new(memory_pool));
//    //
//    //            let block_1 = Block::deserialize(&hex::decode(&BLOCK_2).unwrap()).unwrap();
//    //
//    //            let consensus = TEST_CONSENSUS;
//    //            let server_address = random_socket_address();
//    //
//    //            let propagate = true;
//    //
//    //            handle_peer_block(
//    //                server_address,
//    //                block_1,
//    //                &consensus,
//    //                storage_ref,
//    //                memory_pool_lock.clone(),
//    //                propagate,
//    //            )
//    //            .await;
//    //
//    //            // Check that invalid block was not added to the storage
//    //            let new_block_height = storage.latest_block_height();
//    //
//    //            assert_eq!(new_block_height, 0);
//    //
//    //            drop(memory_pool_lock);
//    //            kill_storage_sync(storage, path);
//    //        }
//    //    }
//    //
//    //    #[tokio::test]
//    //    #[serial]
//    //    async fn test_handle_block_request() {
//    //        let (storage, path) = initialize_test_blockchain();
//    //
//    //        let peer_address = random_socket_address();
//    //
//    //        let mut peer_listener = TcpListener::bind(peer_address).await.unwrap();
//    //
//    //        let block_serialized = hex::decode(BLOCK_1).unwrap();
//    //        let block = Block::deserialize(&block_serialized).unwrap();
//    //        let block_hash = block.header.get_hash();
//    //
//    //        // 1. insert block into storage
//    //
//    //        storage.insert_and_commit(block).unwrap();
//    //
//    //        // 2. handle_block_request to server from peer
//    //
//    //        handle_block_request(peer_address, block_hash, Arc::clone(&storage)).await;
//    //
//    //        // 3. check that peer received SyncBlock message
//    //
//    //        let expected = Message::SyncBlock { block_serialized };
//    //        let actual = get_next_message(&mut peer_listener).await;
//    //
//    //        assert_eq!(actual, expected);
//    //
//    //        kill_storage_sync(storage, path);
//    //    }
//    //
//    //    mod handle_sync_request {
//    //        use super::*;
//    //
//    //        #[tokio::test]
//    //        #[serial]
//    //        async fn shared_block_locator_hashes() {
//    //            let (storage, path) = initialize_test_blockchain();
//    //
//    //            let peer_address = random_socket_address();
//    //
//    //            let mut peer_listener = TcpListener::bind(peer_address).await.unwrap();
//    //
//    //            let block = Block::deserialize(&hex::decode(&BLOCK_1).unwrap()).unwrap();
//    //            let block_hash = block.header.get_hash();
//    //
//    //            let block_locator_hashes = storage.get_block_locator_hashes().unwrap();
//    //            let latest_shared_hash = storage.latest_shared_hash(block_locator_hashes).unwrap();
//    //
//    //            // 1. Insert block into storage
//    //
//    //            storage.insert_and_commit(block).unwrap();
//    //
//    //            // 2. handle_sync_request to server from peer
//    //
//    //            handle_sync_request(peer_address, latest_shared_hash, Arc::clone(&storage)).await;
//    //
//    //            // 3. Check that peer received SyncResponse message
//    //
//    //            let expected = Message::SyncResponse {
//    //                block_hashes: vec![block_hash],
//    //            };
//    //            let actual = get_next_message(&mut peer_listener).await;
//    //
//    //            assert_eq!(actual, expected);
//    //
//    //            kill_storage_sync(storage, path);
//    //        }
//    //    }
//    //
//    //    #[tokio::test]
//    //    #[serial]
//    //    async fn test_handle_sync_response() {
//    //        let (storage, path) = initialize_test_blockchain();
//    //
//    //        let bootnode_address = random_socket_address();
//    //
//    //        let mut bootnode_listener = TcpListener::bind(bootnode_address).await.unwrap();
//    //
//    //        let block = Block::deserialize(&hex::decode(&BLOCK_1).unwrap()).unwrap();
//    //        let block_hash = block.header.get_hash();
//    //
//    //        let sync_handler = SyncHandler::new(bootnode_address);
//    //        let sync_handler_lock = Arc::new(Mutex::new(sync_handler));
//    //
//    //        // 1. handle_sync_response to server from bootnode
//    //
//    //        handle_sync_response(vec![block_hash.clone()], sync_handler_lock, Arc::clone(&storage)).await;
//    //
//    //        // 2. Check that bootnode received BlockRequest message
//    //
//    //        let expected = Message::BlockRequest { block_hash };
//    //        let actual = get_next_message(&mut bootnode_listener).await;
//    //
//    //        assert_eq!(actual, expected);
//    //
//    //        kill_storage_sync(storage, path);
//    //    }
//
//    mod increment_sync_handler {
//        use super::*;
//
//        #[tokio::test]
//        #[serial]
//        async fn request_block() {
//            let (storage, path) = initialize_test_blockchain();
//
//            let bootnode_address = random_socket_address();
//
//            let mut bootnode_listener = TcpListener::bind(bootnode_address).await.unwrap();
//
//            let block_hash = BlockHeaderHash::new(hex::decode(BLOCK_1_HEADER_HASH).unwrap());
//
//            // 1. Push hash to sync handler, set syncing to true
//
//            let mut sync_handler = SyncHandler::new(bootnode_address);
//            sync_handler.block_headers.push(block_hash.clone());
//            sync_handler.update_syncing(1);
//            let sync_handler_lock = Arc::new(Mutex::new(sync_handler));
//
//            // 2. Call increment_sync_handler internally
//
//            increment_sync_handler(sync_handler_lock, Arc::clone(&storage))
//                .await
//                .unwrap();
//
//            // 3. Check that bootnode received BlockRequest message
//
//            let expected = Message::BlockRequest { block_hash };
//            let actual = get_next_message(&mut bootnode_listener).await;
//
//            assert_eq!(actual, expected);
//
//            kill_storage_sync(storage, path);
//        }
//
//        #[tokio::test]
//        #[serial]
//        async fn sync_blocks_request() {
//            let (storage, path) = initialize_test_blockchain();
//
//            let bootnode_address = random_socket_address();
//
//            let mut bootnode_listener = TcpListener::bind(bootnode_address).await.unwrap();
//
//            // 1. Set syncing to true
//
//            let mut sync_handler = SyncHandler::new(bootnode_address);
//            sync_handler.update_syncing(0);
//            let sync_handler_lock = Arc::new(Mutex::new(sync_handler));
//
//            // 2. Call increment_sync_handler_internally
//
//            increment_sync_handler(sync_handler_lock, Arc::clone(&storage))
//                .await
//                .unwrap();
//
//            // 3. Check that bootnode received SyncRequest message
//
//            let expected = Message::SyncRequest {
//                block_locator_hashes: vec![],
//            };
//            let actual = get_next_message(&mut bootnode_listener).await;
//
//            assert_eq!(actual, expected);
//
//            kill_storage_sync(storage, path);
//        }
//    }
//}
