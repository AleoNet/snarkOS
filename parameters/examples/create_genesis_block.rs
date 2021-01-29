// Copyright (C) 2019-2021 Aleo Systems Inc.
// This file is part of the snarkVM library.

// The snarkVM library is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// The snarkVM library is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with the snarkVM library. If not, see <https://www.gnu.org/licenses/>.

use snarkvm_dpc::base_dpc::{
    instantiated::Components, transaction::DPCTransaction, BaseDPCComponents,
};
use snarkvm_errors::objects::TransactionError;
use snarkvm_models::genesis::Genesis;
use snarkvm_objects::{
    BlockHeader, BlockHeaderHash, DPCTransactions, MerkleRootHash, PedersenMerkleRootHash,
    ProofOfSuccinctWork,
};
use snarkvm_parameters::Transaction1;
use snarkvm_utilities::bytes::FromBytes;

use chrono::Utc;
use std::{
    fs::File,
    io::{Result as IoResult, Write},
    path::PathBuf,
};

pub fn generate<C: BaseDPCComponents>() -> Result<Vec<u8>, TransactionError> {
    // Add transactions to block
    let mut transactions = DPCTransactions::new();

    let transaction_1 = DPCTransaction::<C>::read(Transaction1::load_bytes().as_slice())?;
    transactions.push(transaction_1);

    let genesis_header = BlockHeader {
        previous_block_hash: BlockHeaderHash([0u8; 32]),
        merkle_root_hash: MerkleRootHash([0u8; 32]),
        time: Utc::now().timestamp(),
        difficulty_target: 0x07FF_FFFF_FFFF_FFFF_u64,
        nonce: 0,
        pedersen_merkle_root_hash: PedersenMerkleRootHash([0u8; 32]),
        proof: ProofOfSuccinctWork::default(),
    };

    Ok(genesis_header.serialize().to_vec())
}

pub fn store(path: &PathBuf, bytes: &[u8]) -> IoResult<()> {
    let mut file = File::create(path)?;
    file.write_all(&bytes)?;
    drop(file);
    Ok(())
}

fn main() {
    let bytes = generate::<Components>().unwrap();
    let filename = PathBuf::from("block_header.genesis");
    store(&filename, &bytes).unwrap();
}
