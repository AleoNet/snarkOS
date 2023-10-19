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
    pub epoch_challenge: EpochChallenge<N>,
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
        self.epoch_challenge.write_le(&mut writer)?;
        self.block_header.write_le(&mut writer)
    }
}

impl<N: Network> FromBytes for PuzzleResponse<N> {
    fn read_le<R: io::Read>(mut reader: R) -> io::Result<Self> {
        Ok(Self { epoch_challenge: EpochChallenge::read_le(&mut reader)?, block_header: Data::read_le(reader)? })
    }
}
