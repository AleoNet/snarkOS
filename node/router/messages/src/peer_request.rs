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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PeerRequest;

impl MessageTrait for PeerRequest {
    /// Returns the message name.
    #[inline]
    fn name(&self) -> Cow<'static, str> {
        "PeerRequest".into()
    }
}

impl ToBytes for PeerRequest {
    fn write_le<W: io::Write>(&self, _writer: W) -> io::Result<()> {
        Ok(())
    }
}

impl FromBytes for PeerRequest {
    fn read_le<R: io::Read>(_reader: R) -> io::Result<Self> {
        Ok(Self)
    }
}

#[cfg(test)]
pub mod tests {
    use crate::PeerRequest;
    use snarkvm::utilities::{FromBytes, ToBytes};

    use bytes::{Buf, BufMut, BytesMut};

    #[test]
    fn peer_request_roundtrip() {
        let peer_request = PeerRequest;
        let mut bytes = BytesMut::default().writer();
        peer_request.write_le(&mut bytes).unwrap();
        let decoded = PeerRequest::read_le(&mut bytes.into_inner().reader()).unwrap();
        assert_eq!(decoded, peer_request);
    }
}
