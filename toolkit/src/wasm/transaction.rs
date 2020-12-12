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

use crate::{
    account::{Address, PrivateKey},
    dpc::{
        OfflineTransaction as OfflineTransactionNative,
        OfflineTransactionBuilder as OfflineTransactionBuilderNative,
        Record,
    },
};

use rand::{rngs::StdRng, SeedableRng};
use std::str::FromStr;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct OfflineTransactionBuilder {
    pub(crate) builder: OfflineTransactionBuilderNative,
}

#[wasm_bindgen]
impl OfflineTransactionBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            builder: OfflineTransactionBuilderNative::new(),
        }
    }

    #[wasm_bindgen]
    pub fn add_input(self, private_key: &str, record: &str) -> Self {
        let private_key = PrivateKey::from_str(private_key).unwrap();
        let record = Record::from_str(record).unwrap();

        Self {
            builder: self.builder.add_input(private_key, record).unwrap(),
        }
    }

    #[wasm_bindgen]
    pub fn add_output(self, address: &str, amount: u64) -> Self {
        let recipient = Address::from_str(address).unwrap();

        Self {
            builder: self.builder.add_output(recipient, amount).unwrap(),
        }
    }

    #[wasm_bindgen]
    pub fn network_id(self, network_id: u8) -> Self {
        let builder = self.builder;
        Self {
            builder: builder.network_id(network_id),
        }
    }

    #[wasm_bindgen]
    pub fn build(&self) -> OfflineTransaction {
        let rng = &mut StdRng::from_entropy();

        OfflineTransaction {
            offline_transaction: self.builder.build(rng).unwrap(),
        }
    }
}

#[wasm_bindgen]
pub struct OfflineTransaction {
    pub(crate) offline_transaction: OfflineTransactionNative,
}

#[wasm_bindgen]
impl OfflineTransaction {
    #[wasm_bindgen]
    pub fn from_string(offline_transaction: &str) -> Self {
        let offline_transaction = OfflineTransactionNative::from_str(offline_transaction).unwrap();
        Self { offline_transaction }
    }

    #[wasm_bindgen]
    pub fn to_string(&self) -> String {
        format!(
            "OfflineTransaction {{ offline_transaction: {} }}",
            self.offline_transaction
        )
    }
}
