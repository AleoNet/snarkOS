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

mod server_listen {
    use snarkos_consensus::{MemoryPool, MerkleTreeLedger};
    use snarkos_dpc::base_dpc::{
        instantiated::{CommitmentMerkleParameters, Components, Tx},
        parameters::PublicParameters,
    };
    use snarkos_network::{
        external::{
            message::Message,
            message_types::{GetPeers, GetSync, Verack},
            protocol::SyncHandler,
            Handshakes,
        },
        internal::context::Context,
        server::Server,
    };
    use snarkos_testing::{consensus::*, dpc::load_verifying_parameters, network::*, storage::*};

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
        let path = storage.storage.db.path().to_owned();
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
        let path = storage.storage.db.path().to_owned();
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
            let (mut bootnode_hand, _, _) = bootnode_handshakes
                .receive_any(1u64, 1u32, server_address, reader)
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
        let path = storage.storage.db.path().to_owned();
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

            // 5. Send handshake response from remote node to local node

            let mut peer_handshakes = Handshakes::new();
            peer_handshakes
                .receive_any(1u64, 1u32, server_address, reader)
                .await
                .unwrap();
        });

        drop(rt);
        kill_storage_async::<Tx, CommitmentMerkleParameters>(path);
    }
}
