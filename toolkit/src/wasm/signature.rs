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

use crate::account::PrivateKey;
use crate::signature::Signature;
use crate::signature::SignaturePublicKey;

use rand::rngs::StdRng;
use rand::SeedableRng;
use std::str::FromStr;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct SignatureScheme {
    pub(crate) signature: Signature,
}

#[wasm_bindgen]
pub struct SignatureSchemePublicKey {
    pub(crate) public_key: SignaturePublicKey,
}

#[wasm_bindgen]
impl SignatureSchemePublicKey {
    #[wasm_bindgen]
    pub fn from_private_key(private_key: &str) -> Self {
        let private_key = PrivateKey::from_str(private_key).unwrap();
        let public_key = SignaturePublicKey::from(&private_key).unwrap();
        Self { public_key }
    }

    #[wasm_bindgen]
    pub fn to_string(&self) -> String {
        format!("SignatureSchemePublicKey {{ public_key: {} }}", self.public_key)
    }
}

#[wasm_bindgen]
impl SignatureScheme {
    #[wasm_bindgen]
    pub fn from_string(signature: &str) -> Self {
        let signature = Signature::from_str(signature).unwrap();
        Self { signature }
    }

    #[wasm_bindgen]
    pub fn sign(private_key: &str, message: &str) -> Self {
        let rng = &mut StdRng::from_entropy();

        let private_key = PrivateKey::from_str(private_key).unwrap();
        let message = message.as_bytes();

        let signature = Signature::sign(&private_key, &message, rng).unwrap();
        Self { signature }
    }

    #[wasm_bindgen]
    pub fn verify(&self, public_key: &str, message: &str) -> bool {
        let public_key = SignaturePublicKey::from_str(public_key).unwrap();
        let message = message.as_bytes();

        self.signature.verify(&public_key, &message).unwrap()
    }

    #[wasm_bindgen]
    pub fn to_string(&self) -> String {
        format!("Signature {{ signature: {} }}", self.signature)
    }
}
