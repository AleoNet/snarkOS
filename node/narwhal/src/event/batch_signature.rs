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

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct BatchSignature<N: Network> {
    pub batch_id: Field<N>,
    pub signature: Signature<N>,
    pub timestamp: i64,
}

impl<N: Network> BatchSignature<N> {
    /// Initializes a new batch signature event.
    pub fn new(batch_id: Field<N>, signature: Signature<N>, timestamp: i64) -> Self {
        Self { batch_id, signature, timestamp }
    }
}

impl<N: Network> EventTrait for BatchSignature<N> {
    /// Returns the event name.
    #[inline]
    fn name(&self) -> &'static str {
        "BatchSignature"
    }

    /// Serializes the event into the buffer.
    #[inline]
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_all(&self.batch_id.to_bytes_le()?)?;
        writer.write_all(&self.signature.to_bytes_le()?)?;
        writer.write_all(&self.timestamp.to_bytes_le()?)?;
        Ok(())
    }

    /// Deserializes the given buffer into an event.
    #[inline]
    fn deserialize(bytes: BytesMut) -> Result<Self> {
        let mut reader = bytes.reader();
        Ok(Self {
            batch_id: Field::read_le(&mut reader)?,
            signature: Signature::read_le(&mut reader)?,
            timestamp: i64::read_le(&mut reader)?,
        })
    }
}
