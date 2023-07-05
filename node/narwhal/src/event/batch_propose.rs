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
pub struct BatchPropose<N: Network> {
    pub batch_header: Data<BatchHeader<N>>,
}

impl<N: Network> BatchPropose<N> {
    /// Initializes a new batch propose event.
    pub fn new(batch_header: Data<BatchHeader<N>>) -> Self {
        Self { batch_header }
    }
}

impl<N: Network> EventTrait for BatchPropose<N> {
    /// Returns the event name.
    #[inline]
    fn name(&self) -> String {
        "BatchPropose".to_string()
    }

    /// Serializes the event into the buffer.
    #[inline]
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<()> {
        self.batch_header.serialize_blocking_into(writer)
    }

    /// Deserializes the given buffer into an event.
    #[inline]
    fn deserialize(bytes: BytesMut) -> Result<Self> {
        let reader = bytes.reader();

        let batch_header = Data::Buffer(reader.into_inner().freeze());

        Ok(Self { batch_header })
    }
}
