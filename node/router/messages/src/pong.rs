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

use super::*;

use std::borrow::Cow;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pong {
    pub is_fork: Option<bool>,
}

impl MessageTrait for Pong {
    /// Returns the message name.
    #[inline]
    fn name(&self) -> Cow<'static, str> {
        "Pong".into()
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
