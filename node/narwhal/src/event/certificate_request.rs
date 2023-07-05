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
pub struct CertificateRequest<N: Network> {
    pub certificate_id: Field<N>,
}

impl<N: Network> CertificateRequest<N> {
    /// Initializes a new certificate request event.
    pub const fn new(certificate_id: Field<N>) -> Self {
        Self { certificate_id }
    }
}

impl<N: Network> From<Field<N>> for CertificateRequest<N> {
    /// Initializes a new certificate request event.
    fn from(certificate_id: Field<N>) -> Self {
        Self::new(certificate_id)
    }
}

impl<N: Network> EventTrait for CertificateRequest<N> {
    /// Returns the event name.
    #[inline]
    fn name(&self) -> String {
        "CertificateRequest".to_string()
    }

    /// Serializes the event into the buffer.
    #[inline]
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_all(&self.certificate_id.to_bytes_le()?)?;
        Ok(())
    }

    /// Deserializes the given buffer into an event.
    #[inline]
    fn deserialize(bytes: BytesMut) -> Result<Self> {
        let mut reader = bytes.reader();

        let certificate_id = Field::read_le(&mut reader)?;

        Ok(Self { certificate_id })
    }
}
