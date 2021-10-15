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

use snarkos_consensus::{ConsensusParameters, MemoryPool};
use snarkvm_algorithms::CRH;
use snarkvm_dpc::{
    testnet1::instantiated::{Components, Testnet1Transaction},
    Block,
    DPCComponents,
    Network,
    TransactionError,
    TransactionScheme,
};
use snarkvm_parameters::{global::InnerCircuitIDCRH, testnet1::InnerSNARKVKParameters, Parameter};
use snarkvm_posw::PoswMarlin;
use snarkvm_utilities::{to_bytes_le, FromBytes, ToBytes};

use once_cell::sync::Lazy;
use std::{
    io::{Read, Result as IoResult, Write},
    sync::Arc,
};

mod data;
pub use data::*;

mod fixture;
pub use fixture::*;

pub static TEST_CONSENSUS_PARAMS: Lazy<ConsensusParameters> = Lazy::new(|| {
    let inner_snark_verification_key_crh_parameters: <<Components as DPCComponents>::InnerCircuitIDCRH as CRH>::Parameters = FromBytes::read_le(InnerCircuitIDCRH::load_bytes().unwrap().as_slice()).unwrap();

    let inner_snark_verification_key_crh: <Components as DPCComponents>::InnerCircuitIDCRH =
        From::from(inner_snark_verification_key_crh_parameters);

    let inner_snark_id = to_bytes_le![
        inner_snark_verification_key_crh
            .hash(&InnerSNARKVKParameters::load_bytes().unwrap())
            .unwrap()
    ]
    .unwrap();

    ConsensusParameters {
        max_block_size: 1_000_000usize,
        max_nonce: u32::max_value(),
        target_block_time: 2i64, //unix seconds
        network_id: Network::Testnet1,
        verifier: PoswMarlin::verify_only().unwrap(),
        authorized_inner_snark_ids: vec![inner_snark_id],
    }
});

pub async fn create_test_consensus() -> Arc<snarkos_consensus::Consensus> {
    let genesis_block: Block<Testnet1Transaction> = genesis();
    let ledger = FIXTURE_VK.ledger();

    let genesis_block = genesis_block.to_bytes_le().unwrap();
    let consensus = snarkos_consensus::Consensus::new(
        TEST_CONSENSUS_PARAMS.clone(),
        FIXTURE.dpc.clone(),
        genesis_block,
        ledger,
        FIXTURE_VK.storage(),
        MemoryPool::new(),
    )
    .await;
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await; // plenty of time to let consensus setup genesis block
    consensus
}
