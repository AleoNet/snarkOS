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

use crate::account::{PrivateKey, ViewKey as ViewKeyNative};

use std::str::FromStr;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct ViewKey {
    pub(crate) view_key: ViewKeyNative,
}

#[wasm_bindgen]
impl ViewKey {
    #[wasm_bindgen]
    pub fn from_private_key(private_key: &str) -> Self {
        let private_key = PrivateKey::from_str(private_key).unwrap();
        let view_key = ViewKeyNative::from(&private_key).unwrap();
        Self { view_key }
    }

    #[wasm_bindgen]
    pub fn to_string(&self) -> String {
        format!("ViewKey {{ view_key: {} }}", self.view_key)
    }
}
