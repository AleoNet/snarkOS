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

mod header;
pub use header::*;

mod transaction;
pub use transaction::*;

mod transactions;
pub use transactions::*;

use snarkvm::prelude::*;

use core::fmt;
use serde::ser::SerializeStruct;

#[derive(Clone, Debug)]
pub struct Block<N: Network> {
    /// The hash of this block.
    block_hash: N::BlockHash,
    /// The hash of the previous block.
    previous_hash: N::BlockHash,
    /// The header of the block.
    header: BlockHeader<N>,
    /// The transactions in the block.
    transactions: Transactions<N>,
    // // QC for parent block
    // pub qc: crate::message::QuorumCertificate,
}

impl<N: Network> Block<N> {
    /// Initializes a new block from a given previous hash, header, and transactions list.
    pub fn from(previous_hash: N::BlockHash, header: BlockHeader<N>, transactions: Transactions<N>) -> Result<Self> {
        // Ensure the block is not empty.
        ensure!(!transactions.is_empty(), "Cannot create block with no transactions");

        // Compute the block hash.
        let block_hash = N::hash_bhp1024(&[previous_hash.to_bits_le(), header.to_header_root()?.to_bits_le()].concat())?.into();
        // N::block_hash_crh().hash_bytes(&to_bytes_le![previous_block_hash, header.to_header_root()?]?)?.into();

        // Construct the block.
        let block = Self {
            block_hash,
            previous_hash,
            header,
            transactions,
        };

        // Ensure the block is valid.
        match block.is_valid() {
            true => Ok(block),
            false => Err(anyhow!("Failed to initialize a block from given inputs").into()),
        }
    }

    // /// Initializes a new genesis block with one coinbase transaction.
    // pub fn genesis<R: Rng + CryptoRng>() -> Result<Self> {
    //     // // Compute the coinbase transaction.
    //     // let (transaction, coinbase_record) = Transaction::new_coinbase(recipient, Self::block_reward(0), true, rng)?;
    //     // let transactions = Transactions::from(&[transaction])?;
    //
    //     // Construct the genesis block header metadata.
    //     let block_height = 0u32;
    //     let block_timestamp = 0i64;
    //     let difficulty_target = u64::MAX;
    //     let cumulative_weight = 0u128;
    //
    //     // // Construct the block template.
    //     // let template = BlockTemplate::new(
    //     //     LedgerProof::<N>::default().block_hash(),
    //     //     block_height,
    //     //     block_timestamp,
    //     //     difficulty_target,
    //     //     cumulative_weight,
    //     //     LedgerTree::<N>::new()?.root(),
    //     //     transactions,
    //     //     coinbase_record,
    //     // );
    //
    //     // Ensure the block is valid genesis block.
    //     match block.is_genesis() {
    //         true => Ok(block),
    //         false => bail!("Failed to initialize a genesis block"),
    //     }
    // }

    /// Returns `true` if the block is well-formed.
    pub fn is_valid(&self) -> bool {
        // Ensure the block is not empty.
        if self.transactions.is_empty() {
            eprintln!("Block contains no transactions");
            return false;
        }

        true
    }

    /// Returns `true` if the block is a genesis block.
    pub fn is_genesis(&self) -> bool {
        // Ensure the previous block hash is zero.
        self.previous_hash == N::BlockHash::default()
            // Ensure the header is a genesis block header.
            && self.header.is_genesis()
            // Ensure there is one transaction in the genesis block.
            && self.transactions.len() == 1
    }

    /// Returns the block hash.
    pub const fn hash(&self) -> N::BlockHash {
        self.block_hash
    }

    /// Returns the previous block hash.
    pub const fn previous_hash(&self) -> N::BlockHash {
        self.previous_hash
    }

    /// Returns the block header.
    pub const fn header(&self) -> &BlockHeader<N> {
        &self.header
    }

    /// Returns the transactions in the block.
    pub const fn transactions(&self) -> &Transactions<N> {
        &self.transactions
    }
}

impl<N: Network> FromStr for Block<N> {
    type Err = anyhow::Error;

    /// Initializes the block from a JSON-string.
    fn from_str(block: &str) -> Result<Self, Self::Err> {
        Ok(serde_json::from_str(block)?)
    }
}

impl<N: Network> Display for Block<N> {
    /// Displays the block as a JSON-string.
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            serde_json::to_string(self).map_err::<fmt::Error, _>(serde::ser::Error::custom)?
        )
    }
}

impl<N: Network> FromBytes for Block<N> {
    /// Reads the block from the buffer.
    #[inline]
    fn read_le<R: Read>(mut reader: R) -> IoResult<Self> {
        // Read from the buffer.
        let block_hash: N::BlockHash = FromBytes::read_le(&mut reader)?;
        let previous_hash = FromBytes::read_le(&mut reader)?;
        let header = FromBytes::read_le(&mut reader)?;
        let transactions = FromBytes::read_le(&mut reader)?;
        let block = Self::from(previous_hash, header, transactions).map_err(|e| error(e.to_string()))?;
        // Ensure the block hash matches, and the block is valid.
        match block_hash == block.hash() && block.is_valid() {
            true => Ok(block),
            false => Err(error("Mismatching block hash, possible data corruption")),
        }
    }
}

impl<N: Network> ToBytes for Block<N> {
    /// Writes the block to the buffer.
    #[inline]
    fn write_le<W: Write>(&self, mut writer: W) -> IoResult<()> {
        // Ensure the block is valid.
        if !self.is_valid() {
            return Err(error("Cannot write an invalid block to the buffer"));
        }
        // Write to the buffer.
        self.block_hash.write_le(&mut writer)?;
        self.previous_hash.write_le(&mut writer)?;
        self.header.write_le(&mut writer)?;
        self.transactions.write_le(&mut writer)
    }
}

impl<N: Network> Serialize for Block<N> {
    /// Serializes the block to a JSON-string or buffer.
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match serializer.is_human_readable() {
            true => {
                let mut block = serializer.serialize_struct("Block", 4)?;
                block.serialize_field("block_hash", &self.block_hash)?;
                block.serialize_field("previous_hash", &self.previous_hash)?;
                block.serialize_field("header", &self.header)?;
                block.serialize_field("transactions", &self.transactions)?;
                block.end()
            }
            false => ToBytesSerializer::serialize_with_size_encoding(self, serializer),
        }
    }
}

impl<'de, N: Network> Deserialize<'de> for Block<N> {
    /// Deserializes the block from a JSON-string or buffer.
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        match deserializer.is_human_readable() {
            true => {
                let block = serde_json::Value::deserialize(deserializer)?;
                let block_hash: N::BlockHash = serde_json::from_value(block["block_hash"].clone()).map_err(de::Error::custom)?;

                // Recover the block.
                let block = Self::from(
                    serde_json::from_value(block["previous_hash"].clone()).map_err(de::Error::custom)?,
                    serde_json::from_value(block["header"].clone()).map_err(de::Error::custom)?,
                    serde_json::from_value(block["transactions"].clone()).map_err(de::Error::custom)?,
                )
                .map_err(de::Error::custom)?;

                // Ensure the block hash matches.
                match block_hash == block.hash() {
                    true => Ok(block),
                    false => Err(error("Mismatching block hash, possible data corruption")).map_err(de::Error::custom),
                }
            }
            false => FromBytesDeserializer::<Self>::deserialize_with_size_encoding(deserializer, "block"),
        }
    }
}
