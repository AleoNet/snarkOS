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

use snarkvm::{
    ledger::narwhal::Data,
    prelude::{FromBytes, ToBytes},
};

use std::borrow::Cow;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockResponse<N: Network> {
    /// The original block request.
    pub request: BlockRequest,
    /// The blocks.
    pub blocks: Data<DataBlocks<N>>,
}

impl<N: Network> MessageTrait for BlockResponse<N> {
    /// Returns the message name.
    #[inline]
    fn name(&self) -> Cow<'static, str> {
        let start = self.request.start_height;
        let end = self.request.end_height;
        match start + 1 == end {
            true => format!("BlockResponse {start}"),
            false => format!("BlockResponse {start}..{end}"),
        }
        .into()
    }
}

impl<N: Network> ToBytes for BlockResponse<N> {
    fn write_le<W: io::Write>(&self, mut writer: W) -> io::Result<()> {
        self.request.write_le(&mut writer)?;
        self.blocks.write_le(writer)
    }
}

impl<N: Network> FromBytes for BlockResponse<N> {
    fn read_le<R: io::Read>(mut reader: R) -> io::Result<Self> {
        let request = BlockRequest::read_le(&mut reader)?;
        let blocks = Data::read_le(reader)?;
        Ok(Self { request, blocks })
    }
}

#[cfg(test)]
pub mod prop_tests {
    use crate::{block_request::prop_tests::any_block_request, BlockResponse, DataBlocks};
    use snarkvm::{
        ledger::ledger_test_helpers::sample_genesis_block,
        prelude::{block::Block, narwhal::Data},
        utilities::{FromBytes, TestRng, ToBytes},
    };

    use bytes::{Buf, BufMut, BytesMut};
    use proptest::{
        collection::vec,
        prelude::{any, BoxedStrategy, Strategy},
    };
    use test_strategy::proptest;

    type CurrentNetwork = snarkvm::prelude::MainnetV0;

    pub fn any_block() -> BoxedStrategy<Block<CurrentNetwork>> {
        any::<u64>().prop_map(|seed| sample_genesis_block(&mut TestRng::fixed(seed))).boxed()
    }

    pub fn any_data_blocks() -> BoxedStrategy<DataBlocks<CurrentNetwork>> {
        vec(any_block(), 0..=1).prop_map(DataBlocks).boxed()
    }

    pub fn any_block_response() -> BoxedStrategy<BlockResponse<CurrentNetwork>> {
        (any_block_request(), any_data_blocks())
            .prop_map(|(request, data_blocks)| BlockResponse { request, blocks: Data::Object(data_blocks) })
            .boxed()
    }

    #[proptest]
    fn block_response_roundtrip(#[strategy(any_block_response())] block_response: BlockResponse<CurrentNetwork>) {
        let mut bytes = BytesMut::default().writer();
        block_response.write_le(&mut bytes).unwrap();
        let decoded = BlockResponse::<CurrentNetwork>::read_le(&mut bytes.into_inner().reader()).unwrap();
        assert_eq!(block_response.request, decoded.request);
        assert_eq!(
            block_response.blocks.deserialize_blocking().unwrap(),
            decoded.blocks.deserialize_blocking().unwrap(),
        );
    }
}
