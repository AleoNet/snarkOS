use crate::errors::PrivateKeyError;

use snarkos_dpc::base_dpc::{instantiated::Components, parameters::CircuitParameters};
use snarkos_objects::AccountPrivateKey;

use rand::{CryptoRng, Rng};
use std::{fmt, str::FromStr};

#[derive(Debug)]
pub struct PrivateKey {
    pub(crate) private_key: AccountPrivateKey<Components>,
}

impl PrivateKey {
    pub fn new<R: Rng + CryptoRng>(rng: &mut R) -> Result<Self, PrivateKeyError> {
        let parameters = CircuitParameters::<Components>::load()?;
        let private_key =
            AccountPrivateKey::<Components>::new(&parameters.account_signature, &parameters.account_commitment, rng)?;
        Ok(Self { private_key })
    }
}

impl FromStr for PrivateKey {
    type Err = PrivateKeyError;

    fn from_str(private_key: &str) -> Result<Self, Self::Err> {
        Ok(Self {
            private_key: AccountPrivateKey::<Components>::from_str(private_key)?,
        })
    }
}

impl fmt::Display for PrivateKey {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.private_key.to_string())
    }
}
