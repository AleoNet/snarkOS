mod server_start_miner {
    use snarkos_network::{message::types::Block as BlockMessage, test_data::*, Channel, Message, MAGIC_MAINNET};
    use snarkos_objects::Block;
    use snarkos_storage::test_data::*;

    use chrono::Utc;
    use serial_test::serial;
    use std::{str::FromStr, sync::Arc};
    use tokio::{net::TcpListener, runtime};
    use wagyu_bitcoin::{BitcoinAddress, Mainnet};

    #[test]
    #[serial]
    fn test_mine() {
        let mut rt = runtime::Runtime::new().unwrap();
        let (storage, path) = initialize_test_blockchain();

        rt.block_on(async move {
            let server_address = random_socket_address();
            let peer_address = random_socket_address();

            let mut peer_listener = TcpListener::bind(peer_address).await.unwrap();

            let server = initialize_test_server(server_address, storage, CONNECTION_FREQUENCY_LONG, vec![]);

            let context = server.context.clone();
            let storage = server.storage.clone();

            // 1. Add peer to server peer book

            let mut peer_book = context.peer_book.write().await;
            peer_book.update_connected(peer_address, Utc::now());
            drop(peer_book);

            // 2. Create channel between peer and server

            let channel_server_side = Arc::new(Channel::new_write_only(MAGIC_MAINNET, peer_address).await.unwrap());
            let (read_stream, _socket) = peer_listener.accept().await.unwrap();
            let channel_peer_side = Channel::new_read_only(MAGIC_MAINNET, read_stream).unwrap();

            // 3. Add channel to server connections

            let mut connections = context.connections.write().await;
            connections.store_channel(&channel_server_side);
            drop(connections);

            // 4. Start miner
            let coinbase_address =
                BitcoinAddress::<Mainnet>::from_str("1NpScgYSLW4WcvmZM55EY5cziEiqZx3wJu".into()).unwrap();
            server.start_miner(coinbase_address);

            // 5. Check that peer receives mined block

            let (message, bytes) = channel_peer_side.read().await.unwrap();

            assert_eq!(BlockMessage::name(), message);

            // 6. Check that block is inserted into server storage

            let block_message = BlockMessage::deserialize(bytes).unwrap();
            let expected = Block::deserialize(&block_message.data).unwrap();
            let actual = storage.get_latest_block().unwrap();

            assert_eq!(expected, actual)
        });

        drop(rt);
        kill_storage_async(path);
    }
}
