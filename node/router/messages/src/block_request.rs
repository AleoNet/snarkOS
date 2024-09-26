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

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct BlockRequest {
    /// The starting block height (inclusive).
    pub start_height: u32,
    /// The ending block height (exclusive).
    pub end_height: u32,
}

impl MessageTrait for BlockRequest {
    /// Returns the message name.
    #[inline]
    fn name(&self) -> Cow<'static, str> {
        let start = self.start_height;
        let end = self.end_height;
        match start + 1 == end {
            true => format!("BlockRequest {start}"),
            false => format!("BlockRequest {start}..{end}"),
        }
        .into()
    }
}

impl ToBytes for BlockRequest {
    fn write_le<W: io::Write>(&self, mut writer: W) -> io::Result<()> {
        self.start_height.write_le(&mut writer)?;
        self.end_height.write_le(&mut writer)?;
        Ok(())
    }
}

impl FromBytes for BlockRequest {
    fn read_le<R: io::Read>(mut reader: R) -> io::Result<Self> {
        let start_height = u32::read_le(&mut reader)?;
        let end_height = u32::read_le(&mut reader)?;
        Ok(Self { start_height, end_height })
    }
}

impl Display for BlockRequest {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}..{}", self.start_height, self.end_height)
    }
}

#[cfg(test)]
pub mod prop_tests {
    use crate::BlockRequest;
    use snarkvm::utilities::{FromBytes, ToBytes};

    use bytes::{Buf, BufMut, BytesMut};
    use proptest::prelude::{any, BoxedStrategy, Strategy};
    use test_strategy::proptest;

    pub fn any_block_request() -> BoxedStrategy<BlockRequest> {
        any::<(u32, u32)>().prop_map(|(start_height, end_height)| BlockRequest { start_height, end_height }).boxed()
    }

    #[proptest]
    fn block_request_roundtrip(#[strategy(any_block_request())] block_request: BlockRequest) {
        let mut bytes = BytesMut::default().writer();
        block_request.write_le(&mut bytes).unwrap();
        let decoded = BlockRequest::read_le(&mut bytes.into_inner().reader()).unwrap();
        assert_eq![decoded, block_request];
    }
}
