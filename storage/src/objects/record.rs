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

use snarkvm_dpc::{AleoAmount, RecordScheme, testnet1::{instantiated::Components, Record}};
use snarkvm_utilities::{variable_length_integer, FromBytes, ToBytes, Write};

use std::{convert::TryInto, io::Result as IoResult};

use crate::{Address, Digest};
use anyhow::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SerialRecord {
    pub owner: Address,
    pub is_dummy: bool,
    pub value: AleoAmount,
    pub payload: Digest,
    pub birth_program_id: Digest,
    pub death_program_id: Digest,
    pub serial_number_nonce: Digest,
    pub commitment: Digest,
    pub commitment_randomness: Digest,
}

impl ToBytes for SerialRecord {
    fn write_le<W: Write>(&self, mut writer: W) -> IoResult<()> {
        self.owner.0.write_le(&mut writer)?;

        self.is_dummy.write_le(&mut writer)?;
        self.value.write_le(&mut writer)?;
        self.payload.write_le(&mut writer)?;

        variable_length_integer(self.birth_program_id.len() as u64).write_le(&mut writer)?;
        self.birth_program_id.write_le(&mut writer)?;

        variable_length_integer(self.death_program_id.len() as u64).write_le(&mut writer)?;
        self.death_program_id.write_le(&mut writer)?;

        self.serial_number_nonce.write_le(&mut writer)?;
        self.commitment.write_le(&mut writer)?;
        self.commitment_randomness.write_le(&mut writer)
    }
}

pub trait VMRecord: Sized {
    fn deserialize(record: &SerialRecord) -> IoResult<Self>;

    fn serialize(&self) -> Result<SerialRecord>;
}

fn to_bytes_to_digest<B: ToBytes>(from: &B) -> IoResult<Digest> {
    let mut out = Digest::default();
    from.write_le(&mut out.0)?;
    Ok(out)
}

// cannot use parameterized types here because recordscheme doesnt bound Owner associated type
impl VMRecord for Record<Components> {
    fn deserialize(record: &SerialRecord) -> IoResult<Self> {
        //todo: make this not bad
        let mut buf = vec![];
        record.write_le(&mut buf)?;
        Self::read_le(&mut &buf[..])
    }

    fn serialize(&self) -> Result<SerialRecord> {
        Ok(SerialRecord {
            owner: self.owner().clone().into(),
            is_dummy: self.is_dummy(),
            value: AleoAmount(self.value().try_into()?),
            payload: self.payload().to_bytes().into(),
            birth_program_id: self.birth_program_id().into(),
            death_program_id: self.death_program_id().into(),
            serial_number_nonce: to_bytes_to_digest(self.serial_number_nonce())?,
            commitment: to_bytes_to_digest(&self.commitment())?,
            commitment_randomness: to_bytes_to_digest(&self.commitment_randomness())?,
        })
    }
}
