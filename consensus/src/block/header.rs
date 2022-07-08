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

use snarkvm::{
    console::{
        collections::merkle_tree::{MerklePath, MerkleTree},
        network::BHPMerkleTree,
        types::field::{Field, Zero},
    },
    fields::{FieldParameters, PrimeField},
    prelude::Network,
    utilities::{
        error,
        fmt,
        io::{Read, Result as IoResult, Write},
        str::FromStr,
        FromBytes,
        FromBytesDeserializer,
        ToBits,
        ToBytes,
        ToBytesSerializer,
        Uniform,
    },
};

use anyhow::{anyhow, bail, Result};
use serde::{de, ser::SerializeStruct, Deserialize, Deserializer, Serialize, Serializer};
// use serde::{de, ser::SerializeStruct, Deserialize, Deserializer, Serialize, Serializer};
use std::{mem::size_of, sync::atomic::AtomicBool};

// TODO (raychu86): Move this declaration.
const HEADER_TREE_DEPTH: u8 = 2;

/// The header for the block contains metadata that uniquely identifies the block.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct BlockHeader<N: Network> {
    /// The network ID of the block.
    network: u16,
    /// The height of this block - 4 bytes.
    height: u32,
    /// The round that produced this block - 8 bytes.
    round: u64,
    /// The coinbase target for this block - 8 bytes.
    coinbase_target: u64,
    /// The proof target for this block - 8 bytes.
    proof_target: u64,
    /// The Unix timestamp (UTC) for this block - 8 bytes.
    timestamp: i64,
    // /// The cumulative weight up to this block (inclusive) - 16 bytes.
    // cumulative_weight: u128,

    // TODO (raychu86): Make formalized type in Network trait.
    /// The Merkle root representing the blocks in the ledger up to the previous block
    previous_ledger_root: Field<N>,
    /// The Merkle root representing the transactions in the block
    transactions_root: Field<N>,
}

impl<N: Network> BlockHeader<N> {
    // /// Initializes a new instance of a block header metadata.
    // pub fn new<N: Network>(template: &BlockTemplate<N>) -> Self {
    //     match template.block_height() == 0 {
    //         true => Self::genesis(),
    //         false => Self {
    //             height: template.block_height(),
    //             timestamp: template.block_timestamp(),
    //             coinbase_target: template.coinbase_target(),
    //             cumulative_weight: template.cumulative_weight(),
    //         },
    //     }
    // }

    /// Initializes a new block header.
    pub fn new(
        network: u16,
        height: u32,
        round: u64,
        coinbase_target: u64,
        proof_target: u64,
        timestamp: i64,
        previous_ledger_root: Field<N>,
        transactions_root: Field<N>,
    ) -> Result<Self> {
        // Construct a new block header.
        let header = Self {
            network,
            height,
            round,
            coinbase_target,
            proof_target,
            timestamp,
            previous_ledger_root,
            transactions_root,
        };
        // Ensure the header is valid.
        match header.is_valid() {
            true => Ok(header),
            false => bail!("Invalid block header: {:?}", header),
        }
    }

    /// Initializes a new instance of a genesis block header metadata.
    pub fn genesis() -> Self {
        Self {
            network: N::ID,
            height: 0u32,
            round: 0u64,
            coinbase_target: u64::MAX,
            proof_target: u64::MAX,
            timestamp: 0i64,
            previous_ledger_root: Field::zero(),
            transactions_root: Field::zero(),
        }
    }

    /// Returns the network ID of the block.
    pub const fn network(&self) -> u16 {
        self.network
    }

    /// Returns the height of the block.
    pub const fn height(&self) -> u32 {
        self.height
    }

    /// Returns the round number of the block.
    pub const fn round(&self) -> u64 {
        self.round
    }

    /// Returns the coinbase target for this block.
    pub fn coinbase_target(&self) -> u64 {
        self.coinbase_target
    }

    /// Returns the proof target for this block.
    pub fn proof_target(&self) -> u64 {
        self.proof_target
    }

    /// Returns the Unix timestamp (UTC) for this block.
    pub fn timestamp(&self) -> i64 {
        self.timestamp
    }

    // /// Returns the size (in bytes) of a block header's metadata.
    // pub fn size() -> usize {
    //     size_of::<u32>() + size_of::<i64>() + size_of::<u64>() + size_of::<u128>()
    // }

