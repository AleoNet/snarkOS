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
    dpc::{OfflineTransaction as OfflineTransactionNative, Record},
};

use rand::{rngs::StdRng, SeedableRng};
use std::str::FromStr;
use wasm_bindgen::prelude::*;

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

    // TODO genericize to Vec<&str> - Currently WasmAbi doesnt support vectors or arrays of strings
    #[wasm_bindgen]
    pub fn create_offline_transaction(
        spender1: &str,
        spender2: Option<String>,
        record1: &str,
        record2: Option<String>,
        recipient1: &str,
        recipient2: Option<String>,
        recipient_amounts: Vec<u64>,
        network_id: u8,
    ) -> Self {
        let rng = &mut StdRng::from_entropy();

        let mut spenders = vec![PrivateKey::from_str(spender1).unwrap()];
        if let Some(spender) = spender2 {
            spenders.push(PrivateKey::from_str(&spender).unwrap());
        }

        let mut records_to_spend = vec![Record::from_str(record1).unwrap()];
        if let Some(record) = record2 {
            records_to_spend.push(Record::from_str(&record).unwrap());
        }

        assert_eq!(spenders.len(), records_to_spend.len());

        let mut recipients = vec![Address::from_str(recipient1).unwrap()];
        if let Some(recipient) = recipient2 {
            recipients.push(Address::from_str(&recipient).unwrap());
        }

        assert_eq!(recipients.len(), recipient_amounts.len());

        let offline_transaction = OfflineTransactionNative::offline_transaction_execution(
            spenders,
            records_to_spend,
            recipients,
            recipient_amounts,
            network_id,
            rng,
        )
        .unwrap();

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
