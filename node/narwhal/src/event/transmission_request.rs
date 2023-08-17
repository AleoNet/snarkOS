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
pub struct TransmissionRequest<N: Network> {
    pub transmission_id: TransmissionID<N>,
}

impl<N: Network> TransmissionRequest<N> {
    /// Initializes a new transmission request event.
    pub const fn new(transmission_id: TransmissionID<N>) -> Self {
        Self { transmission_id }
    }
}

impl<N: Network> From<TransmissionID<N>> for TransmissionRequest<N> {
    /// Initializes a new transmission request event.
    fn from(transmission_id: TransmissionID<N>) -> Self {
        Self::new(transmission_id)
    }
}

impl<N: Network> EventTrait for TransmissionRequest<N> {
    /// Returns the event name.
    #[inline]
    fn name(&self) -> &'static str {
        "TransmissionRequest"
    }

    /// Deserializes the given buffer into an event.
    #[inline]
    fn deserialize(bytes: BytesMut) -> Result<Self> {
        let mut reader = bytes.reader();

        let transmission_id = TransmissionID::read_le(&mut reader)?;

        Ok(Self { transmission_id })
    }
}

impl<N: Network> ToBytes for TransmissionRequest<N> {
    fn write_le<W: Write>(&self, mut writer: W) -> IoResult<()> {
        self.transmission_id.write_le(&mut writer)?;
        Ok(())
    }
}

#[cfg(test)]
pub mod prop_tests {
    use crate::{
        event::EventTrait,
        helpers::storage::prop_tests::{any_puzzle_commitment, any_transaction_id},
        TransmissionRequest,
    };
    use bytes::{BufMut, BytesMut};
    use proptest::{
        prelude::{BoxedStrategy, Strategy},
        prop_oneof,
    };
    use snarkvm::ledger::narwhal::TransmissionID;
    use test_strategy::proptest;
    type CurrentNetwork = snarkvm::prelude::Testnet3;

    fn any_transmission_id() -> BoxedStrategy<TransmissionID<CurrentNetwork>> {
        prop_oneof![
            any_puzzle_commitment().prop_map(TransmissionID::Solution),
            any_transaction_id().prop_map(TransmissionID::Transaction),
        ]
        .boxed()
    }

    pub fn any_transmission_request() -> BoxedStrategy<TransmissionRequest<CurrentNetwork>> {
        any_transmission_id().prop_map(TransmissionRequest::new).boxed()
    }

    #[proptest]
    fn serialize_deserialize(#[strategy(any_transmission_request())] original: TransmissionRequest<CurrentNetwork>) {
        let mut buf = BytesMut::default().writer();
        TransmissionRequest::serialize(&original, &mut buf).unwrap();

        let deserialized = TransmissionRequest::deserialize(buf.get_ref().clone()).unwrap();
        assert_eq!(original, deserialized);
    }
}