    // /// Initializes a new instance of a block header.
    // pub fn from(
    //     previous_ledger_root: N::LedgerRoot,
    //     transactions_root: N::TransactionsRoot,
    //     metadata: BlockHeaderMetadata,
    //     nonce: N::PoSWNonce,
    //     proof: PoSWProof<N>,
    // ) -> Result<Self, BlockError> {
    //     // Construct the block header.
    //     let block_header = Self { previous_ledger_root, transactions_root, metadata, nonce, proof };
    //
    //     // Ensure the block header is well-formed.
    //     match block_header.is_valid() {
    //         true => Ok(block_header),
    //         false => Err(BlockError::Message("Invalid block header".to_string())),
    //     }
    // }

    /// Returns `true` if the block header is well-formed.
    pub fn is_valid(&self) -> bool {
        // // Ensure the ledger root is nonzero.
        // if self.previous_ledger_root == Default::default() {
        //     eprintln!("Invalid ledger root in block header");
        //     return false;
        // }
        //
        // // Ensure the transactions root is nonzero.
        // if self.transactions_root == Default::default() {
        //     eprintln!("Invalid transactions root in block header");
        //     return false;
        // }

        // Ensure the metadata and proof are valid.
        match self.height == 0u32 {
            true => self.is_genesis(),
            false => {
                // Ensure the network ID is correct.
                self.network == N::ID
                // Ensure the timestamp in the block is greater than 0.
                && self.timestamp > 0i64
                // // Ensure the PoSW proof is valid.
                // && N::posw().verify_from_block_header(self)
            }
        }
    }

    /// Returns `true` if the block header is a genesis block header.
    pub fn is_genesis(&self) -> bool {
        // Ensure the network ID is correct.
        self.network == N::ID
            // Ensure the height in the genesis block is 0.
            && self.height == 0u32
        // Ensure the round in the genesis block is 0.
       && self.round == 0u64
            // Ensure the timestamp in the genesis block is 0.
            && self.timestamp == 0i64
            // Ensure the coinbase target in the genesis block is u64::MAX.
            && self.coinbase_target == u64::MAX
            // Ensure the proof target in the genesis block is u64::MAX.
            && self.proof_target == u64::MAX
        // // Ensure the cumulative weight in the genesis block is 0u128.
        // && self.metadata.cumulative_weight == 0u128
        // // Ensure the PoSW proof is valid.
        // && N::posw().verify_from_block_header(self)
    }

    // /// Returns the previous ledger root from the block header.
    // pub fn previous_ledger_root(&self) -> N::LedgerRoot {
    //     self.previous_ledger_root
    // }
    //
    // /// Returns the transactions root in the block header.
    // pub fn transactions_root(&self) -> N::TransactionsRoot {
    //     self.transactions_root
    // }

    // /// Returns the cumulative weight up to this block (inclusive).
    // pub fn cumulative_weight(&self) -> u128 {
    //     self.metadata.cumulative_weight
    // }

    /// Returns the block header size in bytes.
    pub fn size_in_bytes() -> usize {
        2 + 4 + 8 + 8 + 8 + 8 + ((N::Field::size_in_bits() + <N::Field as PrimeField>::Parameters::REPR_SHAVE_BITS as usize) / 8) * 2
    }

    /// Returns an instance of the block header tree.
    pub fn to_header_tree(&self) -> Result<BHPMerkleTree<N, HEADER_TREE_DEPTH>> {
        // TODO (raychu86): Confirm the header tree inputs.

        let previous_ledger_root = self.previous_ledger_root.to_bits_le();
        assert_eq!(previous_ledger_root.len(), 256);

        let transactions_root = self.transactions_root.to_bits_le();
        assert_eq!(transactions_root.len(), 256);

        let metadata_1 = vec![
            self.network.to_bits_le(), // 2 bytes
            self.height.to_bits_le(),  // 4 bytes
            vec![false; 208],          // 208 bytes
        ]
        .concat(); // 256 bits
        assert_eq!(metadata_1.len(), 256);

        let metadata_2 = [
            self.round.to_bits_le(),           // 8 bytes
            self.coinbase_target.to_bits_le(), // 8 bytes
            self.proof_target.to_bits_le(),    // 8 bytes
            self.timestamp.to_bits_le(),       // 8 bytes
        ]
        .concat(); // 256 bits
        assert_eq!(metadata_2.len(), 256);

        let num_leaves = usize::pow(2, HEADER_TREE_DEPTH as u32);
        let mut leaves: Vec<Vec<bool>> = Vec::with_capacity(num_leaves);
        leaves.push(previous_ledger_root);
        leaves.push(transactions_root);
        leaves.push(metadata_1);
        leaves.push(metadata_2);
        // Sanity check that the correct number of leaves are allocated.
        assert_eq!(num_leaves, leaves.len());

        N::merkle_tree_bhp(&leaves)
    }

