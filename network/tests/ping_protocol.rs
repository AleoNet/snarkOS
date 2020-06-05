mod ping_protocol {
    use snarkos_network::{
        types::{Ping, Pong},
        Message,
        PingProtocol,
        PingState,
        Pings,
    };
    use snarkos_testing::network::{accept_channel, connect_channel, random_socket_address};

    use serial_test::serial;
    use std::sync::Arc;
    use tokio::net::TcpListener;

    #[tokio::test]
    #[serial]
    async fn test_pings() {
        let server_address = random_socket_address();
        let peer_address = random_socket_address();

        // 1. Bind to server address

        let mut server_listener = TcpListener::bind(server_address).await.unwrap();

        let (tx, rx) = tokio::sync::oneshot::channel();
        tokio::spawn(async move {
            let mut peer_listener = TcpListener::bind(peer_address).await.unwrap();

            // 2. Peer connects to server address

            let channel = Arc::new(connect_channel(&mut peer_listener, server_address).await);

            // 4. Peer sends ping request

            let mut pings = Pings::new();

            pings.send_ping(channel.clone()).await.unwrap();

            assert_eq!(PingState::Waiting, pings.get_state(server_address).unwrap());

            // 7. Peer receives pong response

            let (_name, bytes) = channel.read().await.unwrap();
            let message = Pong::deserialize(bytes).unwrap();

            pings.accept_pong(channel.address, message).await.unwrap();

            assert_eq!(PingState::Accepted, pings.get_state(server_address).unwrap());
            tx.send(()).unwrap();
        });

        // 3. Server accepts peer connection

        let channel = Arc::new(accept_channel(&mut server_listener, peer_address).await);

        // 5. Server receives ping request

        let (_name, bytes) = channel.read().await.unwrap();
        let message = Ping::deserialize(bytes).unwrap();

        // 6. Server sends pong response

        Pings::send_pong(message, channel).await.unwrap();
        rx.await.unwrap();
    }

    #[tokio::test]
    #[serial]
    async fn test_ping_protocol() {
        let server_address = random_socket_address();
        let peer_address = random_socket_address();

        // 1. Bind listener to Server address

        let mut server_listener = TcpListener::bind(server_address).await.unwrap();

        let (tx, rx) = tokio::sync::oneshot::channel();
        tokio::spawn(async move {
            let mut peer_listener = TcpListener::bind(peer_address).await.unwrap();

            // 2. Peer connects to server address

            let channel = Arc::new(connect_channel(&mut peer_listener, server_address).await);

            // 4. Peer send ping request

            let mut peer_ping = PingProtocol::send(channel.clone()).await.unwrap();

            // 5. Peer accepts server pong response

            let (name, bytes) = channel.read().await.unwrap();

            assert_eq!(Pong::name(), name);

            peer_ping.accept(Pong::deserialize(bytes).unwrap()).await.unwrap();

            tx.send(()).unwrap();
        });

        // 3. Server accepts Peer connection

        let channel = Arc::new(accept_channel(&mut server_listener, peer_address).await);

        // 4. Server receives peer ping request. Sends pong response

        let (name, bytes) = channel.read().await.unwrap();

        assert_eq!(Ping::name(), name);

        PingProtocol::receive(Ping::deserialize(bytes).unwrap(), channel)
            .await
            .unwrap();

        rx.await.unwrap();
    }
}
