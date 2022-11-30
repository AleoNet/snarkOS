// Copyright (C) 2019-2022 Aleo Systems Inc.
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

use super::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pong {
    pub is_fork: Option<bool>,
}

impl MessageTrait for Pong {
    /// Returns the message name.
    #[inline]
    fn name(&self) -> String {
        "Pong".to_string()
    }

    /// Serializes the message into the buffer.
    #[inline]
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<()> {
        let serialized_is_fork: u8 = match self.is_fork {
            Some(true) => 0,
            Some(false) => 1,
            None => 2,
        };

        Ok(writer.write_all(&[serialized_is_fork])?)
    }

    /// Deserializes the given buffer into a message.
    #[inline]
    fn deserialize(mut bytes: BytesMut) -> Result<Self> {
        // Make sure a byte for the fork flag is available.
        if bytes.remaining() == 0 {
            bail!("Missing fork flag in a 'Pong'");
        }

        let fork_flag = bytes.get_u8();

        let is_fork = match fork_flag {
            0 => Some(true),
            1 => Some(false),
            2 => None,
            _ => bail!("Invalid 'Pong' message"),
        };

        Ok(Self { is_fork })
    }
}
