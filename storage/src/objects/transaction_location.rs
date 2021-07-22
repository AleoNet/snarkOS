// Copyright (C) 2019-2021 Aleo Systems Inc.
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

use snarkvm_utilities::bytes::{FromBytes, ToBytes};

use std::io::{Read, Result as IoResult, Write};

use crate::Digest;

/// Represents address of certain transaction within block
#[derive(Debug, PartialEq, Clone)]
pub struct TransactionLocation {
    /// Transaction index within the block
    pub index: u32,
    /// Block hash
    pub block_hash: Digest,
}

impl ToBytes for TransactionLocation {
    #[inline]
    fn write_le<W: Write>(&self, mut writer: W) -> IoResult<()> {
        self.index.write_le(&mut writer)?;
        self.block_hash.write_le(&mut writer)
    }
}

impl FromBytes for TransactionLocation {
    #[inline]
    fn read_le<R: Read>(mut reader: R) -> IoResult<Self> {
        let index: u32 = FromBytes::read_le(&mut reader)?;
        let block_hash: [u8; 32] = FromBytes::read_le(&mut reader)?;

        Ok(Self {
            index,
            block_hash: (&block_hash[..]).into(),
        })
    }
}