    // /// Returns an instance of the block header tree.
    // pub fn to_header_inclusion_proof(
    //     &self,
    //     index: usize,
    //     leaf: impl ToBytes,
    // ) -> Result<MerklePath<N::BlockHeaderRootParameters>> {
    //     let leaf_bytes = leaf.to_bytes_le()?;
    //     assert_eq!(leaf_bytes.len(), 32);
    //
    //     Ok(self.to_header_tree()?.generate_proof(index, &leaf_bytes)?)
    // }

    /// Returns the block header root.
    pub fn to_header_root(&self) -> Result<Field<N>> {
        Ok((*self.to_header_tree()?.root()).into())
    }
}

impl<N: Network> FromBytes for BlockHeader<N> {
    /// Reads the block header from the buffer.
    #[inline]
    fn read_le<R: Read>(mut reader: R) -> IoResult<Self> {
        // Read from the buffer.
        let network = u16::read_le(&mut reader)?;
        let height = u32::read_le(&mut reader)?;
        let round = u64::read_le(&mut reader)?;
        let coinbase_target = u64::read_le(&mut reader)?;
        let proof_target = u64::read_le(&mut reader)?;
        let timestamp = i64::read_le(&mut reader)?;

        let previous_ledger_root = Field::<N>::read_le(&mut reader)?;
        let transactions_root = Field::<N>::read_le(&mut reader)?;

        // Construct the block header.
        Self::new(
            network,
            height,
            round,
            coinbase_target,
            proof_target,
            timestamp,
            previous_ledger_root,
            transactions_root,
        )
        .map_err(|e| error("{e}"))
    }
}

impl<N: Network> ToBytes for BlockHeader<N> {
    /// Writes the block header to the buffer.
    #[inline]
    fn write_le<W: Write>(&self, mut writer: W) -> IoResult<()> {
        // Write to the buffer.
        self.network.write_le(&mut writer)?;
        self.height.write_le(&mut writer)?;
        self.round.write_le(&mut writer)?;
        self.coinbase_target.write_le(&mut writer)?;
        self.proof_target.write_le(&mut writer)?;
        self.timestamp.write_le(&mut writer)?;
        self.previous_ledger_root.write_le(&mut writer)?;
        self.transactions_root.write_le(&mut writer)
    }
}

// impl<N: Network> FromStr for BlockHeader<N> {
//     type Err = anyhow::Error;
//
//     fn from_str(header: &str) -> Result<Self, Self::Err> {
//         Ok(serde_json::from_str(header)?)
//     }
// }
//
// impl<N: Network> fmt::Display for BlockHeader<N> {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         write!(f, "{}", serde_json::to_string(self).map_err::<fmt::Error, _>(serde::ser::Error::custom)?)
//     }
// }

impl<N: Network> Serialize for BlockHeader<N> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match serializer.is_human_readable() {
            true => {
                let mut header = serializer.serialize_struct("BlockHeader", 8)?;
                header.serialize_field("network", &self.network)?;
                header.serialize_field("height", &self.height)?;
                header.serialize_field("round", &self.round)?;
                header.serialize_field("coinbase_target", &self.coinbase_target)?;
                header.serialize_field("proof_target", &self.proof_target)?;
                header.serialize_field("timestamp", &self.timestamp)?;
                header.serialize_field("previous_ledger_root", &self.previous_ledger_root)?;
                header.serialize_field("transactions_root", &self.transactions_root)?;
                header.end()
            }
            false => ToBytesSerializer::serialize(self, serializer),
        }
    }
}

