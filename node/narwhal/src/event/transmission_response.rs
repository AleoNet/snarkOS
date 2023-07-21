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
pub struct TransmissionResponse<N: Network> {
    pub transmission_id: TransmissionID<N>,
    pub transmission: Transmission<N>,
}

impl<N: Network> TransmissionResponse<N> {
    /// Initializes a new transmission response event.
    pub fn new(transmission_id: TransmissionID<N>, transmission: Transmission<N>) -> Self {
        Self { transmission_id, transmission }
    }
}

impl<N: Network> From<(TransmissionID<N>, Transmission<N>)> for TransmissionResponse<N> {
    /// Initializes a new transmission response event.
    fn from((transmission_id, transmission): (TransmissionID<N>, Transmission<N>)) -> Self {
        Self::new(transmission_id, transmission)
    }
}

impl<N: Network> EventTrait for TransmissionResponse<N> {
    /// Returns the event name.
    #[inline]
    fn name(&self) -> &'static str {
        "TransmissionResponse"
    }

    /// Serializes the event into the buffer.
    #[inline]
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_all(&self.transmission_id.to_bytes_le()?)?;
        writer.write_all(&self.transmission.to_bytes_le()?)?;
        Ok(())
    }

    /// Deserializes the given buffer into an event.
    #[inline]
    fn deserialize(bytes: BytesMut) -> Result<Self> {
        let mut reader = bytes.reader();

        let transmission_id = TransmissionID::read_le(&mut reader)?;
        let transmission = Transmission::read_le(&mut reader)?;

        Ok(Self { transmission_id, transmission })
    }
}
