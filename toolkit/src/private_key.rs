use crate::errors::PrivateKeyError;

use snarkos_dpc::base_dpc::{instantiated::Components, parameters::CircuitParameters};
use snarkos_objects::AccountPrivateKey;

use rand::{CryptoRng, Rng};
use std::fmt;

pub struct PrivateKey {
    private_key: AccountPrivateKey<Components>,
}

impl PrivateKey {
    pub fn new<R: Rng + CryptoRng>(metadata: Option<[u8; 32]>, rng: &mut R) -> Result<Self, PrivateKeyError> {
        // Resolve the metadata value
        let metadata = match metadata {
            Some(metadata) => metadata,
            None => [0u8; 32],
        };

        let parameters = CircuitParameters::<Components>::load()?;
        let private_key = AccountPrivateKey::<Components>::new(&parameters.account_signature, &metadata, rng)?;
        Ok(Self { private_key })
    }

    pub(crate) fn to_inner_ref(&self) -> &AccountPrivateKey<Components> {
        &self.private_key
    }
}

impl fmt::Display for PrivateKey {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.private_key.to_string())
    }
}
