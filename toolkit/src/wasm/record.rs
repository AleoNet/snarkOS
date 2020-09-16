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
    account::{PrivateKey, ViewKey},
    record::Record as RecordNative,
};

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
    pub fn decrypt_record(encrypted_record: &str, view_key: &str) -> Self {
        let view_key = ViewKey::from_str(view_key).unwrap();
        let record = RecordNative::decrypt_record(encrypted_record, &view_key).unwrap();
        Self { record }
    }

    #[wasm_bindgen]
    pub fn derive_serial_number(&self, private_key: &str) -> String {
        let private_key = PrivateKey::from_str(private_key).unwrap();
        let serial_number = self.record.derive_serial_number(&private_key).unwrap();
        hex::encode(serial_number)
    }

    #[wasm_bindgen]
    pub fn to_string(&self) -> String {
        format!("Record {{ record: {} }}", self.record)
    }
}
