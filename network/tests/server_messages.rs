//mod server_messages {
//    //TODO: run these tests after network refactor
//    use snarkos_consensus::{miner::Entry, test_data::*};
//    use snarkos_network::{message::types, message::Message, test_data::*, AddressBook};
//    use snarkos_objects::{Block, BlockHeaderHash, Transaction};
//
//    use chrono::Utc;
//    use serial_test::serial;
//    use snarkos_network::message::{Channel, MessageHeader};
//    use std::net::SocketAddr;
//    use std::sync::Arc;
//    use tokio::{net::TcpListener, runtime::Runtime};
//    use tokio_test::assert_ok;

//
//    #[test]
//    #[serial]
//    fn block() {
//        let mut rt = Runtime::new().unwrap();
//
//        let (storage, path) = initialize_test_blockchain();
//        let storage_ref = Arc::clone(&storage);
//
//        rt.block_on(async move {
//            let bootnode_address = random_socket_address();
//            let server_address = random_socket_address();
//            let server = initialize_test_server(server_address, bootnode_address, storage, CONNECTION_FREQUENCY_LONG);
//            let mut server_sender = server.sender.clone();
//
//            // 1. Start server
//
//            start_test_server(server);
//
//            // 2. Send Block message to server
//
//            let message = Message::Block {
//                block_serialized: hex::decode(BLOCK_1).unwrap(),
//            };
//
//            assert_ok!(server_sender.send((message, server_address)).await);
//            sleep(100).await;
//
//            // 3. Check that server inserted block into storage
//
//            let block = Block::deserialize(&hex::decode(BLOCK_1).unwrap()).unwrap();
//
//            assert!(storage_ref.is_exist(&block.header.get_hash()));
//        });
//
//        drop(rt);
//        kill_storage_async(path);
//    }
//
//    #[test]
//    #[serial]
//    fn block_request() {
//        let mut rt = Runtime::new().unwrap();
//
//        let (storage, path) = initialize_test_blockchain();
//
//        rt.block_on(async move {
//            let bootnode_address = random_socket_address();
//            let server_address = random_socket_address();
//            let peer_address = random_socket_address();
//
//            let mut peer_listener = TcpListener::bind(peer_address).await.unwrap();
//
//            let server = initialize_test_server(server_address, bootnode_address, storage, CONNECTION_FREQUENCY_LONG);
//            let mut server_sender = server.sender.clone();
//
//            // 1. Start server
//
//            start_test_server(server);
//
//            // 2. Send BlockRequest to server from peer
//
//            let message = Message::BlockRequest {
//                block_hash: BlockHeaderHash::new(hex::decode(GENESIS_BLOCK_HEADER_HASH).unwrap()),
//            };
//
//            assert_ok!(server_sender.send((message, peer_address)).await);
//
//            // 3. Check that server correctly sent SyncBlock message
//
//            let expected = Message::SyncBlock {
//                block_serialized: hex::decode(GENESIS_BLOCK).unwrap(),
//            };
//            let actual = get_next_message(&mut peer_listener).await;
//
//            assert_eq!(actual, expected);
//        });
//
//        drop(rt);
//        kill_storage_async(path);
//    }
//
//    mod memory_pool {
//        use super::*;
//
//        #[test]
//        #[serial]
//        fn memory_pool_request_empty() {
//            let mut rt = Runtime::new().unwrap();
//
//            let (storage, path) = initialize_test_blockchain();
//
//            rt.block_on(async move {
//                let bootnode_address = random_socket_address();
//                let server_address = random_socket_address();
//                let peer_address = random_socket_address();
//
//                let peer_listener = TcpListener::bind(peer_address).await.unwrap();
//
//                let server =
//                    initialize_test_server(server_address, bootnode_address, storage, CONNECTION_FREQUENCY_LONG);
//                let mut server_sender = server.sender.clone();
//
//                // 1. Start server
//
//                start_test_server(server);
//
//                // 2. Send MemoryPoolRequest to server from peer
//
//                let message = Message::MemoryPoolRequest;
//
//                assert_ok!(server_sender.send((message, peer_address)).await);
//
//                // 3. Check that server did not send a response since it has no transactions to send
//
//                ping(peer_address, peer_listener).await;
//            });
//
//            drop(rt);
//            kill_storage_async(path);
//        }
//
//        #[test]
//        #[serial]
//        fn memory_pool_request_normal() {
//            let mut rt = Runtime::new().unwrap();
//
//            let (storage, path) = initialize_test_blockchain();
//
//            rt.block_on(async move {
//                let bootnode_address = random_socket_address();
//                let server_address = random_socket_address();
//                let peer_address = random_socket_address();
//
//                let mut peer_listener = TcpListener::bind(peer_address).await.unwrap();
//
//                let server =
//                    initialize_test_server(server_address, bootnode_address, storage, CONNECTION_FREQUENCY_LONG);
//                let mut server_sender = server.sender.clone();
//
//                // 1. Insert transaction into server memory pool
//
//                let transaction_bytes = hex::decode(TRANSACTION).unwrap();
//                let entry = Entry {
//                    size: transaction_bytes.len(),
//                    transaction: Transaction::deserialize(&transaction_bytes).unwrap(),
//                };
//                let mut memory_pool = server.memory_pool_lock.lock().await;
//
//                assert!(memory_pool.insert(&server.storage, entry).is_ok());
//
//                drop(memory_pool);
//
//                // 2. Start server
//
//                start_test_server(server);
//
//                // 3. Send MemoryPoolRequest to server from peer
//
//                let message = Message::MemoryPoolRequest;
//
//                assert_ok!(server_sender.send((message, peer_address)).await);
//
//                // 4. Check that server correctly responded with MemoryPoolResponse
//
//                let expected = Message::MemoryPoolResponse {
//                    memory_pool_transactions: vec![transaction_bytes],
//                };
//                let actual = get_next_message(&mut peer_listener).await;
//
//                assert_eq!(actual, expected);
//            });
//
//            drop(rt);
//            kill_storage_async(path);
//        }
//
//        #[test]
//        #[serial]
//        fn memory_pool_response() {
//            let mut rt = Runtime::new().unwrap();
//
//            let (storage, path) = initialize_test_blockchain();
//
//            rt.block_on(async move {
//                let bootnode_address = random_socket_address();
//                let server_address = random_socket_address();
//                let peer_address = random_socket_address();
//
//                let server =
//                    initialize_test_server(server_address, bootnode_address, storage, CONNECTION_FREQUENCY_LONG);
//                let mut server_sender = server.sender.clone();
//                let memory_pool_lock = Arc::clone(&server.memory_pool_lock);
//
//                // 1. Start server
//
//                start_test_server(server);
//
//                // 2. Send MemoryPoolResponse to server from peer
//
//                let transaction_bytes = hex::decode(TRANSACTION).unwrap();
//                let message = Message::MemoryPoolResponse {
//                    memory_pool_transactions: vec![transaction_bytes.clone()],
//                };
//
//                assert_ok!(server_sender.send((message, peer_address)).await);
//                sleep(100).await;
//
//                // 3. Check that server correctly added transaction to memory pool
//
//                let memory_pool = memory_pool_lock.lock().await;
//                assert!(memory_pool.contains(&Entry {
//                    size: transaction_bytes.len(),
//                    transaction: Transaction::deserialize(&transaction_bytes).unwrap(),
//                }));
//            });
//
//            drop(rt);
//            kill_storage_async(path);
//        }
//    }
//
//    #[test]
//    #[serial]
//    fn peers_request() {
//        let mut rt = Runtime::new().unwrap();
//
//        let (storage, path) = initialize_test_blockchain();
//
//        rt.block_on(async move {
//            let bootnode_address = random_socket_address();
//            let server_address = random_socket_address();
//            let peer_address = random_socket_address();
//
//            let mut peer_listener = TcpListener::bind(peer_address).await.unwrap();
//
//            let server = initialize_test_server(server_address, bootnode_address, storage, CONNECTION_FREQUENCY_LONG);
//            let mut server_sender = server.sender.clone();
//            // 1. Start server and bootnode
//
//            start_test_server(server);
//            simulate_active_node(bootnode_address).await;
//
//            // 2. Send GetAddresses message to server from peer
//
//            let message = Message::PeersRequest;
//
//            assert_ok!(server_sender.send((message, peer_address)).await);
//
//            // 3. Check that server correctly responded with PeersResponse message
//
//            let addresses = AddressBook::new().addresses;
//            let expected = Message::PeersResponse { addresses };
//            let actual = get_next_message(&mut peer_listener).await;
//
//            assert_eq!(actual, expected);
//        });
//
//        drop(rt);
//        kill_storage_async(path);
//    }
//
//    #[test]
//    #[serial]
//    fn peers_response() {
//        let mut rt = Runtime::new().unwrap();
//
//        let (storage, path) = initialize_test_blockchain();
//
//        rt.block_on(async move {
//            let bootnode_address = random_socket_address();
//            let server_address = random_socket_address();
//            let peer_address = random_socket_address();
//
//            let server = initialize_test_server(server_address, bootnode_address, storage, CONNECTION_FREQUENCY_LONG);
//            let mut server_sender = server.sender.clone();
//            let server_context = Arc::clone(&server.context);
//
//            // 1. Start server
//
//            start_test_server(server);
//
//            // 2. Send Address message to server with new peer address
//
//            let mut addresses = AddressBook::new().addresses;
//            addresses.insert(peer_address, Utc::now());
//            let message = Message::PeersResponse { addresses };
//
//            assert_ok!(server_sender.send((message, peer_address)).await);
//
//            // 3. Check that new peer address was added correctly
//
//            sleep(100).await;
//            assert!(server_context
//                .peer_book
//                .read()
//                .await
//                .gossiped
//                .addresses
//                .contains_key(&peer_address));
//        });
//
//        drop(rt);
//        kill_storage_async(path);
//    }
//
//    #[test]
//    #[serial]
//    fn test_ping() {
//        let mut rt = Runtime::new().unwrap();
//
//        let (storage, path) = initialize_test_blockchain();
//
//        rt.block_on(async move {
//            let bootnode_address = random_socket_address();
//            let server_address = random_socket_address();
//            let peer_address = "127.0.0.1:4130".parse::<SocketAddr>().unwrap();
//
//            let mut peer_listener = TcpListener::bind(peer_address).await.unwrap();
//
//            let server = initialize_test_server(server_address, bootnode_address, storage, CONNECTION_FREQUENCY_LONG);
//            let mut server_sender = server.sender.clone();
//            let context = server.context.clone();
//
//            // 1. Start server
//
//            start_test_server(server);
//
//            // 2. Send ping request to server from peer
//
//            let message = types::Ping::new(1234567890u64);
//            let message_header = MessageHeader::from([112, 105, 110, 103, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4]);
//
//            //            let channel = Channel::connect(server_address).await.unwrap();
//            //            channel.write(&message).await.unwrap();
//            assert_ok!(
//                server_sender
//                    .send((message_header.name, message.serialize().unwrap(), peer_address))
//                    .await
//            );
//            sleep(1000).await;
//            let channel = context.connections.read().await.get(&peer_address).is_some();
//            println!("{:?}", channel);
//            // 3. Check that peer received pong
//            //
//            //            let expected = Message::Pong;
//            //            let actual = get_next_message(&mut peer_listener).await;
//            //
//            //            assert_eq!(actual, expected);
//        });
//
//        drop(rt);
//        kill_storage_async(path);
//    }
//
//    #[test]
//    #[serial]
//    fn test_pong() {
//        let mut rt = Runtime::new().unwrap();
//
//        let (storage, path) = initialize_test_blockchain();
//
//        rt.block_on(async move {
//            let bootnode_address = random_socket_address();
//            let server_address = random_socket_address();
//            let peer_address = random_socket_address();
//
//            let server = initialize_test_server(server_address, bootnode_address, storage, CONNECTION_FREQUENCY_LONG);
//            let mut server_sender = server.sender.clone();
//            let context = Arc::clone(&server.context);
//
//            // 1. Start server
//
//            start_test_server(server);
//
//            // 2. Send pong response to server from peer
//
//            let message = Message::Pong;
//            assert_ok!(server_sender.send((message, peer_address)).await);
//            sleep(100).await;
//
//            // 3. Check that server updated peer
//
//            let peer_book = context.peer_book.read().await;
//            assert!(peer_book.peer_contains(&peer_address));
//        });
//
//        drop(rt);
//        kill_storage_async(path);
//    }
//
//    #[test]
//    #[serial]
//    fn propagate_block() {
//        let mut rt = Runtime::new().unwrap();
//
//        let (storage, path) = initialize_test_blockchain();
//
//        rt.block_on(async move {
//            let bootnode_address = random_socket_address();
//            let server_address = random_socket_address();
//            let peer_address = random_socket_address();
//
//            let mut peer_listener = TcpListener::bind(peer_address).await.unwrap();
//
//            let server = initialize_test_server(server_address, bootnode_address, storage, CONNECTION_FREQUENCY_LONG);
//            let mut server_sender = server.sender.clone();
//
//            // 1. Add peer to server's peerbook
//
//            let mut peer_book = server.context.peer_book.write().await;
//            peer_book.peers.update(peer_address, Utc::now());
//            drop(peer_book);
//
//            // 2. Start server
//
//            start_test_server(server);
//
//            // 3. Send PropagateBlock message to server from server
//
//            let message = Message::PropagateBlock {
//                block_serialized: hex::decode(BLOCK_1).unwrap(),
//            };
//
//            assert_ok!(server_sender.send((message, server_address)).await);
//
//            // 4. Check that server correctly sent Block message to peer
//
//            let expected = Message::Block {
//                block_serialized: hex::decode(BLOCK_1).unwrap(),
//            };
//            let actual = get_next_message(&mut peer_listener).await;
//
//            assert_eq!(actual, expected);
//        });
//
//        drop(rt);
//        kill_storage_async(path);
//    }
//
//    #[test]
//    #[serial]
//    fn reject() {
//        let mut rt = Runtime::new().unwrap();
//
//        let (storage, path) = initialize_test_blockchain();
//
//        rt.block_on(async move {
//            let bootnode_address = random_socket_address();
//            let server_address = random_socket_address();
//            let server = initialize_test_server(server_address, bootnode_address, storage, CONNECTION_FREQUENCY_LONG);
//
//            let mut server_sender = server.sender.clone();
//
//            // 1. Start server
//
//            start_test_server(server);
//
//            // 2. Send Reject message to server
//
//            let message = Message::Reject;
//
//            assert_ok!(server_sender.send((message, server_address)).await);
//        });
//
//        drop(rt);
//        kill_storage_async(path);
//    }
//
//    #[test]
//    #[serial]
//    fn sync_block() {
//        let mut rt = Runtime::new().unwrap();
//
//        let (storage, path) = initialize_test_blockchain();
//        let storage_ref = Arc::clone(&storage);
//
//        rt.block_on(async move {
//            let bootnode_address = random_socket_address();
//            let bootnode_listener = TcpListener::bind(bootnode_address).await.unwrap();
//
//            let server_address = random_socket_address();
//            let server = initialize_test_server(server_address, bootnode_address, storage, CONNECTION_FREQUENCY_LONG);
//            let mut server_sender = server.sender.clone();
//
//            // 1. Start server
//
//            start_test_server(server);
//
//            // 2. Send SyncBlock message to server
//
//            let message = Message::SyncBlock {
//                block_serialized: hex::decode(BLOCK_1).unwrap(),
//            };
//
//            assert_ok!(server_sender.send((message, server_address)).await);
//            sleep(100).await;
//
//            // 3. Check that server inserted block into storage
//
//            let block = Block::deserialize(&hex::decode(BLOCK_1).unwrap()).unwrap();
//            assert!(storage_ref.is_exist(&block.header.get_hash()));
//
//            // 4. Check that bootnode did not receive any messages
//            ping(bootnode_address, bootnode_listener).await;
//        });
//
//        drop(rt);
//        kill_storage_async(path);
//    }
//
//    #[test]
//    #[serial]
//    fn sync_request() {
//        let mut rt = Runtime::new().unwrap();
//
//        let (storage, path) = initialize_test_blockchain();
//
//        rt.block_on(async move {
//            let bootnode_address = random_socket_address();
//            let server_address = random_socket_address();
//            let peer_address = random_socket_address();
//
//            let mut peer_listener = TcpListener::bind(peer_address).await.unwrap();
//
//            let server = initialize_test_server(server_address, bootnode_address, storage, CONNECTION_FREQUENCY_LONG);
//            let mut server_sender = server.sender.clone();
//
//            // 1. Start server
//
//            start_test_server(server);
//
//            // 2. Send Block 1 to server
//
//            let message = Message::Block {
//                block_serialized: hex::decode(BLOCK_1).unwrap(),
//            };
//
//            assert_ok!(server_sender.send((message, server_address)).await);
//
//            // 3. Send Sync request to server from peer
//
//            let message = Message::SyncRequest {
//                block_locator_hashes: vec![BlockHeaderHash::new(hex::decode(GENESIS_BLOCK_HEADER_HASH).unwrap())],
//            };
//
//            assert_ok!(server_sender.send((message, peer_address)).await);
//
//            // 4. Check that server correctly sent SyncResponse message
//
//            let expected = Message::SyncResponse {
//                block_hashes: vec![BlockHeaderHash::new(hex::decode(BLOCK_1_HEADER_HASH).unwrap())],
//            };
//            let actual = get_next_message(&mut peer_listener).await;
//
//            assert_eq!(actual, expected);
//        });
//
//        drop(rt);
//        kill_storage_async(path);
//    }
//
//    #[test]
//    #[serial]
//    fn sync_response() {
//        let mut rt = Runtime::new().unwrap();
//
//        let (storage, path) = initialize_test_blockchain();
//
//        rt.block_on(async move {
//            let bootnode_address = random_socket_address();
//            let mut bootnode_listener = TcpListener::bind(bootnode_address).await.unwrap();
//
//            let server_address = random_socket_address();
//            let server = initialize_test_server(server_address, bootnode_address, storage, CONNECTION_FREQUENCY_LONG);
//            let mut server_sender = server.sender.clone();
//
//            let block_hash = BlockHeaderHash::new(hex::decode(BLOCK_1_HEADER_HASH).unwrap());
//
//            // 1. Start server
//
//            start_test_server(server);
//
//            // 2. Send SyncResponse message to server
//
//            let message = Message::SyncResponse {
//                block_hashes: vec![block_hash.clone()],
//            };
//
//            assert_ok!(server_sender.send((message, server_address)).await);
//
//            // 3. Check that server sent a BlockRequest message to sync node
//            let expected = Message::BlockRequest { block_hash };
//            let actual = get_next_message(&mut bootnode_listener).await;
//
//            assert_eq!(actual, expected);
//        });
//
//        drop(rt);
//        kill_storage_async(path);
//    }
//    #[test]
//    #[serial]
//    fn transaction() {
//        let mut rt = Runtime::new().unwrap();
//
//        let (storage, path) = initialize_test_blockchain();
//
//        rt.block_on(async move {
//            let bootnode_address = random_socket_address();
//            let server_address = random_socket_address();
//            let server = initialize_test_server(server_address, bootnode_address, storage, CONNECTION_FREQUENCY_LONG);
//
//            let mut server_sender = server.sender.clone();
//            let memory_pool_lock = server.memory_pool_lock.clone();
//
//            // 1. Start server
//
//            start_test_server(server);
//
//            // 2. Send Transaction message to server from server
//
//            let transaction_bytes = hex::decode(TRANSACTION).unwrap();
//
//            let message = Message::Transaction {
//                transaction_bytes: transaction_bytes.clone(),
//            };
//
//            assert_ok!(server_sender.send((message, server_address)).await);
//            sleep(100).await;
//
//            // 3. Check that server added transaction to memory pool
//
//            let memory_pool = memory_pool_lock.lock().await;
//            assert!(memory_pool.contains(&Entry {
//                size: transaction_bytes.len(),
//                transaction: Transaction::deserialize(&transaction_bytes).unwrap(),
//            }));
//        });
//
//        drop(rt);
//        kill_storage_async(path);
//    }
//
//    #[test]
//    #[serial]
//    fn version_verack() {
//        let mut rt = Runtime::new().unwrap();
//
//        let (storage, path) = initialize_test_blockchain();
//
//        rt.block_on(async move {
//            let bootnode_address = random_socket_address();
//            let server_address = random_socket_address();
//            let peer_address = random_socket_address();
//
//            let mut peer_listener = TcpListener::bind(peer_address).await.unwrap();
//
//            let server = initialize_test_server(server_address, bootnode_address, storage, CONNECTION_FREQUENCY_LONG);
//            let mut server_sender = server.sender.clone();
//
//            // 1. Start server
//
//            start_test_server(server);
//
//            // 2. Send Version message to server from peer
//
//            let message = Message::Version {
//                version: 1,
//                timestamp: Utc::now(),
//                height: 1,
//                address_receiver: server_address,
//            };
//
//            assert_ok!(server_sender.send((message, peer_address)).await);
//
//            // 3. Check that server correctly responded with a Verack message
//
//            let expected = Message::Verack;
//            let actual = get_next_message(&mut peer_listener).await;
//
//            assert_eq!(actual, expected);
//        });
//
//        drop(rt);
//        kill_storage_async(path);
//    }
//}
