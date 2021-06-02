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

use crate::{account::PrivateKey, errors::ViewKeyError};
use snarkvm_dpc::{
    testnet1::{instantiated::Components, parameters::SystemParameters},
    AccountViewKey,
};

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

    fn from_str(view_key: &str) -> Result<Self, Self::Err> {
        Ok(Self {
            view_key: AccountViewKey::<Components>::from_str(view_key)?,
        })
    }
}

impl fmt::Display for ViewKey {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.view_key.to_string())
    }
}
