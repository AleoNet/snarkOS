// Copyright (C) 2019-2020 Aleo Systems Inc.
// This file is part of the snarkOS library.

// The snarkOS library is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// The snarkOS library is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with the snarkOS library. If not, see <https://www.gnu.org/licenses/>.

mod server_connection_handler {
    use snarkos_dpc::base_dpc::instantiated::{CommitmentMerkleParameters, Tx};
    use snarkos_network::external::{message::Message, message_types::GetMemoryPool, Channel};
    use snarkos_testing::{consensus::FIXTURE_VK, dpc::load_verifying_parameters, network::*, storage::*};

    use serial_test::serial;
    use std::sync::Arc;
    use tokio::{net::TcpListener, runtime::Runtime};

    #[test]
    #[serial]
    fn peer_connect() {
        let mut rt = Runtime::new().unwrap();
        let storage = Arc::new(FIXTURE_VK.ledger());
        let path = storage.storage.db.path().to_owned();
        let parameters = load_verifying_parameters();

        rt.block_on(async move {
            let bootnode_address = random_socket_address();
            let local_address = random_socket_address();
            let remote_address = random_socket_address();

            let remote_listener = TcpListener::bind(remote_address).await.unwrap();

            let server = initialize_test_server(
                local_address,
                bootnode_address,
                storage,
                parameters,
                CONNECTION_FREQUENCY_SHORT,
            );
            let context = Arc::clone(&server.environment);

            // 1. Add peer to connected in peer_book
            let mut peer_book = context.peer_book.write().await;
            peer_book.connected_peer(&remote_address);
            drop(peer_book);

            // 2. Start server
            start_test_server(server);

            // 3. Check that peer received server connect
            accept_all_messages(remote_listener);

            // 4. Check that the server did not move the peer
            let peer_book = context.peer_book.read().await;
            assert!(peer_book.is_connected(&remote_address));
            assert!(!peer_book.is_disconnected(&remote_address));
        });

        drop(rt);
        kill_storage_async::<Tx, CommitmentMerkleParameters>(path);
    }

    #[test]
    #[serial]
    fn peer_disconnect() {
        let mut rt = Runtime::new().unwrap();
        let storage = Arc::new(FIXTURE_VK.ledger());
        let path = storage.storage.db.path().to_owned();
        let parameters = load_verifying_parameters();

        rt.block_on(async move {
            let bootnode_address = random_socket_address();
            let local_address = random_socket_address();
            let remote_address = random_socket_address();

            let server = initialize_test_server(
                local_address,
                bootnode_address,
                storage,
                parameters,
                CONNECTION_FREQUENCY_SHORT,
            );
            let context = Arc::clone(&server.environment);

            // 1. Add peer with old date to connected in peer_book
            let mut peer_book = context.peer_book.write().await;
            peer_book.connected_peer(&remote_address);
            drop(peer_book);

            // 2. Start server
            start_test_server(server);

            // 3. Wait for connection handler loop
            sleep(CONNECTION_FREQUENCY_SHORT_TIMEOUT * 5).await;

            // 4. Check that the server moved peer from connected to disconnected
            let peer_book = context.peer_book.read().await;

            assert!(!peer_book.is_connected(&remote_address));
            assert!(peer_book.is_disconnected(&remote_address));
        });

        drop(rt);
        kill_storage_async::<Tx, CommitmentMerkleParameters>(path);
    }

