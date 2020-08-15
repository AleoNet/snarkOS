use crate::{account::PrivateKey, errors::ViewKeyError};
use snarkos_dpc::base_dpc::{instantiated::Components, parameters::SystemParameters};
use snarkos_objects::AccountViewKey;

use std::{fmt, str::FromStr};

#[derive(Debug)]
pub struct ViewKey {
    pub(crate) view_key: AccountViewKey<Components>,
}

impl ViewKey {
    pub fn from(private_key: &PrivateKey) -> Result<Self, ViewKeyError> {
        let parameters = SystemParameters::<Components>::load()?;
        let view_key = AccountViewKey::<Components>::from_private_key(
            &parameters.account_signature,
            &parameters.account_commitment,
            &private_key.private_key,
        )?;
        Ok(Self { view_key })
    }
}

impl FromStr for ViewKey {
    type Err = ViewKeyError;

    fn from_str(private_key: &str) -> Result<Self, Self::Err> {
        Ok(Self {
            view_key: AccountViewKey::<Components>::from_str(private_key)?,
        })
    }
}

impl fmt::Display for ViewKey {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.view_key.to_string())
    }
}
