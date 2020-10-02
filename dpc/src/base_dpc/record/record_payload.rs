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

use std::io::{Read, Result as IoResult, Write};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RecordPayload([u8; 32]);

impl Default for RecordPayload {
    fn default() -> Self {
        Self([0u8; 32])
    }
}

impl RecordPayload {
    pub fn to_bytes(&self) -> &[u8] {
        &self.0[..]
    }

    pub fn from_bytes(bytes: &[u8]) -> Self {
        assert_eq!(bytes.len(), 32);

        let mut payload = [0u8; 32];
        payload.copy_from_slice(&bytes[0..32]);

        Self(payload)
    }

    pub fn size(&self) -> usize {
        self.0.len()
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
