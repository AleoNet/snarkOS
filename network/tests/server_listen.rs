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
    use snarkos_network::{
        environment::Environment,
        external::{
            message::Message,
            message_types::{GetPeers, GetSync, Verack},
        },
        Server,
    };
    use snarkos_testing::{consensus::*, dpc::load_verifying_parameters, network::*, storage::*};
    use snarkvm_dpc::base_dpc::{
        instantiated::{CommitmentMerkleParameters, Components, Tx},
        parameters::PublicParameters,
    };

    use chrono::{DateTime, Utc};
    use serial_test::serial;
    use std::{
        collections::HashMap,
        net::SocketAddr,
        sync::{Arc, Mutex, RwLock},
    };
    use tokio::{
        net::TcpListener,
        runtime::Builder,
        sync::{oneshot, oneshot::Sender},
    };
    use tokio_test::assert_err;

    //     #[test]
    //     #[serial]
    //     fn startup_handshake_stored_peers() {
    //         let storage = Arc::new(FIXTURE_VK.ledger());
    //         let path = storage.storage.db.path().to_owned();
    //         let parameters = load_verifying_parameters();

    //         let mut rt = Runtime::new().unwrap();

    //         rt.block_on(async move {
    //             let server_address = random_socket_address();
    //             let peer_address = random_socket_address();

    //             // 1. Start peer
    //             let mut peer_listener = TcpListener::bind(peer_address).await.unwrap();

    //             // 2. Add peer to storage
    //             let mut connected_peers = HashMap::<SocketAddr, DateTime<Utc>>::new();
    //             connected_peers.insert(peer_address, Utc::now());
    //             storage
    //                 .save_peer_book_to_storage(bincode::serialize(&connected_peers).unwrap())
    //                 .unwrap();

    //             // 3. Start server
    //             let (tx, rx) = oneshot::channel();
    //             tokio::spawn(async move {
    //                 start_server(tx, server_address, peer_address, storage, parameters, false).await;
    //                 sleep(5000).await;
    //             });
    //             rx.await.unwrap();

    //             // 4. Check that peer received Version message
    //             let (reader, _peer) = peer_listener.accept().await.unwrap();
    //             sleep(1000).await;

    //             // 5. Send handshake response from remote node to local node
    //             let mut peers = Outbound::new();
    //             peers
    //                 .receive_connection_request(1u64, 1u32, server_address, reader)
    //                 .await
    //                 .unwrap();
    //         });

    //         drop(rt);
    //         kill_storage_async::<Tx, CommitmentMerkleParameters>(path);
    //     }
}
