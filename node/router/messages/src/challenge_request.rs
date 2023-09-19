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

use bincode::Options;
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

    /// Serializes the message into the buffer.
    #[inline]
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<()> {
        Ok(bincode::serialize_into(
            writer,
            &(self.version, self.listener_port, self.node_type, self.address, self.nonce),
        )?)
    }

    /// Deserializes the given buffer into a message.
    #[inline]
    fn deserialize(bytes: BytesMut) -> Result<Self> {
        let options =
            bincode::options().with_limit(MAXIMUM_MESSAGE_SIZE as u64).with_fixint_encoding().allow_trailing_bytes();
        let (version, listener_port, node_type, address, nonce) = options.deserialize_from(&mut bytes.reader())?;
        Ok(Self { version, listener_port, node_type, address, nonce })
    }
}

impl<N: Network> ChallengeRequest<N> {
    pub fn new(listener_port: u16, node_type: NodeType, address: Address<N>, nonce: u64) -> Self {
        Self { version: Message::<N>::VERSION, listener_port, node_type, address, nonce }
    }
}
