// Copyright (C) 2019-2023 Aleo Systems Inc.
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

use serde::{Deserialize, Serialize};

use super::state::{Address, Amount};

pub const MAX_TRANSFER_AMOUNT: u64 = 10_000;

// A test transaction to be used in the BFT consensus.
#[derive(Serialize, Deserialize)]
pub enum Transaction {
    Transfer(Transfer),
}

// A simple transfer from A to B.
#[derive(Serialize, Deserialize)]
pub struct Transfer {
    pub from: Address,
    pub to: Address,
    pub amount: Amount,
}
