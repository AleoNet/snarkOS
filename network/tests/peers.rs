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

use snarkos_network::external::message::*;
use snarkos_testing::{
    network::{
        handshaken_node_and_peer,
        random_bound_address,
        read_header,
        read_payload,
        write_message_to_stream,
        TestSetup,
    },
    wait_until,
};

#[tokio::test]
async fn peer_initiator_side() {
    let setup = TestSetup {
        consensus_setup: None,
        peer_sync_interval: 2,
        min_peers: 2,
        ..Default::default()
    };
    let (node, mut peer_stream) = handshaken_node_and_peer(setup).await;

    // the buffer for peer's reads
    let mut peer_buf = [0u8; 64];

    // check if the peer has received the GetPeers message from the node
    let len = read_header(&mut peer_stream).await.unwrap().len();
    let payload = read_payload(&mut peer_stream, &mut peer_buf[..len]).await.unwrap();
    assert!(matches!(bincode::deserialize(&payload).unwrap(), Payload::GetPeers));

    // respond with a Peers message
    let (addr, _) = random_bound_address().await;
    let peers = Payload::Peers(vec![(addr, chrono::Utc::now())]);
    write_message_to_stream(peers, &mut peer_stream).await;

    // check the address has been added to the disconnected list in the peer book
    wait_until!(5, node.peers.is_disconnected(&addr));
}
