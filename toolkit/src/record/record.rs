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

use crate::{
    account::{Address, PrivateKey, ViewKey},
    errors::RecordError,
};

use snarkos_dpc::base_dpc::{
    instantiated::Components,
    parameters::SystemParameters,
    record::{DPCRecord, EncryptedRecord, RecordEncryption},
    DPC,
};
use snarkos_models::dpc::Record as RecordTrait;
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

    /// Decrypt the encrypted record given a view key.
    pub fn decrypt(encrypted_record: &str, view_key: &ViewKey) -> Result<Self, RecordError> {
        let encrypted_record_bytes = hex::decode(encrypted_record)?;
        let encrypted_record = EncryptedRecord::<Components>::read(&encrypted_record_bytes[..])?;

        let parameters = SystemParameters::<Components>::load()?;
        let record = RecordEncryption::decrypt_record(&parameters, &view_key.view_key, &encrypted_record)?;

        Ok(Self { record })
    }

    /// Return the serial number that corresponds to the record.
    pub fn to_serial_number(&self, private_key: &PrivateKey) -> Result<Vec<u8>, RecordError> {
        let address = Address::from(&private_key)?;

        // Check that the private key corresponds with the owner of the record
        if self.record.owner() != &address.address {
            return Err(RecordError::InvalidPrivateKey);
        }

        let parameters = SystemParameters::<Components>::load()?;
        let (serial_number, _randomizer) =
            DPC::<Components>::generate_sn(&parameters, &self.record, &private_key.private_key)?;

        Ok(to_bytes![serial_number]?)
    }
}

impl FromStr for Record {
    type Err = RecordError;

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
