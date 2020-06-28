use crate::private_key::PrivateKey;

use rand::{rngs::StdRng, SeedableRng};

#[wasm_bindgen_test]
pub fn private_key_is_ok_test() {
    let rng = &mut StdRng::from_entropy();
    let private_key = PrivateKey::new(None, rng);

    assert!(private_key.is_ok());
    println!("{}", private_key.unwrap());
}
