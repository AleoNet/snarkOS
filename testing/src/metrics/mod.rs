// Copyright (C) 2019-2021 Aleo Systems Inc.
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

use crate::{
    network::{handshaken_peer, test_node, FakeNode, TestSetup},
    wait_until,
};
use snarkos_metrics::stats::NODE_STATS;
use snarkos_network::Version;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

#[tokio::test]
async fn connect_and_disconnect_responder_side() {
    let setup: TestSetup = Default::default();
    let node = test_node(setup).await;
    node.initialize_metrics().await.unwrap();

    // The fake node connects to the node's listener...
    let peer = handshaken_peer(node.expect_local_addr()).await;

    // Needed to make sure the values have been updated.
    wait_until!(5, node.peer_book.get_connected_peer_count() == 1);

    // ...the metrics should reflect this.
    let metrics = NODE_STATS.snapshot();

    assert_eq!(metrics.connections.all_accepted, 1);
    assert_eq!(metrics.connections.all_initiated, 0);
    assert_eq!(metrics.handshakes.successes_resp, 1);
    assert_eq!(metrics.handshakes.successes_init, 0);

    assert_eq!(metrics.connections.connected_peers, 1);
    assert_eq!(metrics.connections.connecting_peers, 0);
    assert_eq!(metrics.connections.disconnected_peers, 0);

    // Break the connection by dropping the peer.
    drop(peer);

    // Wait until the node has handled the broken connection.
    wait_until!(5, node.peer_book.get_connected_peer_count() == 0);

    let metrics = NODE_STATS.snapshot();

    assert_eq!(metrics.connections.connected_peers, 0);
    assert_eq!(metrics.connections.disconnected_peers, 1);

    // Make sure the global metrics state is reset as it will leak.
    NODE_STATS.clear();
}

#[tokio::test]
async fn connect_and_disconnect_initiator_side() {
    let setup: TestSetup = Default::default();
    let node = test_node(setup).await;
    node.initialize_metrics().await.unwrap();

    // Start a fake peer which is just a socket.
    let peer_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let peer_address = peer_listener.local_addr().unwrap();

    node.connect_to_addresses(&[peer_address]).await;

    // Accept the node's connection on peer side.
    let (mut peer_stream, _node_address) = peer_listener.accept().await.unwrap();

    let builder = snow::Builder::with_resolver(
        snarkos_network::HANDSHAKE_PATTERN.parse().unwrap(),
        Box::new(snow::resolvers::SodiumResolver),
    );
    let static_key = builder.generate_keypair().unwrap().private;
    let noise_builder = builder
        .local_private_key(&static_key)
        .psk(3, snarkos_network::HANDSHAKE_PSK);
    let mut noise = noise_builder.build_responder().unwrap();
    let mut buffer: Box<[u8]> = vec![0u8; snarkos_network::NOISE_BUF_LEN].into();
    let mut buf = [0u8; snarkos_network::NOISE_BUF_LEN]; // a temporary intermediate buffer to decrypt from

    // <- e
    peer_stream.read_exact(&mut buf[..1]).await.unwrap();
    let len = buf[0] as usize;
    let len = peer_stream.read_exact(&mut buf[..len]).await.unwrap();
    noise.read_message(&buf[..len], &mut buffer).unwrap();

    // -> e, ee, s, es
    let peer_version =
        Version::serialize(&Version::new(snarkos_network::PROTOCOL_VERSION, peer_address.port(), 0)).unwrap();
    let len = noise.write_message(&peer_version, &mut buffer).unwrap();
    peer_stream.write_all(&[len as u8]).await.unwrap();
    peer_stream.write_all(&buffer[..len]).await.unwrap();

    // <- s, se, psk
    peer_stream.read_exact(&mut buf[..1]).await.unwrap();
    let len = buf[0] as usize;
    let len = peer_stream.read_exact(&mut buf[..len]).await.unwrap();
    let len = noise.read_message(&buf[..len], &mut buffer).unwrap();
    let _node_version = Version::deserialize(&buffer[..len]).unwrap();

    let noise = noise.into_transport_mode().unwrap();
    let peer = FakeNode::new(peer_stream, peer_address, noise);

    // Needed to make sure the values have been updated.
    wait_until!(5, node.peer_book.get_connected_peer_count() == 1);

    // ...the metrics should reflect this.
    let metrics = NODE_STATS.snapshot();

    assert_eq!(metrics.connections.all_accepted, 0);
    assert_eq!(metrics.connections.all_initiated, 1);
    assert_eq!(metrics.handshakes.successes_resp, 0);
    assert_eq!(metrics.handshakes.successes_init, 1);

    assert_eq!(metrics.connections.connected_peers, 1);
    assert_eq!(metrics.connections.connecting_peers, 0);
    assert_eq!(metrics.connections.disconnected_peers, 0);

    // Break the connection by dropping the peer.
    drop(peer);

    // Wait until the node has handled the broken connection.
    wait_until!(5, node.peer_book.get_connected_peer_count() == 0);

    let metrics = NODE_STATS.snapshot();

    assert_eq!(metrics.connections.connected_peers, 0);
    assert_eq!(metrics.connections.disconnected_peers, 1);

    // Make sure the global metrics state is reset as it will leak.
    NODE_STATS.clear();
}
