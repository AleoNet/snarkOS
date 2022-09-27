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

// TODO (raychu86): Move block locators out of the network module.

/// The maximum number of linear block locators.
pub const MAXIMUM_LINEAR_BLOCK_LOCATORS: u32 = 64;
/// The maximum number of quadratic block locators.
pub const MAXIMUM_QUADRATIC_BLOCK_LOCATORS: u32 = 32;
/// The total maximum number of block locators.
pub const MAXIMUM_BLOCK_LOCATORS: u32 = MAXIMUM_LINEAR_BLOCK_LOCATORS.saturating_add(MAXIMUM_QUADRATIC_BLOCK_LOCATORS);

use snarkvm::{
    compiler::Header,
    prelude::Network,
    utilities::{
        fmt,
        io::{Read, Result as IoResult, Write},
        str::FromStr,
        FromBytes,
        FromBytesDeserializer,
        ToBytes,
        ToBytesSerializer,
    },
};

use anyhow::{bail, Result};
use rayon::{iter::IntoParallelIterator, prelude::*};
use serde::{de, ser, ser::SerializeStruct, Deserialize, Deserializer, Serialize, Serializer};
use std::{
    collections::BTreeMap,
    io::{Error, ErrorKind},
    ops::Deref,
};

///
/// A helper struct to represent block locators from the ledger.
///
/// The current format of block locators is \[(block_height, block_hash, block_header)\].
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BlockLocators<N: Network> {
    block_locators: BTreeMap<u32, (N::BlockHash, Option<Header<N>>)>,
}

impl<N: Network> BlockLocators<N> {
    #[inline]
    pub fn from(block_locators: BTreeMap<u32, (N::BlockHash, Option<Header<N>>)>) -> Result<Self> {
        // Check that the number of block_locators is greater than 0 and less than the MAXIMUM_BLOCK_LOCATORS.
        let num_locators = block_locators.len();
        match num_locators > 0 && num_locators <= MAXIMUM_BLOCK_LOCATORS as usize {
            true => Ok(Self { block_locators }),
            false => {
                let error = format!(
                    "Invalid number of block locators. Expected between 1 and {}, found {}",
                    MAXIMUM_BLOCK_LOCATORS, num_locators
                );
                error!("{}", error);
                bail!("{}", error)
            }
        }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.block_locators.is_empty()
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.block_locators.len()
    }

    #[inline]
    pub fn get_block_hash(&self, block_height: u32) -> Option<N::BlockHash> {
        self.block_locators.get(&block_height).map(|(block_hash, _)| *block_hash)
    }

    // #[inline]
    // pub fn get_cumulative_weight(&self, block_height: u32) -> Option<u128> {
    //     match self.block_locators.get(&block_height) {
    //         Some((_, header)) => header.as_ref().map(|header| header.cumulative_weight()),
    //         _ => None,
    //     }
    // }
}

impl<N: Network> FromBytes for BlockLocators<N> {
    #[inline]
    fn read_le<R: Read>(mut reader: R) -> IoResult<Self> {
        let num_locators: u32 = FromBytes::read_le(&mut reader)?;

        // Check that the number of block_locators is greater than 0 and less than the MAXIMUM_BLOCK_LOCATORS.
        if num_locators == 0 || num_locators > MAXIMUM_BLOCK_LOCATORS {
            error!(
                "Invalid number of block locators. Expected between 1 and {}, found {}",
                MAXIMUM_BLOCK_LOCATORS, num_locators
            );
            return Err(ErrorKind::Other.into());
        }

        let mut block_headers_bytes = Vec::with_capacity(num_locators as usize);

        for _ in 0..num_locators {
            let height: u32 = FromBytes::read_le(&mut reader)?;
            let hash: N::BlockHash = FromBytes::read_le(&mut reader)?;
            let header_exists: bool = FromBytes::read_le(&mut reader)?;

            if header_exists {
                // TODO (raychu86): Reintroduce formal block header size.
                // let mut buffer = vec![0u8; Header::<N>::size_in_bytes()];
                let mut buffer = vec![0u8; 102];
                reader.read_exact(&mut buffer)?;
                block_headers_bytes.push((height, hash, Some(buffer)));
            } else {
                block_headers_bytes.push((height, hash, None));
            }
        }

        let block_locators = block_headers_bytes
            .into_par_iter()
            .map(|(height, hash, bytes)| (height, (hash, bytes.map(|bytes| Header::<N>::read_le(&bytes[..]).unwrap()))))
            .collect::<BTreeMap<_, (_, _)>>();

        Self::from(block_locators).map_err(|error| Error::new(ErrorKind::Other, error))
    }
}

