use snarkos_utilities::bytes::{FromBytes, ToBytes};

use std::io::{Read, Result as IoResult, Write};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RecordPayload([u8; 32]);

impl Default for RecordPayload {
    fn default() -> Self {
        Self([0u8; 32])
    }
}

impl RecordPayload {
    pub fn to_bytes(&self) -> Vec<u8> {
        self.0.to_vec()
    }

    pub fn from_bytes(bytes: &[u8]) -> Self {
        assert_eq!(bytes.len(), 32);

        let mut payload = [0u8; 32];
        payload.copy_from_slice(&bytes[0..32]);

        Self(payload)
    }

    pub fn size(&self) -> u64 {
        32
    }
}

impl ToBytes for RecordPayload {
    #[inline]
    fn write<W: Write>(&self, mut writer: W) -> IoResult<()> {
        self.0.write(&mut writer)
    }
}

impl FromBytes for RecordPayload {
    #[inline]
    fn read<R: Read>(mut reader: R) -> IoResult<Self> {
        let payload: [u8; 32] = FromBytes::read(&mut reader)?;

        Ok(Self(payload))
    }
}
