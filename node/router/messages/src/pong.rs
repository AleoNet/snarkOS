// Copyright 2024 Aleo Network Foundation
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

use snarkvm::prelude::{FromBytes, ToBytes};

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
}

impl ToBytes for Pong {
    fn write_le<W: io::Write>(&self, writer: W) -> io::Result<()> {
        let serialized_is_fork: u8 = match self.is_fork {
            Some(true) => 0,
            Some(false) => 1,
            None => 2,
        };

        serialized_is_fork.write_le(writer)
    }
}

impl FromBytes for Pong {
    fn read_le<R: io::Read>(mut reader: R) -> io::Result<Self> {
        let is_fork = match u8::read_le(&mut reader)? {
            0 => Some(true),
            1 => Some(false),
            2 => None,
            _ => return Err(error("Invalid 'Pong' message")),
        };

        Ok(Self { is_fork })
    }
}

#[cfg(test)]
pub mod tests {
    use crate::Pong;
    use snarkvm::utilities::{FromBytes, ToBytes};

    use bytes::{Buf, BufMut, BytesMut};
    use proptest::{
        option::of,
        prelude::{any, BoxedStrategy, Strategy},
    };
    use test_strategy::proptest;

    pub fn any_pong() -> BoxedStrategy<Pong> {
        of(any::<bool>()).prop_map(|is_fork| Pong { is_fork }).boxed()
    }

    #[proptest]
    fn pong_roundtrip(#[strategy(any_pong())] pong: Pong) {
        let mut bytes = BytesMut::default().writer();
        pong.write_le(&mut bytes).unwrap();
        let decoded = Pong::read_le(&mut bytes.into_inner().reader()).unwrap();
        assert_eq!(pong, decoded);
    }
}
