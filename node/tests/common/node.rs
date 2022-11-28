// Copyright (C) 2019-2022 Aleo Systems Inc.
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

use crate::common::test_peer::sample_genesis_block;
use snarkos_account::Account;
use snarkos_node::{Beacon, Client, Prover, Validator};
use snarkvm::prelude::{ConsensusMemory, Testnet3 as CurrentNetwork};

use std::str::FromStr;

pub async fn beacon() -> Beacon<CurrentNetwork, ConsensusMemory<CurrentNetwork>> {
    Beacon::new(
        "127.0.0.1:0".parse().unwrap(),
        None,
        Account::<CurrentNetwork>::from_str("APrivateKey1zkp2oVPTci9kKcUprnbzMwq95Di1MQERpYBhEeqvkrDirK1").unwrap(),
        &[],
        sample_genesis_block(), // Should load the current network's genesis block.
        None,                   // No CDN.
        None,
    )
    .await
    .expect("couldn't create beacon instance")
}

pub async fn client() -> Client<CurrentNetwork, ConsensusMemory<CurrentNetwork>> {
    Client::new(
        "127.0.0.1:0".parse().unwrap(),
        Account::<CurrentNetwork>::from_str("APrivateKey1zkp2oVPTci9kKcUprnbzMwq95Di1MQERpYBhEeqvkrDirK1").unwrap(),
        &[],
        sample_genesis_block(),
        None,
    )
    .await
    .expect("couldn't create client instance")
}

pub async fn prover() -> Prover<CurrentNetwork, ConsensusMemory<CurrentNetwork>> {
    Prover::new(
        "127.0.0.1:0".parse().unwrap(),
        Account::<CurrentNetwork>::from_str("APrivateKey1zkp2oVPTci9kKcUprnbzMwq95Di1MQERpYBhEeqvkrDirK1").unwrap(),
        &[],
        sample_genesis_block(),
        None,
    )
    .await
    .expect("couldn't create prover instance")
}

pub async fn validator() -> Validator<CurrentNetwork, ConsensusMemory<CurrentNetwork>> {
    Validator::new(
        "127.0.0.1:0".parse().unwrap(),
        None,
        Account::<CurrentNetwork>::from_str("APrivateKey1zkp2oVPTci9kKcUprnbzMwq95Di1MQERpYBhEeqvkrDirK1").unwrap(),
        &[],
        sample_genesis_block(), // Should load the current network's genesis block.
        None,                   // No CDN.
        None,
    )
    .await
    .expect("couldn't create validator instance")
}
