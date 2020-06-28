use crate::account::{PrivateKey, PublicKey};

use rand::{rngs::StdRng, SeedableRng};
use std::str::FromStr;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct Account {
    pub(crate) private_key: PrivateKey,
    pub(crate) public_key: PublicKey,
}

#[wasm_bindgen]
impl Account {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        let rng = &mut StdRng::from_entropy();
        let private_key = PrivateKey::new(None, rng).unwrap();
        let public_key = PublicKey::from(&private_key).unwrap();
        Self {
            private_key,
            public_key,
        }
    }

    #[wasm_bindgen]
    pub fn from_private_key(private_key: &str) -> Self {
        let private_key = PrivateKey::from_str(private_key).unwrap();
        let public_key = PublicKey::from(&private_key).unwrap();
        Self {
            private_key,
            public_key,
        }
    }
}
