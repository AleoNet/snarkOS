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

use serde::{Deserialize, Serialize};

/// A fixed size message corresponding to a variable sized message.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct MessageHeader {
    pub len: u32,
}

impl MessageHeader {
    pub fn as_bytes(&self) -> [u8; 4] {
        self.len.to_be_bytes()
    }
}

// FIXME(ljedrz): use TryFrom instead
impl From<[u8; 4]> for MessageHeader {
    fn from(bytes: [u8; 4]) -> Self {
        let len = u32::from_be_bytes(bytes);

        Self { len }
    }
}
