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
    address::Address,
    private_key::{PrivateKey, Signature, SignaturePublicKey},
};

use rand::{rngs::StdRng, SeedableRng};
use std::str::FromStr;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct Account {
    pub(crate) private_key: PrivateKey,
    pub(crate) address: Address,
}

#[wasm_bindgen]
impl Account {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        let rng = &mut StdRng::from_entropy();
        let private_key = PrivateKey::new(rng).unwrap();
        let address = Address::from(&private_key).unwrap();
        Self { private_key, address }
    }

    #[wasm_bindgen]
    pub fn from_private_key(private_key: &str) -> Self {
        let private_key = PrivateKey::from_str(private_key).unwrap();
        let address = Address::from(&private_key).unwrap();
        Self { private_key, address }
    }

    #[wasm_bindgen]
    pub fn to_string(&self) -> String {
        format!(
            "Account {{ private_key: {}, address: {} }}",
            self.private_key, self.address
        )
    }

    #[wasm_bindgen]
    pub fn to_signature_public_key(&self) -> String {
        let public_key = self.private_key.to_signature_public_key().unwrap();
        public_key.to_string()
    }

    /// Sign a message with the private key `sk_sig`
    #[wasm_bindgen]
    pub fn sign(&self, message: &str) -> String {
        let rng = &mut StdRng::from_entropy();

        let message = message.as_bytes();

        let signature = self.private_key.sign(&message, rng).unwrap();

        signature.to_string()
    }

    /// Verify a signature signed by the private key
    /// Returns `true` if the signature is verified correctly. Otherwise, returns `false`.
    #[wasm_bindgen]
    pub fn verify(public_key: &str, message: &str, signature: &str) -> bool {
        let public_key = SignaturePublicKey::from_str(public_key).unwrap();
        let signature = Signature::from_str(signature).unwrap();
        let message = message.as_bytes();

        PrivateKey::verify(&public_key, &message, &signature).unwrap()
    }
}
