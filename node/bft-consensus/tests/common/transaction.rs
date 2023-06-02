// Copyright (C) 2019-2023 Aleo Systems Inc.
// This file is part of the snarkOS library.

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at:
// http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use narwhal_crypto::PublicKey;
use serde::{Deserialize, Serialize};

use super::state::{Address, Amount};

pub const MAX_TRANSFER_AMOUNT: u64 = 10_000;

// A test transaction to be used in the BFT consensus.
#[derive(Serialize, Deserialize)]
pub enum Transaction {
    Transfer(Transfer),
    StakeChange(StakeChange),
}

impl Transaction {
    pub fn id(&self) -> u64 {
        match self {
            Transaction::Transfer(t) => t.id,
            Transaction::StakeChange(t) => t.id,
        }
    }
}

// A simple transfer from A to B.
#[derive(Serialize, Deserialize)]
pub struct Transfer {
    pub id: u64,
    pub from: Address,
    pub to: Address,
    pub amount: Amount,
}

#[derive(Serialize, Deserialize)]
pub struct StakeChange {
    pub id: u64,
    pub pub_key: PublicKey,
    pub stake: i64,
}
