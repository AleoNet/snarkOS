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
    account::{Address, ViewKey},
    signature::public::Signature,
};

use rand::{rngs::StdRng, SeedableRng};
use std::str::FromStr;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct SignatureSchemePublic {
    pub(crate) signature: Signature,
}

#[wasm_bindgen]
impl SignatureSchemePublic {
    #[wasm_bindgen]
    pub fn from_string(signature: &str) -> Self {
        let signature = Signature::from_str(signature).unwrap();
        Self { signature }
    }

    #[wasm_bindgen]
    pub fn sign(view_key: &str, message: &str) -> Self {
        let rng = &mut StdRng::from_entropy();

        let view_key = ViewKey::from_str(view_key).unwrap();
        let message = message.as_bytes();

        let signature = Signature::sign(&view_key, &message, rng).unwrap();
        Self { signature }
    }

    #[wasm_bindgen]
    pub fn verify(&self, address: &str, message: &str) -> bool {
        let address = Address::from_str(address).unwrap();
        let message = message.as_bytes();

        self.signature.verify(&address, &message).unwrap()
    }

    #[wasm_bindgen]
    pub fn to_string(&self) -> String {
        format!("Signature {{ signature: {} }}", self.signature)
    }
}
