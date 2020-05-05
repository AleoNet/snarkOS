use hex;
use serde::{
    de::{Error as DeserializeError, SeqAccess, Visitor},
    ser::SerializeTuple,
    Deserialize, Deserializer, Serialize, Serializer,
};
use std::fmt::{self, Display, Debug, Formatter};
use snarkos_utilities::bytes::{FromBytes, ToBytes};
use std::io::{Read, Result as IoResult, Write};

// 2 * G1 + 1 * G2 assuming Bls12-377 and GM17.
// Marlin requires 13 * G1 + 21 * Fq = 1296 btyes.
const PROOF_SIZE: usize = 192;

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
    pub const fn size() -> usize { PROOF_SIZE }
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
                    *b = seq.next_element()?.ok_or_else(|| DeserializeError::custom("could not read bytes"))?;
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
