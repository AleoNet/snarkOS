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
pub struct PeerResponse {
    pub peers: Vec<SocketAddr>,
}

impl MessageTrait for PeerResponse {
    /// Returns the message name.
    #[inline]
    fn name(&self) -> Cow<'static, str> {
        "PeerResponse".into()
    }
}

impl ToBytes for PeerResponse {
    fn write_le<W: io::Write>(&self, mut writer: W) -> io::Result<()> {
        // Return error if the number of peers exceeds the maximum.
        if self.peers.len() > u8::MAX as usize {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, format!("Too many peers: {}", self.peers.len())));
        }

        (self.peers.len() as u8).write_le(&mut writer)?;
        for peer in self.peers.iter() {
            peer.write_le(&mut writer)?;
        }
        Ok(())
    }
}

impl FromBytes for PeerResponse {
    fn read_le<R: io::Read>(mut reader: R) -> io::Result<Self> {
        let count = u8::read_le(&mut reader)?;
        let mut peers = Vec::with_capacity(count as usize);
        for _ in 0..count {
            peers.push(SocketAddr::read_le(&mut reader)?);
        }

        Ok(Self { peers })
    }
}

#[cfg(test)]
pub mod prop_tests {
    use crate::PeerResponse;
    use snarkvm::utilities::{FromBytes, ToBytes};

    use bytes::{Buf, BufMut, BytesMut};
    use proptest::{
        collection::vec,
        prelude::{any, BoxedStrategy, Strategy},
    };
    use std::net::{IpAddr, SocketAddr};
    use test_strategy::proptest;

    pub fn any_valid_socket_addr() -> BoxedStrategy<SocketAddr> {
        any::<(IpAddr, u16)>().prop_map(|(ip_addr, port)| SocketAddr::new(ip_addr, port)).boxed()
    }

    pub fn any_vec() -> BoxedStrategy<Vec<SocketAddr>> {
        vec(any_valid_socket_addr(), 0..50).prop_map(|v| v).boxed()
    }

    pub fn any_peer_response() -> BoxedStrategy<PeerResponse> {
        any_vec().prop_map(|peers| PeerResponse { peers }).boxed()
    }

    #[proptest]
    fn peer_response_roundtrip(#[strategy(any_peer_response())] peer_response: PeerResponse) {
        let mut bytes = BytesMut::default().writer();
        peer_response.write_le(&mut bytes).unwrap();
        let decoded = PeerResponse::read_le(&mut bytes.into_inner().reader()).unwrap();
        assert_eq!(decoded, peer_response);
    }
}
