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
pub struct TransmissionResponse<N: Network> {
    pub transmission_id: TransmissionID<N>,
    pub transmission: Transmission<N>,
}

impl<N: Network> TransmissionResponse<N> {
    /// Initializes a new transmission response event.
    pub fn new(transmission_id: TransmissionID<N>, transmission: Transmission<N>) -> Self {
        Self { transmission_id, transmission }
    }
}

impl<N: Network> From<(TransmissionID<N>, Transmission<N>)> for TransmissionResponse<N> {
    /// Initializes a new transmission response event.
    fn from((transmission_id, transmission): (TransmissionID<N>, Transmission<N>)) -> Self {
        Self::new(transmission_id, transmission)
    }
}

impl<N: Network> EventTrait for TransmissionResponse<N> {
    /// Returns the event name.
    #[inline]
    fn name(&self) -> Cow<'static, str> {
        "TransmissionResponse".into()
    }
}

impl<N: Network> ToBytes for TransmissionResponse<N> {
    fn write_le<W: Write>(&self, mut writer: W) -> IoResult<()> {
        self.transmission_id.write_le(&mut writer)?;
        self.transmission.write_le(&mut writer)?;
        Ok(())
    }
}

impl<N: Network> FromBytes for TransmissionResponse<N> {
    fn read_le<R: Read>(mut reader: R) -> IoResult<Self> {
        let transmission_id = TransmissionID::read_le(&mut reader)?;
        let transmission = Transmission::read_le(&mut reader)?;

        Ok(Self { transmission_id, transmission })
    }
}

#[cfg(test)]
pub mod prop_tests {
    use crate::{
        prop_tests::{any_puzzle_commitment, any_transaction_id},
        TransmissionResponse,
    };
    use snarkvm::{
        console::prelude::{FromBytes, ToBytes},
        ledger::narwhal::{Data, Transmission, TransmissionID},
    };

    use bytes::{Buf, BufMut, Bytes, BytesMut};
    use proptest::{
        collection,
        prelude::{any, BoxedStrategy, Strategy},
        prop_oneof,
    };
    use test_strategy::proptest;

    type CurrentNetwork = snarkvm::prelude::Testnet3;

    pub fn any_transmission() -> BoxedStrategy<(TransmissionID<CurrentNetwork>, Transmission<CurrentNetwork>)> {
        prop_oneof![
            (any_puzzle_commitment(), collection::vec(any::<u8>(), 256..=256)).prop_map(|(pc, bytes)| (
                TransmissionID::Solution(pc),
                Transmission::Solution(Data::Buffer(Bytes::from(bytes)))
            )),
            (any_transaction_id(), collection::vec(any::<u8>(), 512..=512)).prop_map(|(tid, bytes)| (
                TransmissionID::Transaction(tid),
                Transmission::Transaction(Data::Buffer(Bytes::from(bytes)))
            )),
        ]
        .boxed()
    }

    pub fn any_transmission_response() -> BoxedStrategy<TransmissionResponse<CurrentNetwork>> {
        any_transmission().prop_map(|(id, t)| TransmissionResponse::new(id, t)).boxed()
    }

    // TODO(nkls): dedup this?
    pub fn any_large_transmission() -> BoxedStrategy<(TransmissionID<CurrentNetwork>, Transmission<CurrentNetwork>)> {
        prop_oneof![
            (any_puzzle_commitment(), collection::vec(any::<u8>(), 65535..=65535 * 5)).prop_map(|(pc, bytes)| (
                TransmissionID::Solution(pc),
                Transmission::Solution(Data::Buffer(Bytes::from(bytes)))
            )),
            (any_transaction_id(), collection::vec(any::<u8>(), 65535..=65353 * 5)).prop_map(|(tid, bytes)| (
                TransmissionID::Transaction(tid),
                Transmission::Transaction(Data::Buffer(Bytes::from(bytes)))
            )),
        ]
        .boxed()
    }

    pub fn any_large_transmission_response() -> BoxedStrategy<TransmissionResponse<CurrentNetwork>> {
        any_large_transmission().prop_map(|(id, t)| TransmissionResponse::new(id, t)).boxed()
    }

    #[proptest]
    fn serialize_deserialize(#[strategy(any_transmission_response())] original: TransmissionResponse<CurrentNetwork>) {
        let mut buf = BytesMut::default().writer();
        TransmissionResponse::write_le(&original, &mut buf).unwrap();

        let deserialized = TransmissionResponse::read_le(buf.into_inner().reader()).unwrap();
        assert_eq!(original, deserialized);
    }
}
