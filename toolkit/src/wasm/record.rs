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

use crate::{
    account::{PrivateKey, ViewKey},
    record::Record as RecordNative,
};
use snarkvm_models::dpc::Record as RecordTrait;
use snarkvm_utilities::{to_bytes, ToBytes};

use std::str::FromStr;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct Record {
    pub(crate) record: RecordNative,
}

#[wasm_bindgen]
impl Record {
    #[wasm_bindgen]
    pub fn from_string(record: &str) -> Self {
        let record = RecordNative::from_str(record).unwrap();
        Self { record }
    }

    #[wasm_bindgen]
    pub fn decrypt(encrypted_record: &str, view_key: &str) -> Self {
        let view_key = ViewKey::from_str(view_key).unwrap();
        let record = RecordNative::decrypt(encrypted_record, &view_key).unwrap();
        Self { record }
    }

    #[wasm_bindgen]
    pub fn to_serial_number(&self, private_key: &str) -> String {
        let private_key = PrivateKey::from_str(private_key).unwrap();
        let serial_number = self.record.to_serial_number(&private_key).unwrap();
        hex::encode(serial_number)
    }

    #[wasm_bindgen]
    pub fn owner(&self) -> String {
        self.record.record.owner().to_string()
    }

    #[wasm_bindgen]
    pub fn is_dummy(&self) -> bool {
        self.record.record.is_dummy()
    }

    #[wasm_bindgen]
    pub fn payload(&self) -> String {
        hex::encode(to_bytes![self.record.record.payload()].unwrap())
    }

    #[wasm_bindgen]
    pub fn birth_program_id(&self) -> String {
        hex::encode(to_bytes![self.record.record.birth_program_id()].unwrap())
    }

    #[wasm_bindgen]
    pub fn death_program_id(&self) -> String {
        hex::encode(to_bytes![self.record.record.death_program_id()].unwrap())
    }

    #[wasm_bindgen]
    pub fn serial_number_nonce(&self) -> String {
        hex::encode(to_bytes![self.record.record.serial_number_nonce()].unwrap())
    }

    #[wasm_bindgen]
    pub fn commitment(&self) -> String {
        hex::encode(to_bytes![self.record.record.commitment()].unwrap())
    }

    #[wasm_bindgen]
    pub fn commitment_randomness(&self) -> String {
        hex::encode(to_bytes![self.record.record.commitment_randomness()].unwrap())
    }

    #[wasm_bindgen]
    pub fn value(&self) -> u64 {
        self.record.record.value()
    }

    #[wasm_bindgen]
    pub fn to_string(&self) -> String {
        format!("Record {{ record: {} }}", self.record)
    }
}
