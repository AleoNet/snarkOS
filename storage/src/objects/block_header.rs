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

use snarkvm_algorithms::crh::{double_sha256, sha256d_to_u64};
use snarkvm_dpc::{BlockHeader, MerkleRootHash, PedersenMerkleRootHash, ProofOfSuccinctWork};
use snarkvm_utilities::{FromBytes, Read, ToBytes, Write};
use std::io::Result as IoResult;

use crate::Digest;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SerialBlockHeader {
    /// Hash of the previous block - 32 bytes
    pub previous_block_hash: Digest,

    /// Merkle root representing the transactions in the block - 32 bytes
    pub merkle_root_hash: MerkleRootHash,

    /// Merkle root of the transactions in the block using a Pedersen hash - 32 bytes
    pub pedersen_merkle_root_hash: PedersenMerkleRootHash,

    /// Proof of Succinct Work
    pub proof: ProofOfSuccinctWork,

    /// The block timestamp is a Unix epoch time (UTC) when the miner
    /// started hashing the header (according to the miner). - 8 bytes
    pub time: i64,

    /// Proof of work algorithm difficulty target for this block - 8 bytes
    pub difficulty_target: u64,

    /// Nonce for solving the PoW puzzle - 4 bytes
    pub nonce: u32,
}

impl SerialBlockHeader {
    pub fn hash(&self) -> Digest {
        let mut out = vec![];
        self.write_le(&mut out).expect("failed to serialize block header");
        double_sha256(&out)[..].into()
    }

    pub fn to_difficulty_hash(&self) -> u64 {
        sha256d_to_u64(&self.proof.0[..])
    }
}

impl ToBytes for SerialBlockHeader {
    fn write_le<W: Write>(&self, mut writer: W) -> IoResult<()> {
        writer.write_all(&self.previous_block_hash)?;
        writer.write_all(&self.merkle_root_hash.0)?;
        writer.write_all(&self.pedersen_merkle_root_hash.0)?;
        writer.write_all(&self.proof.0)?;
        writer.write_all(&self.time.to_le_bytes())?;
        writer.write_all(&self.difficulty_target.to_le_bytes())?;
        writer.write_all(&self.nonce.to_le_bytes())?;
        Ok(())
    }
}

impl FromBytes for SerialBlockHeader {
    fn read_le<R: Read>(mut reader: R) -> IoResult<Self> {
        let previous_block_hash = <[u8; 32]>::read_le(&mut reader)?;
        let merkle_root_hash = <[u8; 32]>::read_le(&mut reader)?;
        let pedersen_merkle_root_hash = <[u8; 32]>::read_le(&mut reader)?;
        let proof = ProofOfSuccinctWork::read_le(&mut reader)?;
        let time = <[u8; 8]>::read_le(&mut reader)?;
        let difficulty_target = <[u8; 8]>::read_le(&mut reader)?;
        let nonce = <[u8; 4]>::read_le(&mut reader)?;

        Ok(Self {
            previous_block_hash: previous_block_hash.into(),
            merkle_root_hash: MerkleRootHash(merkle_root_hash),
            time: i64::from_le_bytes(time),
            difficulty_target: u64::from_le_bytes(difficulty_target),
            nonce: u32::from_le_bytes(nonce),
            pedersen_merkle_root_hash: PedersenMerkleRootHash(pedersen_merkle_root_hash),
            proof,
        })
    }
}

impl From<BlockHeader> for SerialBlockHeader {
    fn from(other: BlockHeader) -> Self {
        SerialBlockHeader {
            previous_block_hash: other.previous_block_hash.0.into(),
            merkle_root_hash: other.merkle_root_hash,
            pedersen_merkle_root_hash: other.pedersen_merkle_root_hash,
            proof: other.proof,
            time: other.time,
            difficulty_target: other.difficulty_target,
            nonce: other.nonce,
        }
    }
}