impl<N: Network> ToBytes for BlockLocators<N> {
    #[inline]
    fn write_le<W: Write>(&self, mut writer: W) -> IoResult<()> {
        (self.block_locators.len() as u32).write_le(&mut writer)?;

        for (height, (hash, header)) in &self.block_locators {
            height.write_le(&mut writer)?;
            hash.write_le(&mut writer)?;
            match header {
                Some(header) => {
                    true.write_le(&mut writer)?;
                    header.write_le(&mut writer)?;
                }
                None => false.write_le(&mut writer)?,
            }
        }
        Ok(())
    }
}

impl<N: Network> FromStr for BlockLocators<N> {
    type Err = anyhow::Error;

    fn from_str(block_locators: &str) -> Result<Self, Self::Err> {
        Ok(serde_json::from_str(block_locators)?)
    }
}

impl<N: Network> fmt::Display for BlockLocators<N> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", serde_json::to_string(self).map_err::<fmt::Error, _>(ser::Error::custom)?)
    }
}

impl<N: Network> Serialize for BlockLocators<N> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match serializer.is_human_readable() {
            true => {
                let mut block_locators = serializer.serialize_struct("BlockLocators", 1)?;
                block_locators.serialize_field("block_locators", &self.block_locators)?;
                block_locators.end()
            }
            false => ToBytesSerializer::serialize_with_size_encoding(self, serializer),
        }
    }
}

impl<'de, N: Network> Deserialize<'de> for BlockLocators<N> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        match deserializer.is_human_readable() {
            true => {
                let block_locators = serde_json::Value::deserialize(deserializer)?;
                let block_locators: BTreeMap<u32, (N::BlockHash, Option<Header<N>>)> =
                    serde_json::from_value(block_locators["block_locators"].clone()).map_err(de::Error::custom)?;
                Self::from(block_locators).map_err(de::Error::custom)
            }
            false => FromBytesDeserializer::<Self>::deserialize_with_size_encoding(deserializer, "block locators"),
        }
    }
}

impl<N: Network> Default for BlockLocators<N> {
    #[inline]
    fn default() -> Self {
        // Initialize the list of block locators.
        let mut block_locators: BTreeMap<u32, (N::BlockHash, Option<Header<N>>)> = Default::default();
        // Add the genesis locator.
        // TODO (raychu86): Insert the genesis block hash.
        block_locators.insert(0, (N::BlockHash::default(), None));
        // block_locators.insert(0, (N::genesis_block().hash(), None));

        Self { block_locators }
    }
}

impl<N: Network> Deref for BlockLocators<N> {
    type Target = BTreeMap<u32, (N::BlockHash, Option<Header<N>>)>;

    fn deref(&self) -> &Self::Target {
        &self.block_locators
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    pub type CurrentNetwork = snarkvm::prelude::Testnet3;

    #[test]
    fn test_block_locators_serde_json() {
        // let expected_block_height = CurrentNetwork::genesis_block().height();
        // let expected_block_hash = CurrentNetwork::genesis_block().hash();
        // let expected_block_header = CurrentNetwork::genesis_block().header().clone();
        // let expected_block_locators =
        //     BlockLocators::<CurrentNetwork>::from([(expected_block_height, (expected_block_hash, Some(expected_block_header)))].into())
        //         .unwrap();
        //
        // // Serialize
        // let expected_string = expected_block_locators.to_string();
        // let candidate_string = serde_json::to_string(&expected_block_locators).unwrap();
        // assert_eq!(1703, candidate_string.len(), "Update me if serialization has changed");
        // assert_eq!(expected_string, candidate_string);
        //
        // // Deserialize
        // assert_eq!(expected_block_locators, BlockLocators::from_str(&candidate_string).unwrap());
        // assert_eq!(expected_block_locators, serde_json::from_str(&candidate_string).unwrap());
    }

    #[test]
    fn test_block_locators_bincode() {
        // let expected_block_height = CurrentNetwork::genesis_block().height();
        // let expected_block_hash = CurrentNetwork::genesis_block().hash();
        // let expected_block_header = CurrentNetwork::genesis_block().header().clone();
        // let expected_block_locators =
        //     BlockLocators::<CurrentNetwork>::from([(expected_block_height, (expected_block_hash, Some(expected_block_header)))].into())
        //         .unwrap();
        //
        // // Serialize
        // let expected_bytes = expected_block_locators.to_bytes_le().unwrap();
        // let candidate_bytes = bincode::serialize(&expected_block_locators).unwrap();
        // assert_eq!(944, expected_bytes.len(), "Update me if serialization has changed");
        // // TODO (howardwu): Serialization - Handle the inconsistency between ToBytes and Serialize (off by a length encoding).
        // assert_eq!(&expected_bytes[..], &candidate_bytes[8..]);
        //
        // // Deserialize
        // assert_eq!(expected_block_locators, BlockLocators::read_le(&expected_bytes[..]).unwrap());
        // assert_eq!(expected_block_locators, bincode::deserialize(&candidate_bytes[..]).unwrap());
    }
}
