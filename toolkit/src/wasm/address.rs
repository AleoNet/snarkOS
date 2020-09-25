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

use crate::account::{
    address::Address as AddressNative,
    private_key::PrivateKey,
    view_key::{Signature, ViewKey},
};

use std::str::FromStr;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct Address {
    pub(crate) address: AddressNative,
}

#[wasm_bindgen]
impl Address {
    #[wasm_bindgen]
    pub fn from_private_key(private_key: &str) -> Self {
        let private_key = PrivateKey::from_str(private_key).unwrap();
        let address = AddressNative::from(&private_key).unwrap();
        Self { address }
    }

    #[wasm_bindgen]
    pub fn from_view_key(view_key: &str) -> Self {
        let view_key = ViewKey::from_str(view_key).unwrap();
        let address = AddressNative::from_view_key(&view_key).unwrap();
        Self { address }
    }

    #[wasm_bindgen]
    pub fn from_string(address: &str) -> Self {
        let address = AddressNative::from_str(address).unwrap();
        Self { address }
    }

    /// Verify a signature signed by the view key
    /// Returns `true` if the signature is verified correctly. Otherwise, returns `false`.
    #[wasm_bindgen]
    pub fn verify(&self, message: &str, signature: &str) -> bool {
        let signature = Signature::from_str(signature).unwrap();
        let message = message.as_bytes();

        self.address.verify(&message, &signature).unwrap()
    }

    #[wasm_bindgen]
    pub fn to_string(&self) -> String {
        format!("Address {{ address: {} }}", self.address)
    }
}
