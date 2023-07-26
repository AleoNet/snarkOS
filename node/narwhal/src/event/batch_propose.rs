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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BatchPropose<N: Network> {
    pub round: u64,
    pub batch_header: Data<BatchHeader<N>>,
}

impl<N: Network> BatchPropose<N> {
    /// Initializes a new batch propose event.
    pub fn new(round: u64, batch_header: Data<BatchHeader<N>>) -> Self {
        Self { round, batch_header }
    }
}

impl<N: Network> From<BatchHeader<N>> for BatchPropose<N> {
    /// Initializes a new batch propose event.
    fn from(batch_header: BatchHeader<N>) -> Self {
        Self::new(batch_header.round(), Data::Object(batch_header))
    }
}

impl<N: Network> EventTrait for BatchPropose<N> {
    /// Returns the event name.
    #[inline]
    fn name(&self) -> &'static str {
        "BatchPropose"
    }

    /// Serializes the event into the buffer.
    #[inline]
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_all(&self.round.to_le_bytes())?;
        self.batch_header.serialize_blocking_into(writer)
    }

    /// Deserializes the given buffer into an event.
    #[inline]
    fn deserialize(bytes: BytesMut) -> Result<Self> {
        let mut reader = bytes.reader();

        let round = u64::read_le(&mut reader)?;
        let batch_header = Data::Buffer(reader.into_inner().freeze());

        Ok(Self { round, batch_header })
    }
}

#[cfg(test)]
mod prop_tests {
    use crate::{
        event::{certificate_response::prop_tests::any_batch_header, EventTrait},
        BatchPropose,
    };
    use bytes::{BufMut, BytesMut};
    use proptest::prelude::{any, BoxedStrategy, Strategy};
    use snarkos_node_narwhal_committee::prop_tests::CommitteeContext;
    use snarkvm::prelude::narwhal::Data;
    use test_strategy::proptest;

    type CurrentNetwork = snarkvm::prelude::Testnet3;

    fn any_batch_propose() -> BoxedStrategy<BatchPropose<CurrentNetwork>> {
        any::<CommitteeContext>()
            .prop_flat_map(|committee| (any::<u64>(), any_batch_header(&committee)))
            .prop_map(|(round, batch_header)| BatchPropose::new(round, Data::Object(batch_header)))
            .boxed()
    }

    #[proptest]
    fn serialize_deserialize(#[strategy(any_batch_propose())] original: BatchPropose<CurrentNetwork>) {
        let mut buf = BytesMut::with_capacity(64).writer();
        BatchPropose::serialize(&original, &mut buf).unwrap();

        let deserialized: BatchPropose<CurrentNetwork> = BatchPropose::deserialize(buf.get_ref().clone()).unwrap();
        // because of the Data enum, we cannot compare the structs directly even though it derives PartialEq
        assert_eq!(original.round, deserialized.round);
        assert_eq!(
            original.batch_header.deserialize_blocking().unwrap(),
            deserialized.batch_header.deserialize_blocking().unwrap()
        );
    }
}
