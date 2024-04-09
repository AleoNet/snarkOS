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
pub struct UnconfirmedSolution<N: Network> {
    pub solution_id: SolutionID<N>,
    pub solution: Data<Solution<N>>,
}

impl<N: Network> MessageTrait for UnconfirmedSolution<N> {
    /// Returns the message name.
    #[inline]
    fn name(&self) -> Cow<'static, str> {
        "UnconfirmedSolution".into()
    }
}

impl<N: Network> ToBytes for UnconfirmedSolution<N> {
    fn write_le<W: io::Write>(&self, mut writer: W) -> io::Result<()> {
        self.solution_id.write_le(&mut writer)?;
        self.solution.write_le(&mut writer)
    }
}

impl<N: Network> FromBytes for UnconfirmedSolution<N> {
    fn read_le<R: io::Read>(mut reader: R) -> io::Result<Self> {
        Ok(Self { solution_id: SolutionID::read_le(&mut reader)?, solution: Data::read_le(reader)? })
    }
}

#[cfg(test)]
pub mod prop_tests {
    use crate::{Solution, SolutionID, UnconfirmedSolution};
    use snarkvm::{
        ledger::narwhal::Data,
        prelude::{Address, FromBytes, PrivateKey, Rng, TestRng, ToBytes},
    };

    use bytes::{Buf, BufMut, BytesMut};
    use proptest::prelude::{any, BoxedStrategy, Strategy};
    use test_strategy::proptest;

    type CurrentNetwork = snarkvm::prelude::MainnetV0;

    pub fn any_solution_id() -> BoxedStrategy<SolutionID<CurrentNetwork>> {
        any::<u64>().prop_map(|seed| TestRng::fixed(seed).gen::<u64>().into()).boxed()
    }

    pub fn any_solution() -> BoxedStrategy<Solution<CurrentNetwork>> {
        any::<u64>()
            .prop_map(|seed| {
                let mut rng = TestRng::fixed(seed);
                let private_key = PrivateKey::<CurrentNetwork>::new(&mut rng).unwrap();
                let address = Address::try_from(private_key).unwrap();
                Solution::new(rng.gen(), address, rng.gen()).unwrap()
            })
            .boxed()
    }

    pub fn any_unconfirmed_solution() -> BoxedStrategy<UnconfirmedSolution<CurrentNetwork>> {
        (any_solution_id(), any_solution())
            .prop_map(|(solution_id, ps)| UnconfirmedSolution { solution_id, solution: Data::Object(ps) })
            .boxed()
    }

    #[proptest]
    fn unconfirmed_solution_roundtrip(
        #[strategy(any_unconfirmed_solution())] original: UnconfirmedSolution<CurrentNetwork>,
    ) {
        let mut buf = BytesMut::default().writer();
        UnconfirmedSolution::write_le(&original, &mut buf).unwrap();

        let deserialized: UnconfirmedSolution<CurrentNetwork> =
            UnconfirmedSolution::read_le(buf.into_inner().reader()).unwrap();
        assert_eq!(original.solution_id, deserialized.solution_id);
        assert_eq!(
            original.solution.deserialize_blocking().unwrap(),
            deserialized.solution.deserialize_blocking().unwrap(),
        );
    }
}
