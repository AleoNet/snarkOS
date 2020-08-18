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

use crate::{BlockHeaderHash, MerkleRootHash, PedersenMerkleRootHash, ProofOfSuccinctWork};
use snarkos_algorithms::crh::{double_sha256, sha256d_to_u64};
use snarkos_utilities::bytes::{FromBytes, ToBytes};

use serde::{Deserialize, Serialize};
use std::{
    io::{Read, Result as IoResult, Write},
    mem::size_of,
};

/// Block header.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BlockHeader {
    /// Hash of the previous block - 32 bytes
    pub previous_block_hash: BlockHeaderHash,

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

const HEADER_SIZE: usize = {
    BlockHeaderHash::size()
        + MerkleRootHash::size()
        + PedersenMerkleRootHash::size()
        + ProofOfSuccinctWork::size()
        + size_of::<i64>()
        + size_of::<u64>()
        + size_of::<u32>()
};

impl BlockHeader {
    pub const fn size() -> usize {
        HEADER_SIZE
    }

    pub fn serialize(&self) -> [u8; HEADER_SIZE] {
        let mut header_bytes = [0u8; HEADER_SIZE];
        let mut start = 0;
        let mut end = BlockHeaderHash::size();

        header_bytes[start..end].copy_from_slice(&self.previous_block_hash.0);

        start = end;
        end += MerkleRootHash::size();
        header_bytes[start..end].copy_from_slice(&self.merkle_root_hash.0);

        start = end;
        end += PedersenMerkleRootHash::size();
        header_bytes[start..end].copy_from_slice(&self.pedersen_merkle_root_hash.0);

        start = end;
        end += ProofOfSuccinctWork::size();
        header_bytes[start..end].copy_from_slice(&self.proof.0);

        start = end;
        end += size_of::<i64>();
        header_bytes[start..end].copy_from_slice(&self.time.to_le_bytes());

        start = end;
        end += size_of::<u64>();
        header_bytes[start..end].copy_from_slice(&self.difficulty_target.to_le_bytes());

        start = end;
        end += size_of::<u32>();
        header_bytes[start..end].copy_from_slice(&self.nonce.to_le_bytes());

        header_bytes
    }

    pub fn deserialize(bytes: &[u8; HEADER_SIZE]) -> Self {
        let mut previous_block_hash = [0u8; 32];
        let mut merkle_root_hash = [0u8; 32];
        let mut pedersen_merkle_root_hash = [0u8; 32];
        let mut proof = [0u8; ProofOfSuccinctWork::size()];
        let mut time = [0u8; 8];
        let mut difficulty_target = [0u8; 8];
        let mut nonce = [0u8; 4];

        let mut start = 0;
        let mut end = BlockHeaderHash::size();
        previous_block_hash.copy_from_slice(&bytes[start..end]);

        start = end;
        end += MerkleRootHash::size();
        merkle_root_hash.copy_from_slice(&bytes[start..end]);

        start = end;
        end += PedersenMerkleRootHash::size();
        pedersen_merkle_root_hash.copy_from_slice(&bytes[start..end]);

        start = end;
        end += ProofOfSuccinctWork::size();
        proof.copy_from_slice(&bytes[start..end]);

        start = end;
        end += size_of::<i64>();
        time.copy_from_slice(&bytes[start..end]);

        start = end;
        end += size_of::<u64>();
        difficulty_target.copy_from_slice(&bytes[start..end]);

        start = end;
        end += size_of::<u32>();
        nonce.copy_from_slice(&bytes[start..end]);

        Self {
            previous_block_hash: BlockHeaderHash(previous_block_hash),
            merkle_root_hash: MerkleRootHash(merkle_root_hash),
            pedersen_merkle_root_hash: PedersenMerkleRootHash(pedersen_merkle_root_hash),
            proof: ProofOfSuccinctWork(proof),
            time: i64::from_le_bytes(time),
            difficulty_target: u64::from_le_bytes(difficulty_target),
            nonce: u32::from_le_bytes(nonce),
        }
    }

    pub fn get_hash(&self) -> BlockHeaderHash {
        let serialized = self.serialize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&double_sha256(&serialized));

        BlockHeaderHash(hash)
    }

    pub fn to_difficulty_hash(&self) -> u64 {
        sha256d_to_u64(&self.proof.0[..])
    }
}

impl ToBytes for BlockHeader {
    #[inline]
    fn write<W: Write>(&self, mut writer: W) -> IoResult<()> {
        self.previous_block_hash.0.write(&mut writer)?;
        self.merkle_root_hash.0.write(&mut writer)?;
        self.pedersen_merkle_root_hash.0.write(&mut writer)?;
        self.proof.write(&mut writer)?;
        self.time.to_le_bytes().write(&mut writer)?;
        self.difficulty_target.to_le_bytes().write(&mut writer)?;
        self.nonce.to_le_bytes().write(&mut writer)
    }
}

impl FromBytes for BlockHeader {
    #[inline]
    fn read<R: Read>(mut reader: R) -> IoResult<Self> {
        let previous_block_hash = <[u8; 32]>::read(&mut reader)?;
        let merkle_root_hash = <[u8; 32]>::read(&mut reader)?;
        let pedersen_merkle_root_hash = <[u8; 32]>::read(&mut reader)?;
        let proof = ProofOfSuccinctWork::read(&mut reader)?;
        let time = <[u8; 8]>::read(&mut reader)?;
        let difficulty_target = <[u8; 8]>::read(&mut reader)?;
        let nonce = <[u8; 4]>::read(&mut reader)?;

        Ok(Self {
            previous_block_hash: BlockHeaderHash(previous_block_hash),
            merkle_root_hash: MerkleRootHash(merkle_root_hash),
            time: i64::from_le_bytes(time),
            difficulty_target: u64::from_le_bytes(difficulty_target),
            nonce: u32::from_le_bytes(nonce),
            pedersen_merkle_root_hash: PedersenMerkleRootHash(pedersen_merkle_root_hash),
            proof,
        })
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;

    #[test]
    fn serialize() {
        let block_header = BlockHeader {
            previous_block_hash: BlockHeaderHash([0u8; 32]),
            merkle_root_hash: MerkleRootHash([0u8; 32]),
            time: Utc::now().timestamp(),
            difficulty_target: 0u64,
            nonce: 0u32,
            pedersen_merkle_root_hash: PedersenMerkleRootHash([0u8; 32]),
            proof: ProofOfSuccinctWork([0u8; ProofOfSuccinctWork::size()]),
        };

        let serialized1 = block_header.serialize();
        let result = BlockHeader::deserialize(&serialized1);

        let mut serialized2 = vec![];
        block_header.write(&mut serialized2).unwrap();
        let de = BlockHeader::read(&serialized2[..]).unwrap();

        assert_eq!(&serialized1[..], &serialized2[..]);
        assert_eq!(&serialized1[..], &bincode::serialize(&block_header).unwrap()[..]);
        assert_eq!(block_header, result);
        assert_eq!(block_header, de);
    }
}
