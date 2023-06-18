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
    pub worker: u8,
    pub transmission_id: TransmissionID<N>,
    pub transmission: Data<Transmission<N>>,
}

impl<N: Network> TransmissionResponse<N> {
    /// Initializes a new transmission response event.
    pub fn new(worker: u8, transmission_id: TransmissionID<N>, transmission: Data<Transmission<N>>) -> Self {
        Self { worker, transmission_id, transmission }
    }
}

impl<N: Network> EventTrait for TransmissionResponse<N> {
    /// Returns the event name.
    #[inline]
    fn name(&self) -> String {
        "TransmissionRequest".to_string()
    }

    /// Serializes the event into the buffer.
    #[inline]
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_all(&self.worker.to_bytes_le()?)?;
        writer.write_all(&self.transmission_id.to_bytes_le()?)?;
        self.transmission.serialize_blocking_into(writer)
    }

    /// Deserializes the given buffer into an event.
    #[inline]
    fn deserialize(bytes: BytesMut) -> Result<Self> {
        let mut reader = bytes.reader();

        let worker = u8::read_le(&mut reader)?;
        let transmission_id = TransmissionID::read_le(&mut reader)?;
        let transmission = Data::Buffer(reader.into_inner().freeze());

        Ok(Self { worker, transmission_id, transmission })
    }
}