impl<'de, N: Network> Deserialize<'de> for BlockHeader<N> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        match deserializer.is_human_readable() {
            true => {
                let header = serde_json::Value::deserialize(deserializer)?;
                Ok(Self::new(
                    serde_json::from_value(header["network"].clone()).map_err(de::Error::custom)?,
                    serde_json::from_value(header["height"].clone()).map_err(de::Error::custom)?,
                    serde_json::from_value(header["round"].clone()).map_err(de::Error::custom)?,
                    serde_json::from_value(header["coinbase_target"].clone()).map_err(de::Error::custom)?,
                    serde_json::from_value(header["proof_target"].clone()).map_err(de::Error::custom)?,
                    serde_json::from_value(header["timestamp"].clone()).map_err(de::Error::custom)?,
                    serde_json::from_value(header["previous_ledger_root"].clone()).map_err(de::Error::custom)?,
                    serde_json::from_value(header["transactions_root"].clone()).map_err(de::Error::custom)?,
                )
                .map_err(de::Error::custom)?)
            }
            false => FromBytesDeserializer::<Self>::deserialize(deserializer, "block header", Self::size_in_bytes()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // /// Returns the expected block header size by summing its expected subcomponents.
    // /// Update this method if the contents of a block header have changed.
    // fn get_expected_size<N: Network>() -> usize {
    //     32 // LedgerRoot
    //         + 32 // TransactionsRoot
    //         + BlockHeaderMetadata::size()
    //         + 32 // N::InnerScalarField
    //         + N::HEADER_PROOF_SIZE_IN_BYTES
    // }
    //
    // #[test]
    // fn test_block_header_size() {
    //     assert_eq!(get_expected_size::<Testnet1>(), Testnet1::HEADER_SIZE_IN_BYTES);
    //     assert_eq!(get_expected_size::<Testnet1>(), Testnet1::HEADER_SIZE_IN_BYTES);
    //
    //     assert_eq!(get_expected_size::<Testnet2>(), Testnet2::HEADER_SIZE_IN_BYTES);
    //     assert_eq!(get_expected_size::<Testnet2>(), Testnet2::HEADER_SIZE_IN_BYTES);
    // }
    //
    // #[test]
    // fn test_block_header_genesis_size() {
    //     let block_header = Testnet2::genesis_block().header();
    //     assert_eq!(block_header.to_bytes_le().unwrap().len(), Testnet2::HEADER_SIZE_IN_BYTES);
    //     assert_eq!(bincode::serialize(&block_header).unwrap().len(), Testnet2::HEADER_SIZE_IN_BYTES);
    // }

    // #[test]
    // fn test_block_header_serialization() {
    //     let block_header = Testnet2::genesis_block().header().to_owned();
    //
    //     // Serialize
    //     let serialized = block_header.to_bytes_le().unwrap();
    //     assert_eq!(&serialized[..], &bincode::serialize(&block_header).unwrap()[..]);
    //
    //     // Deserialize
    //     let deserialized = BlockHeader::read_le(&serialized[..]).unwrap();
    //     assert_eq!(deserialized, block_header);
    // }
    //
    // #[test]
    // fn test_block_header_serde_json() {
    //     let block_header = Testnet2::genesis_block().header().to_owned();
    //
    //     // Serialize
    //     let expected_string = block_header.to_string();
    //     let candidate_string = serde_json::to_string(&block_header).unwrap();
    //     assert_eq!(1669, candidate_string.len(), "Update me if serialization has changed");
    //     assert_eq!(expected_string, candidate_string);
    //
    //     // Deserialize
    //     assert_eq!(block_header, BlockHeader::from_str(&candidate_string).unwrap());
    //     assert_eq!(block_header, serde_json::from_str(&candidate_string).unwrap());
    // }
    //
    // #[test]
    // fn test_block_header_bincode() {
    //     let block_header = Testnet2::genesis_block().header().to_owned();
    //
    //     let expected_bytes = block_header.to_bytes_le().unwrap();
    //     assert_eq!(&expected_bytes[..], &bincode::serialize(&block_header).unwrap()[..]);
    //
    //     assert_eq!(block_header, BlockHeader::read_le(&expected_bytes[..]).unwrap());
    //     assert_eq!(block_header, bincode::deserialize(&expected_bytes[..]).unwrap());
    // }
    //
    // #[test]
    // fn test_block_header_genesis() {
    //     let block_header = Testnet2::genesis_block().header();
    //     assert!(block_header.is_genesis());
    //
    //     // Ensure the genesis block contains the following.
    //     assert_eq!(block_header.height, 0);
    //     assert_eq!(block_header.timestamp, 0);
    //     assert_eq!(block_header.coinbase_target, u64::MAX);
    //     assert_eq!(block_header.proof_target, u64::MAX);
    //     assert_eq!(block_header.cumulative_weight, 0);
    //
    //     // Ensure the genesis block does *not* contain the following.
    //     assert_ne!(block_header.previous_ledger_root, Default::default());
    //     assert_ne!(block_header.transactions_root, Default::default());
    // }
}
