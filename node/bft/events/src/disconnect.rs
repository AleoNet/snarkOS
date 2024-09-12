// Copyright 2024 Aleo Network Foundation
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

use std::io;

/// The reason behind the node disconnecting from a peer.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[repr(u8)]
pub enum DisconnectReason {
    /// The peer's challenge response is invalid.
    InvalidChallengeResponse,
    /// No reason given.
    NoReasonGiven,
    /// The peer is not following the protocol.
    ProtocolViolation,
    /// The peer's client is outdated, judging by its version.
    OutdatedClientVersion,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Disconnect {
    pub reason: DisconnectReason,
}

impl From<DisconnectReason> for Disconnect {
    fn from(reason: DisconnectReason) -> Self {
        Self { reason }
    }
}

impl EventTrait for Disconnect {
    /// Returns the event name.
    #[inline]
    fn name(&self) -> Cow<'static, str> {
        "Disconnect".into()
    }
}

impl ToBytes for Disconnect {
    fn write_le<W: Write>(&self, mut writer: W) -> IoResult<()> {
        (self.reason as u8).write_le(&mut writer)?;
        Ok(())
    }
}

impl FromBytes for Disconnect {
    fn read_le<R: Read>(mut reader: R) -> IoResult<Self> {
        let reason = match u8::read_le(&mut reader) {
            Ok(0) => DisconnectReason::InvalidChallengeResponse,
            Ok(1) => DisconnectReason::NoReasonGiven,
            Ok(2) => DisconnectReason::ProtocolViolation,
            Ok(3) => DisconnectReason::OutdatedClientVersion,
            _ => return Err(io::Error::new(io::ErrorKind::Other, "Invalid 'Disconnect' event")),
        };

        Ok(Self { reason })
    }
}

#[cfg(test)]
mod tests {
    use crate::{Disconnect, DisconnectReason};
    use snarkvm::console::prelude::{FromBytes, ToBytes};

    use bytes::{Buf, BufMut, BytesMut};

    #[test]
    fn serialize_deserialize() {
        // TODO switch to an iteration method that doesn't require manually updating this vec if enums are added
        let all_reasons = [
            DisconnectReason::ProtocolViolation,
            DisconnectReason::NoReasonGiven,
            DisconnectReason::InvalidChallengeResponse,
            DisconnectReason::OutdatedClientVersion,
        ];

        for reason in all_reasons.iter() {
            let disconnect = Disconnect::from(*reason);
            let mut buf = BytesMut::default().writer();
            Disconnect::write_le(&disconnect, &mut buf).unwrap();

            let disconnect = Disconnect::read_le(buf.into_inner().reader()).unwrap();
            assert_eq!(reason, &disconnect.reason);
        }
    }

    #[test]
    #[should_panic(expected = "Invalid 'Disconnect' event")]
    fn deserializing_invalid_data_panics() {
        let mut buf = BytesMut::default().writer();
        "not a DisconnectReason-value".as_bytes().write_le(&mut buf).unwrap();
        let _disconnect = Disconnect::read_le(buf.into_inner().reader()).unwrap();
    }
}
