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

use snarkvm::{
    ledger::narwhal::Data,
    prelude::{FromBytes, ToBytes},
};

use std::borrow::Cow;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PuzzleResponse<N: Network> {
    pub epoch_hash: N::BlockHash,
    pub block_header: Data<Header<N>>,
}

impl<N: Network> MessageTrait for PuzzleResponse<N> {
    /// Returns the message name.
    #[inline]
    fn name(&self) -> Cow<'static, str> {
        "PuzzleResponse".into()
    }
}

impl<N: Network> ToBytes for PuzzleResponse<N> {
    fn write_le<W: io::Write>(&self, mut writer: W) -> io::Result<()> {
        self.epoch_hash.write_le(&mut writer)?;
        self.block_header.write_le(&mut writer)
    }
}

impl<N: Network> FromBytes for PuzzleResponse<N> {
    fn read_le<R: io::Read>(mut reader: R) -> io::Result<Self> {
        Ok(Self { epoch_hash: N::BlockHash::read_le(&mut reader)?, block_header: Data::read_le(reader)? })
    }
}

#[cfg(test)]
pub mod prop_tests {
    use crate::{challenge_response::prop_tests::any_genesis_header, PuzzleResponse};
    use snarkvm::{
        console::prelude::{FromBytes, ToBytes},
        ledger::narwhal::Data,
        prelude::{Network, Rng, TestRng},
    };

    use bytes::{Buf, BufMut, BytesMut};
    use proptest::prelude::{any, BoxedStrategy, Strategy};
    use test_strategy::proptest;

    type CurrentNetwork = snarkvm::prelude::MainnetV0;

    pub fn any_epoch_hash() -> BoxedStrategy<<CurrentNetwork as Network>::BlockHash> {
        any::<u64>()
            .prop_map(|seed| {
                let mut rng = TestRng::fixed(seed);
                rng.gen()
            })
            .boxed()
    }

    pub fn any_puzzle_response() -> BoxedStrategy<PuzzleResponse<CurrentNetwork>> {
        (any_epoch_hash(), any_genesis_header())
            .prop_map(|(epoch_hash, bh)| PuzzleResponse { epoch_hash, block_header: Data::Object(bh) })
            .boxed()
    }

    #[proptest]
    fn puzzle_response_roundtrip(#[strategy(any_puzzle_response())] original: PuzzleResponse<CurrentNetwork>) {
        let mut buf = BytesMut::default().writer();
        PuzzleResponse::write_le(&original, &mut buf).unwrap();

        let deserialized: PuzzleResponse<CurrentNetwork> = PuzzleResponse::read_le(buf.into_inner().reader()).unwrap();
        assert_eq!(original.epoch_hash, deserialized.epoch_hash);
        assert_eq!(
            original.block_header.deserialize_blocking().unwrap(),
            deserialized.block_header.deserialize_blocking().unwrap(),
        );
    }
}
