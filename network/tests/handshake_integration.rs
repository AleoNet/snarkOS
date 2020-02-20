mod handshake_integration {
    use serial_test::serial;
    use snarkos_network::{
        base::{handshake_request, handshake_response},
        message::{types::Version, Channel, Message, MessageName},
        test_data::*,
    };
    use std::sync::Arc;
    use tokio::{net::TcpListener, sync::oneshot};

    #[tokio::test]
    #[serial]
    async fn test_handshake_request() {
        let expected_version = 1u64;
        let expected_height = 1u32;
        let server_address = aleo_socket_address();
        let peer_address = random_socket_address();
        let mut peer_listener = TcpListener::bind(peer_address).await.unwrap();

        let (tx, rx) = oneshot::channel();
        tokio::spawn(async move {
            let channel = Arc::new(Channel::connect(peer_address).await.unwrap());

            // 1. Simulate handshake request from server to peer

            handshake_request(channel, expected_version, expected_height, server_address)
                .await
                .unwrap();

            tx.send(()).unwrap();
        });

        rx.await.unwrap();

        // 2. Check that peer received the Version message

        let channel = get_next_channel(&mut peer_listener).await;
        let (name, bytes) = channel.read().await.unwrap();

        assert_eq!(MessageName::from("version"), name);

        let message = Version::deserialize(bytes).unwrap();

        assert_eq!(expected_version, message.version);
        assert_eq!(expected_height, message.height);
        assert_eq!(peer_address, message.address_receiver);
        assert_eq!(server_address, message.address_sender);
    }

    #[tokio::test]
    #[serial]
    async fn test_handshake_response_new_peer() {
        let expected_version = 1u64;
        let expected_height = 1u32;
        let server_address = aleo_socket_address();
        let peer_address = random_socket_address();
        let mut peer_listener = TcpListener::bind(peer_address).await.unwrap();

        let (tx, rx) = oneshot::channel();
        tokio::spawn(async move {
            let channel = Arc::new(Channel::connect(peer_address).await.unwrap());

            // 1. Send response from server to peer

            handshake_response(channel, true, expected_version, expected_height, server_address)
                .await
                .unwrap();

            tx.send(()).unwrap();
        });
        rx.await.unwrap();

        // 2. Check that peer received a Verack message

        let channel = get_next_channel(&mut peer_listener).await;
        let (name, _bytes) = channel.read().await.unwrap();

        assert_eq!(MessageName::from("verack"), name);

        // 3. Check that peer received a Version message

        let (name, bytes) = channel.read().await.unwrap();

        assert_eq!(MessageName::from("version"), name);

        let message = Version::deserialize(bytes).unwrap();

        assert_eq!(expected_version, message.version);
        assert_eq!(expected_height, message.height);
        assert_eq!(peer_address, message.address_receiver);
        assert_eq!(server_address, message.address_sender);
    }

    #[tokio::test]
    #[serial]
    async fn test_handshake_response_old_peer() {
        let expected_version = 1u64;
        let expected_height = 1u32;
        let server_address = aleo_socket_address();
        let peer_address = random_socket_address();
        let mut peer_listener = TcpListener::bind(peer_address).await.unwrap();

        let (tx, rx) = oneshot::channel();
        tokio::spawn(async move {
            let channel = Arc::new(Channel::connect(peer_address).await.unwrap());

            // 1. Send response from server to peer

            handshake_response(channel, false, expected_version, expected_height, server_address)
                .await
                .unwrap();

            tx.send(()).unwrap();
        });
        rx.await.unwrap();

        // 2. Check that peer received a Verack message

        let channel = get_next_channel(&mut peer_listener).await;
        let (name, _bytes) = channel.read().await.unwrap();

        assert_eq!(MessageName::from("verack"), name);

        // 3. Ping peer and make sure no more messages were received

        ping(peer_address, peer_listener).await;
    }
}
