mod sync_integration {
    use snarkos_consensus::test_data::*;
    use snarkos_network::{
        message::{types::*, Channel, Message},
        protocol::sync::*,
        test_data::*,
    };
    use snarkos_objects::BlockHeaderHash;

    use serial_test::serial;
    use std::sync::Arc;
    use tokio::{net::TcpListener, sync::oneshot};

    mod increment_sync_handler {
        use super::*;

        #[tokio::test]
        #[serial]
        async fn sends_get_block() {
            let (storage, path) = initialize_test_blockchain();

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

            kill_storage_async(path);
        }

        #[tokio::test]
        #[serial]
        async fn sends_get_sync() {
            let (storage, path) = initialize_test_blockchain();

            let bootnode_address = random_socket_address();

            let mut bootnode_listener = TcpListener::bind(bootnode_address).await.unwrap();

            // 1. Set syncing to true

            let mut sync_handler = SyncHandler::new(bootnode_address);
            sync_handler.update_syncing(0);

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

            kill_storage_async(path);
        }
    }
}
