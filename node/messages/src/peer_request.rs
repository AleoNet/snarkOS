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
pub struct PeerRequest;

impl MessageTrait for PeerRequest {
    /// Returns the message name.
    #[inline]
    fn name(&self) -> String {
        "PeerRequest".to_string()
    }

    /// Serializes the message into the buffer.
    #[inline]
    fn serialize<W: Write>(&self, _writer: &mut W) -> Result<()> {
        Ok(())
    }

    /// Deserializes the given buffer into a message.
    #[inline]
    fn deserialize(bytes: BytesMut) -> Result<Self> {
        match bytes.remaining() == 0 {
            true => Ok(Self),
            false => bail!("Invalid 'PeerRequest' message"),
        }
    }
}