    #[test]
    #[serial]
    fn found_peer() {
        let mut rt = Runtime::new().unwrap();
        let storage = Arc::new(FIXTURE_VK.ledger());
        let path = storage.storage.db.path().to_owned();
        let parameters = load_verifying_parameters();

        rt.block_on(async move {
            let bootnode_address = random_socket_address();
            let local_address = random_socket_address();
            let remote_address = random_socket_address();

            let remote_listener = TcpListener::bind(remote_address).await.unwrap();

            let server = initialize_test_server(
                local_address,
                bootnode_address,
                storage,
                parameters,
                CONNECTION_FREQUENCY_SHORT,
            );
            let context = Arc::clone(&server.environment);

            // 1. Add peer to the disconnected peers in peer_book
            let mut peer_book = context.peer_book.write().await;
            peer_book.found_peer(&remote_address);
            drop(peer_book);

            // 2. Start server
            start_test_server(server);

            // 3. Check that peer received server connect
            accept_all_messages(remote_listener);

            // 4. Check that the server did not move the peer from the disconnected peers
            let peer_book = context.peer_book.read().await;
            assert!(!peer_book.is_connected(&remote_address));
            assert!(peer_book.is_disconnected(&remote_address));
        });

        drop(rt);
        kill_storage_async::<Tx, CommitmentMerkleParameters>(path);
    }

    #[test]
    #[serial]
    fn sync_node_disconnect() {
        let mut rt = Runtime::new().unwrap();
        let storage = Arc::new(FIXTURE_VK.ledger());
        let path = storage.storage.db.path().to_owned();
        let parameters = load_verifying_parameters();

        rt.block_on(async move {
            let bootnode_address = random_socket_address();
            let local_address = random_socket_address();
            let remote_address = random_socket_address();

            let remote_listener = TcpListener::bind(remote_address).await.unwrap();

            let server = initialize_test_server(
                local_address,
                bootnode_address,
                storage,
                parameters,
                CONNECTION_FREQUENCY_SHORT,
            );
            let context = Arc::clone(&server.environment);
            let sync_handler_lock = Arc::clone(&server.sync_handler_lock);

            // 1. Add peer to peers
            let mut peer_book = context.peer_book.write().await;
            peer_book.connected_peer(&remote_address);
            drop(peer_book);

            // 2. Start remote_listener
            accept_all_messages(remote_listener);

            // 2. Start server
            start_test_server(server);

            // 4. Add sync_handler to disconnected
            peer_book = context.peer_book.write().await;
            peer_book.disconnected_peer(&bootnode_address);
            drop(peer_book);

            // 5. Wait for connection handler loop
            sleep(CONNECTION_FREQUENCY_SHORT_TIMEOUT).await;

            // 6. Check that the server set sync_node to peer
            assert_eq!(sync_handler_lock.lock().await.sync_node_address, remote_address);
        });

        drop(rt);
        kill_storage_async::<Tx, CommitmentMerkleParameters>(path);
    }

    #[test]
    #[serial]
    fn memory_pool_interval() {
        let mut rt = Runtime::new().unwrap();
        let storage = Arc::new(FIXTURE_VK.ledger());
        let path = storage.storage.db.path().to_owned();
        let parameters = load_verifying_parameters();

        rt.block_on(async move {
            let bootnode_address = random_socket_address();
            let local_address = random_socket_address();

            let mut sync_node_listener = TcpListener::bind(bootnode_address).await.unwrap();

            let server = initialize_test_server(
                local_address,
                bootnode_address,
                storage,
                parameters,
                CONNECTION_FREQUENCY_SHORT,
            );
            let context = server.environment.clone();

            // 1. Start server
            start_test_server(server);
            sleep(1000).await; // Sleep to give testing server time to spin up on a new thread

            // 2. Add sync handler to connections

            let bootnode_channel = Arc::new(Channel::new_writer(bootnode_address).await.unwrap());

            context.connections.write().await.store_channel(&bootnode_channel);

            let channel_sync_side = accept_channel(&mut sync_node_listener, local_address).await;

            // 3. Wait for memory pool interval
            let (name, bytes) = channel_sync_side.read().await.unwrap();
            assert_eq!(GetMemoryPool::name(), name);
            assert!(GetMemoryPool::deserialize(bytes).is_ok());
        });

        drop(rt);
        kill_storage_async::<Tx, CommitmentMerkleParameters>(path);
    }
}
