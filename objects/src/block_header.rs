use crate::{BlockHeaderHash, MerkleRootHash};
use snarkos_algorithms::crh::double_sha256;
use snarkvm_utilities::bytes::{FromBytes, ToBytes};

use serde::{Deserialize, Serialize};
use std::io::{Read, Result as IoResult, Write};

/// Block header.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BlockHeader {
    /// Hash of the previous block - 32 bytes
    pub previous_block_hash: BlockHeaderHash,

    /// Merkle root representing the transactions in the block - 32 bytes
    pub merkle_root_hash: MerkleRootHash,

    /// The block timestamp is a Unix epoch time (UTC) when the miner
    /// started hashing the header (according to the miner). - 8 bytes
    pub time: i64,

    /// Proof of work algorithm difficulty target for this block - 8 bytes
    pub difficulty_target: u64,

    /// Nonce for solving the PoW puzzle - 4 bytes
    pub nonce: u32,
}

impl BlockHeader {
    pub fn serialize(&self) -> [u8; 84] {
        let mut header_bytes = [0u8; 84];
        header_bytes[0..32].copy_from_slice(&self.previous_block_hash.0);
        header_bytes[32..64].copy_from_slice(&self.merkle_root_hash.0);
        header_bytes[64..72].copy_from_slice(&self.time.to_le_bytes());
        header_bytes[72..80].copy_from_slice(&self.difficulty_target.to_le_bytes());
        header_bytes[80..84].copy_from_slice(&self.nonce.to_le_bytes());
        header_bytes
    }

    pub fn deserialize(bytes: &[u8; 84]) -> Self {
        let mut previous_block_hash = [0u8; 32];
        let mut merkle_root_hash = [0u8; 32];
        let mut time = [0u8; 8];
        let mut difficulty_target = [0u8; 8];
        let mut nonce = [0u8; 4];

        previous_block_hash.copy_from_slice(&bytes[0..32]);
        merkle_root_hash.copy_from_slice(&bytes[32..64]);
        time.copy_from_slice(&bytes[64..72]);
        difficulty_target.copy_from_slice(&bytes[72..80]);
        nonce.copy_from_slice(&bytes[80..84]);

        Self {
            previous_block_hash: BlockHeaderHash(previous_block_hash),
            merkle_root_hash: MerkleRootHash(merkle_root_hash),
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
        let mut sliced = [0u8; 8];
        sliced.copy_from_slice(&self.get_hash().0[0..8]);

        u64::from_le_bytes(sliced)
    }
}

impl ToBytes for BlockHeader {
    #[inline]
    fn write<W: Write>(&self, mut writer: W) -> IoResult<()> {
        self.previous_block_hash.0.write(&mut writer)?;
        self.merkle_root_hash.0.write(&mut writer)?;
        self.time.to_le_bytes().write(&mut writer)?;
        self.difficulty_target.to_le_bytes().write(&mut writer)?;
        self.nonce.to_le_bytes().write(&mut writer)
    }
}

impl FromBytes for BlockHeader {
    #[inline]
    fn read<R: Read>(mut reader: R) -> IoResult<Self> {
        let previous_block_hash: [u8; 32] = FromBytes::read(&mut reader)?;
        let merkle_root_hash: [u8; 32] = FromBytes::read(&mut reader)?;
        let time: [u8; 8] = FromBytes::read(&mut reader)?;
        let difficulty_target: [u8; 8] = FromBytes::read(&mut reader)?;
        let nonce: [u8; 4] = FromBytes::read(&mut reader)?;

        Ok(Self {
            previous_block_hash: BlockHeaderHash(previous_block_hash),
            merkle_root_hash: MerkleRootHash(merkle_root_hash),
            time: i64::from_le_bytes(time),
            difficulty_target: u64::from_le_bytes(difficulty_target),
            nonce: u32::from_le_bytes(nonce),
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
        };

        let result = BlockHeader::deserialize(&block_header.serialize());

        assert_eq!(block_header, result)
    }
}
