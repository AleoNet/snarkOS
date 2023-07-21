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
pub struct CertificateResponse<N: Network> {
    pub certificate: BatchCertificate<N>,
}

impl<N: Network> CertificateResponse<N> {
    /// Initializes a new certificate response event.
    pub fn new(certificate: BatchCertificate<N>) -> Self {
        Self { certificate }
    }
}

impl<N: Network> From<BatchCertificate<N>> for CertificateResponse<N> {
    /// Initializes a new certificate response event.
    fn from(certificate: BatchCertificate<N>) -> Self {
        Self::new(certificate)
    }
}

impl<N: Network> EventTrait for CertificateResponse<N> {
    /// Returns the event name.
    #[inline]
    fn name(&self) -> &'static str {
        "CertificateResponse"
    }

    /// Serializes the event into the buffer.
    #[inline]
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_all(&self.certificate.to_bytes_le()?)?;
        Ok(())
    }

    /// Deserializes the given buffer into an event.
    #[inline]
    fn deserialize(bytes: BytesMut) -> Result<Self> {
        let mut reader = bytes.reader();

        let certificate = BatchCertificate::read_le(&mut reader)?;

        Ok(Self { certificate })
    }
}
