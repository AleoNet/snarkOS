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
pub struct Ping<N: Network> {
    pub version: u32,
    pub node_type: NodeType,
    pub block_locators: Option<BlockLocators<N>>,
}

impl<N: Network> MessageTrait for Ping<N> {
    /// Returns the message name.
    #[inline]
    fn name(&self) -> Cow<'static, str> {
        "Ping".into()
    }
}

impl<N: Network> ToBytes for Ping<N> {
    fn write_le<W: io::Write>(&self, mut writer: W) -> io::Result<()> {
        self.version.write_le(&mut writer)?;
        self.node_type.write_le(&mut writer)?;
        if let Some(locators) = &self.block_locators {
            1u8.write_le(&mut writer)?;
            locators.write_le(&mut writer)?;
        } else {
            0u8.write_le(&mut writer)?;
        }

        Ok(())
    }
}

impl<N: Network> FromBytes for Ping<N> {
    fn read_le<R: io::Read>(mut reader: R) -> io::Result<Self> {
        let version = u32::read_le(&mut reader)?;
        let node_type = NodeType::read_le(&mut reader)?;

        let selector = u8::read_le(&mut reader)?;
        let block_locators = match selector {
            0 => None,
            1 => Some(BlockLocators::read_le(&mut reader)?),
            _ => return Err(error("Invalid block locators marker")),
        };

        Ok(Self { version, node_type, block_locators })
    }
}

impl<N: Network> Ping<N> {
    pub fn new(node_type: NodeType, block_locators: Option<BlockLocators<N>>) -> Self {
        Self { version: <Message<N>>::VERSION, node_type, block_locators }
    }
}

#[cfg(test)]
pub mod prop_tests {
    use crate::{challenge_request::prop_tests::any_node_type, Ping};
    use snarkos_node_sync_locators::{test_helpers::sample_block_locators, BlockLocators};
    use snarkvm::utilities::{FromBytes, ToBytes};

    use bytes::{Buf, BufMut, BytesMut};
    use proptest::prelude::{any, BoxedStrategy, Strategy};
    use test_strategy::proptest;

    type CurrentNetwork = snarkvm::prelude::MainnetV0;

    pub fn any_block_locators() -> BoxedStrategy<BlockLocators<CurrentNetwork>> {
        any::<u32>().prop_map(sample_block_locators).boxed()
    }

    pub fn any_ping() -> BoxedStrategy<Ping<CurrentNetwork>> {
        (any::<u32>(), any_block_locators(), any_node_type())
            .prop_map(|(version, bls, node_type)| Ping { version, block_locators: Some(bls), node_type })
            .boxed()
    }

    #[proptest]
    fn ping_roundtrip(#[strategy(any_ping())] ping: Ping<CurrentNetwork>) {
        let mut bytes = BytesMut::default().writer();
        ping.write_le(&mut bytes).unwrap();
        let decoded = Ping::<CurrentNetwork>::read_le(&mut bytes.into_inner().reader()).unwrap();
        assert_eq!(ping, decoded);
    }
}
