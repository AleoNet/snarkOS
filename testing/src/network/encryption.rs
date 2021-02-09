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

use crate::network::spawn_2_fake_nodes;

use snarkos_network::external::message::Payload;

use rand::{distributions::Standard, thread_rng, Rng};

#[tokio::test]
async fn encrypt_and_decrypt_a_big_payload() {
    let (mut node0, mut node1) = spawn_2_fake_nodes().await;

    // account for the overhead of serialization and noise tags
    let block_size = snarkos_network::MAX_MESSAGE_SIZE / 2;

    // create a big block containing random data
    let fake_block_bytes: Vec<u8> = (&mut thread_rng()).sample_iter(Standard).take(block_size).collect();
    let big_block = Payload::Block(fake_block_bytes.clone());

    let reading_task = tokio::spawn(async move { node1.read_payload().await.unwrap() });

    // send it from node0 to node1
    node0.write_message(&big_block).await;
    let payload = reading_task.await.unwrap();

    // check if node1 received the expected data
    if let Payload::Block(bytes) = payload {
        assert!(bytes == fake_block_bytes);
    } else {
        panic!("wrong payload received");
    }
}

#[tokio::test]
async fn encrypt_and_decrypt_small_payloads() {
    let (mut node0, mut node1) = spawn_2_fake_nodes().await;

    let mut rng = thread_rng();

    for _ in 0..100 {
        // create a small block containing random data
        let block_size: u8 = rng.gen();
        let fake_block_bytes: Vec<u8> = (&mut rng).sample_iter(Standard).take(block_size as usize).collect();
        let big_block = Payload::Block(fake_block_bytes.clone());

        // send it from node0 to node1
        node0.write_message(&big_block).await;
        let payload = node1.read_payload().await.unwrap();

        // check if node1 received the expected data
        if let Payload::Block(bytes) = payload {
            assert!(bytes == fake_block_bytes);
        } else {
            panic!("wrong payload received");
        }
    }
}
