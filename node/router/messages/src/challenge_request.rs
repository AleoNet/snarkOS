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

use snarkvm::prelude::{FromBytes, ToBytes};

use std::borrow::Cow;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChallengeRequest<N: Network> {
    pub version: u32,
    pub listener_port: u16,
    pub node_type: NodeType,
    pub address: Address<N>,
    pub nonce: u64,
}

impl<N: Network> MessageTrait for ChallengeRequest<N> {
    /// Returns the message name.
    #[inline]
    fn name(&self) -> Cow<'static, str> {
        "ChallengeRequest".into()
    }
}

impl<N: Network> ToBytes for ChallengeRequest<N> {
    fn write_le<W: io::Write>(&self, mut writer: W) -> io::Result<()> {
        self.version.write_le(&mut writer)?;
        self.listener_port.write_le(&mut writer)?;
        self.node_type.write_le(&mut writer)?;
        self.address.write_le(&mut writer)?;
        self.nonce.write_le(&mut writer)?;
        Ok(())
    }
}

impl<N: Network> FromBytes for ChallengeRequest<N> {
    fn read_le<R: io::Read>(mut reader: R) -> io::Result<Self>
    where
        Self: Sized,
    {
        let version = u32::read_le(&mut reader)?;
        let listener_port = u16::read_le(&mut reader)?;
        let node_type = NodeType::read_le(&mut reader)?;
        let address = Address::<N>::read_le(&mut reader)?;
        let nonce = u64::read_le(&mut reader)?;

        Ok(Self { version, listener_port, node_type, address, nonce })
    }
}

impl<N: Network> ChallengeRequest<N> {
    pub fn new(listener_port: u16, node_type: NodeType, address: Address<N>, nonce: u64) -> Self {
        Self { version: Message::<N>::VERSION, listener_port, node_type, address, nonce }
    }
}
