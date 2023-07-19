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
pub struct ChallengeRequest<N: Network> {
    pub version: u32,
    pub listener_port: u16,
    pub address: Address<N>,
    pub nonce: u64,
}

impl<N: Network> ChallengeRequest<N> {
    /// Creates a new `ChallengeRequest` event.
    pub fn new(listener_port: u16, address: Address<N>, nonce: u64) -> Self {
        Self { version: Event::<N>::VERSION, listener_port, address, nonce }
    }
}

impl<N: Network> EventTrait for ChallengeRequest<N> {
    /// Returns the event name.
    #[inline]
    fn name(&self) -> &'static str {
        "ChallengeRequest"
    }

    /// Serializes the event into the buffer.
    #[inline]
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<()> {
        Ok(bincode::serialize_into(writer, &(self.version, self.listener_port, self.address, self.nonce))?)
    }

    /// Deserializes the given buffer into an event.
    #[inline]
    fn deserialize(bytes: BytesMut) -> Result<Self> {
        let (version, listener_port, address, nonce) = bincode::deserialize_from(&mut bytes.reader())?;
        Ok(Self { version, listener_port, address, nonce })
    }
}
