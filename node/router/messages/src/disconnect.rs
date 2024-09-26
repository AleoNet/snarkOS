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

use snarkvm::prelude::{FromBytes, ToBytes};

use std::borrow::Cow;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Disconnect {
    pub reason: DisconnectReason,
}

impl From<DisconnectReason> for Disconnect {
    fn from(reason: DisconnectReason) -> Self {
        Self { reason }
    }
}

impl MessageTrait for Disconnect {
    /// Returns the message name.
    #[inline]
    fn name(&self) -> Cow<'static, str> {
        "Disconnect".into()
    }
}

impl ToBytes for Disconnect {
    fn write_le<W: io::Write>(&self, writer: W) -> io::Result<()> {
        self.reason.write_le(writer)
    }
}

impl FromBytes for Disconnect {
    fn read_le<R: io::Read>(mut reader: R) -> io::Result<Self> {
        Ok(Disconnect { reason: DisconnectReason::read_le(&mut reader)? })
    }
}

#[cfg(test)]
mod tests {
    use crate::{Disconnect, DisconnectReason};
    use snarkvm::{
        console::prelude::{FromBytes, ToBytes},
        prelude::{Rng, TestRng},
    };

    use bytes::{Buf, BufMut, BytesMut};

    #[test]
    fn disconnect_roundtrip() {
        // TODO switch to an iteration method that doesn't require manually updating this vec if variants are added
        let all_reasons = [
            DisconnectReason::ExceededForkRange,
            DisconnectReason::InvalidChallengeResponse,
            DisconnectReason::InvalidForkDepth,
            DisconnectReason::INeedToSyncFirst,
            DisconnectReason::NoReasonGiven,
            DisconnectReason::ProtocolViolation,
            DisconnectReason::OutdatedClientVersion,
            DisconnectReason::PeerHasDisconnected,
            DisconnectReason::PeerRefresh,
            DisconnectReason::ShuttingDown,
            DisconnectReason::SyncComplete,
            DisconnectReason::TooManyFailures,
            DisconnectReason::TooManyPeers,
            DisconnectReason::YouNeedToSyncFirst,
            DisconnectReason::YourPortIsClosed(TestRng::default().gen()),
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
    #[should_panic]
    fn disconnect_invalid_data_panics() {
        let mut buf = BytesMut::default().writer();
        "not a DisconnectReason-value".as_bytes().write_le(&mut buf).unwrap();
        let _disconnect = Disconnect::read_le(buf.into_inner().reader()).unwrap();
    }
}
