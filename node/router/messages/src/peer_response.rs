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
        (self.peers.len().min(u8::MAX as usize) as u8).write_le(&mut writer)?;
        for peer in &self.peers {
            peer.write_le(&mut writer)?;
        }

        Ok(())
    }
}

impl FromBytes for PeerResponse {
    fn read_le<R: io::Read>(mut reader: R) -> io::Result<Self>
    where
        Self: Sized,
    {
        let count = u8::read_le(&mut reader)?;
        let mut peers = Vec::with_capacity(count as usize);
        for _ in 0..count {
            peers.push(SocketAddr::read_le(&mut reader)?);
        }

        Ok(Self { peers })
    }
}
