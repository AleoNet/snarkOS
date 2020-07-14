use crate::account::{Address, PrivateKey};

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
}
