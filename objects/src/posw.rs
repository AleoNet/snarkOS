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

use snarkos_utilities::bytes::{FromBytes, ToBytes};

use hex;
use serde::{
    de::{Error as DeserializeError, SeqAccess, Visitor},
    ser::SerializeTuple,
    Deserialize,
    Deserializer,
    Serialize,
    Serializer,
};
use std::{
    fmt::{self, Debug, Display, Formatter},
    io::{Read, Result as IoResult, Write},
};

// Marlin PoSW proof size
const PROOF_SIZE: usize = 972;

#[derive(Clone)]
/// A Proof of Succinct Work is a SNARK proof which
pub struct ProofOfSuccinctWork(pub [u8; PROOF_SIZE]);

impl std::default::Default for ProofOfSuccinctWork {
    fn default() -> Self {
        Self::new()
    }
}

impl ProofOfSuccinctWork {
    /// Initializes an empty proof array
    fn new() -> Self {
        Self([0; PROOF_SIZE])
    }

    /// Returns the proof's size
    pub const fn size() -> usize {
        PROOF_SIZE
    }
}

impl Display for ProofOfSuccinctWork {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", hex::encode(&self.0[..]))
    }
}

impl Debug for ProofOfSuccinctWork {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "ProofOfSuccinctWork({})", hex::encode(&self.0[..]))
    }
}

impl PartialEq for ProofOfSuccinctWork {
    fn eq(&self, other: &ProofOfSuccinctWork) -> bool {
        &self.0[..] == &other.0[..]
    }
}

impl Eq for ProofOfSuccinctWork {}

impl<'de> Deserialize<'de> for ProofOfSuccinctWork {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ArrayVisitor;

        impl<'de> Visitor<'de> for ArrayVisitor {
            type Value = ProofOfSuccinctWork;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a valid proof")
            }

            fn visit_seq<S>(self, mut seq: S) -> Result<ProofOfSuccinctWork, S::Error>
            where
                S: SeqAccess<'de>,
            {
                let mut bytes = [0u8; PROOF_SIZE];
                for b in &mut bytes[..] {
                    *b = seq
                        .next_element()?
                        .ok_or_else(|| DeserializeError::custom("could not read bytes"))?;
                }
                Ok(ProofOfSuccinctWork(bytes))
            }
        }

        deserializer.deserialize_tuple(PROOF_SIZE, ArrayVisitor)
    }
}

impl Serialize for ProofOfSuccinctWork {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut tup = s.serialize_tuple(PROOF_SIZE)?;
        for byte in &self.0[..] {
            tup.serialize_element(byte)?;
        }
        tup.end()
    }
}

impl ToBytes for ProofOfSuccinctWork {
    #[inline]
    fn write<W: Write>(&self, mut writer: W) -> IoResult<()> {
        (&self.0[..]).write(&mut writer)
    }
}

impl FromBytes for ProofOfSuccinctWork {
    #[inline]
    fn read<R: Read>(mut reader: R) -> IoResult<Self> {
        let mut proof = [0; PROOF_SIZE];
        reader.read_exact(&mut proof)?;
        Ok(ProofOfSuccinctWork(proof))
    }
}

impl From<&[u8]> for ProofOfSuccinctWork {
    fn from(proof: &[u8]) -> Self {
        let mut bytes = [0; ProofOfSuccinctWork::size()];
        bytes.copy_from_slice(&proof);
        Self(bytes)
    }
}

impl From<Vec<u8>> for ProofOfSuccinctWork {
    fn from(proof: Vec<u8>) -> Self {
        Self::from(proof.as_ref())
    }
}
