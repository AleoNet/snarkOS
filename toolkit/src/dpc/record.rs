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

use crate::errors::DPCError;

use snarkos_dpc::base_dpc::{instantiated::Components, DPCRecord};
use snarkos_utilities::{
    bytes::{FromBytes, ToBytes},
    to_bytes,
};

use std::{fmt, str::FromStr};

#[derive(Debug)]
pub struct Record {
    pub(crate) record: DPCRecord<Components>,
}

impl Record {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut output = vec![];
        self.record.write(&mut output).expect("serialization to bytes failed");
        output
    }
}

impl FromStr for Record {
    type Err = DPCError;

    fn from_str(record: &str) -> Result<Self, Self::Err> {
        let record = hex::decode(record)?;

        Ok(Self {
            record: DPCRecord::<Components>::read(&record[..])?,
        })
    }
}

impl fmt::Display for Record {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            hex::encode(to_bytes![self.record].expect("serialization to bytes failed"))
        )
    }
}
