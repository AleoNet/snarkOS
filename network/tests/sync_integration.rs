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

mod sync_integration {
    use snarkos_dpc::base_dpc::instantiated::{CommitmentMerkleParameters, Tx};
    use snarkos_network::external::{message::Message, message_types::*, protocol::sync::*, Channel};
    use snarkos_objects::BlockHeaderHash;
    use snarkos_testing::{consensus::*, network::*, storage::*};

    use serial_test::serial;
    use std::sync::Arc;
    use tokio::{net::TcpListener, sync::oneshot};

    mod increment_sync_handler {
        use super::*;

        #[tokio::test]
        #[serial]
        async fn sends_get_block() {
            let storage = Arc::new(FIXTURE_VK.ledger());
            let path = storage.storage.db.path().to_owned();
            let bootnode_address = random_socket_address();

            let mut bootnode_listener = TcpListener::bind(bootnode_address).await.unwrap();

            let block_hash = BlockHeaderHash::new(BLOCK_1_HEADER_HASH.to_vec());

            // 1. Push hash to sync handler, set syncing to true

            let mut sync_handler = SyncHandler::new(bootnode_address);
            sync_handler.receive_hashes(vec![block_hash.clone()], 1);

            // 2. Call increment_sync_handler internally

            let (tx, rx) = oneshot::channel();
            tokio::spawn(async move {
                sync_handler
                    .increment(
                        Arc::new(Channel::new_write_only(bootnode_address).await.unwrap()),
                        storage,
                    )
                    .await
                    .unwrap();

                tx.send(()).unwrap();
            });
            rx.await.unwrap();

            // 3. Check that bootnode received GetBlock message

            let channel = accept_channel(&mut bootnode_listener, bootnode_address).await;
            let (name, bytes) = channel.read().await.unwrap();

            assert_eq!(GetBlock::name(), name);
            assert_eq!(GetBlock::new(block_hash).serialize().unwrap(), bytes);

            kill_storage_async::<Tx, CommitmentMerkleParameters>(path);
        }

        #[tokio::test]
        #[serial]
        async fn sends_get_sync() {
            let storage = Arc::new(FIXTURE_VK.ledger());
            let path = storage.storage.db.path().to_owned();

            let bootnode_address = random_socket_address();

            let mut bootnode_listener = TcpListener::bind(bootnode_address).await.unwrap();

            // 1. Set syncing to true

            let mut sync_handler = SyncHandler::new(bootnode_address);
            sync_handler.update_sync_state(0);

            // 2. Call increment_sync_handler_internally
            let (tx, rx) = oneshot::channel();
            tokio::spawn(async move {
                sync_handler
                    .increment(
                        Arc::new(Channel::new_write_only(bootnode_address).await.unwrap()),
                        storage,
                    )
                    .await
                    .unwrap();
                tx.send(()).unwrap();
            });
            rx.await.unwrap();

            // 3. Check that bootnode received GetSync message

            let channel = accept_channel(&mut bootnode_listener, bootnode_address).await;
            let (name, bytes) = channel.read().await.unwrap();

            assert_eq!(GetSync::name(), name);
            assert_eq!(GetSync::new(vec![]).serialize().unwrap(), bytes);

            kill_storage_async::<Tx, CommitmentMerkleParameters>(path);
        }
    }
}
