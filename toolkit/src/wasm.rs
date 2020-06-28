use crate::private_key::PrivateKey;

use rand::{rngs::StdRng, SeedableRng};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct Account;

#[wasm_bindgen]
impl Account {
    #[wasm_bindgen]
    pub fn new() -> Self {
        let rng = &mut StdRng::from_entropy();
        let private_key = PrivateKey::new(None, rng);

        println!("{}", private_key.unwrap());
        Self
    }
}
