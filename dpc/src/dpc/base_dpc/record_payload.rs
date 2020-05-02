use snarkos_utilities::bytes::{FromBytes, ToBytes};

use std::io::{Read, Result as IoResult, Write};

//TODO enforce lock condition
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PaymentRecordPayload {
    /// Attributes
    pub balance: u64,
    pub lock: u32,
}

impl Default for PaymentRecordPayload {
    fn default() -> Self {
        Self { balance: 0, lock: 0 }
    }
}

impl PaymentRecordPayload {
    pub fn to_bytes(&self) -> Vec<u8> {
        [self.balance.to_le_bytes().to_vec(), self.lock.to_le_bytes().to_vec()].concat()
    }

    pub fn from_bytes(bytes: &[u8]) -> Self {
        assert_eq!(bytes.len(), 12);

        let mut balance_bytes = [0u8; 8];
        let mut lock_bytes = [0u8; 4];

        balance_bytes.copy_from_slice(&bytes[0..8]);
        lock_bytes.copy_from_slice(&bytes[8..12]);

        Self {
            balance: u64::from_le_bytes(balance_bytes),
            lock: u32::from_le_bytes(lock_bytes),
        }
    }

    pub fn size(&self) -> u64 {
        12
    }
}

impl ToBytes for PaymentRecordPayload {
    #[inline]
    fn write<W: Write>(&self, mut writer: W) -> IoResult<()> {
        self.balance.write(&mut writer)?;
        self.lock.write(&mut writer)
    }
}

impl FromBytes for PaymentRecordPayload {
    #[inline]
    fn read<R: Read>(mut reader: R) -> IoResult<Self> {
        let balance: u64 = FromBytes::read(&mut reader)?;
        let lock: u32 = FromBytes::read(&mut reader)?;

        Ok(Self { balance, lock })
    }
}
