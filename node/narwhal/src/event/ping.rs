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
pub struct Ping<N: Network> {
    pub worker: u8,
    pub batch: Vec<EntryID<N>>,
}

impl<N: Network> Ping<N> {
    /// Initializes a new ping event.
    pub fn new(worker: u8, batch: Vec<EntryID<N>>) -> Self {
        Self { worker, batch }
    }
}

impl<N: Network> EventTrait for Ping<N> {
    /// Returns the event name.
    #[inline]
    fn name(&self) -> String {
        "Ping".to_string()
    }

    /// Serializes the event into the buffer.
    #[inline]
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_all(&self.worker.to_bytes_le()?)?;
        writer.write_all(&(self.batch.len() as u32).to_bytes_le()?)?;
        writer.write_all(&self.batch.to_bytes_le()?)?;
        Ok(())
    }

    /// Deserializes the given buffer into an event.
    #[inline]
    fn deserialize(bytes: BytesMut) -> Result<Self> {
        let mut reader = bytes.reader();

        let worker = u8::read_le(&mut reader)?;
        let num_entries = u32::read_le(&mut reader)?;
        let mut batch = Vec::with_capacity(num_entries as usize);
        for _ in 0..num_entries {
            batch.push(EntryID::read_le(&mut reader)?);
        }
        Ok(Self { worker, batch })
    }
}
