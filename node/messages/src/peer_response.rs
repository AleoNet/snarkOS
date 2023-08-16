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
pub struct PeerResponse {
    pub peers: Vec<SocketAddr>,
}

impl MessageTrait for PeerResponse {
    /// Returns the message name.
    #[inline]
    fn name(&self) -> Cow<'static, str> {
        "PeerResponse".into()
    }

    /// Serializes the message into the buffer.
    #[inline]
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<()> {
        Ok(bincode::serialize_into(writer, &self.peers)?)
    }

    /// Deserializes the given buffer into a message.
    #[inline]
    fn deserialize(bytes: BytesMut) -> Result<Self> {
        let options =
            bincode::options().with_limit(MAXIMUM_MESSAGE_SIZE as u64).with_fixint_encoding().allow_trailing_bytes();
        Ok(Self { peers: options.deserialize_from(&mut bytes.reader())? })
    }
}
