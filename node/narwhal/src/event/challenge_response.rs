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
pub struct ChallengeResponse<N: Network> {
    pub signature: Data<Signature<N>>,
}

impl<N: Network> EventTrait for ChallengeResponse<N> {
    /// Returns the event name.
    #[inline]
    fn name(&self) -> &'static str {
        "ChallengeResponse"
    }

    /// Serializes the event into the buffer.
    #[inline]
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<()> {
        self.signature.serialize_blocking_into(writer)
    }

    /// Deserializes the given buffer into an event.
    #[inline]
    fn deserialize(bytes: BytesMut) -> Result<Self> {
        let reader = bytes.reader();
        Ok(Self { signature: Data::Buffer(reader.into_inner().freeze()) })
    }
}

#[cfg(test)]
mod prop_tests {
    use crate::{event::EventTrait, helpers::storage::prop_tests::CryptoTestRng, ChallengeResponse};
    use bytes::{BufMut, BytesMut};
    use proptest::prelude::{any, BoxedStrategy, Strategy};
    use snarkvm::{
        ledger::narwhal::Data,
        prelude::{PrivateKey, Signature},
        utilities::rand::Uniform,
    };
    use test_strategy::proptest;

    type CurrentNetwork = snarkvm::prelude::Testnet3;

    fn any_signature() -> BoxedStrategy<Signature<CurrentNetwork>> {
        (any::<CryptoTestRng>(), 0..64)
            .prop_map(|(mut rng, message_size)| {
                let message: Vec<_> = (0..message_size).map(|_| Uniform::rand(&mut rng)).collect();
                let private_key = PrivateKey::new(&mut rng).unwrap();
                Signature::sign(&private_key, &message, &mut rng).unwrap()
            })
            .boxed()
    }

    #[proptest]
    fn serialize_deserialize(#[strategy(any_signature())] signature: Signature<CurrentNetwork>) {
        let response = ChallengeResponse::<CurrentNetwork> { signature: Data::Object(signature) };

        let mut buf = BytesMut::with_capacity(64).writer();
        ChallengeResponse::serialize(&response, &mut buf).unwrap();

        let response: ChallengeResponse<CurrentNetwork> =
            ChallengeResponse::deserialize(buf.get_ref().clone()).unwrap();
        assert_eq!(signature, response.signature.deserialize_blocking().unwrap());
    }
}
