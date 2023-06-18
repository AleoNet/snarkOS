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

use crate::helpers::{Transmission, TransmissionID};
use snarkos_node_messages::Data;
use snarkvm::{
    console::prelude::*,
    prelude::{Address, Field, PrivateKey, Signature},
};

use std::{collections::HashMap, sync::Arc};
use time::OffsetDateTime;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BatchCertificate<N: Network> {
    /// The batch ID.
    pub batch_id: Field<N>,
    /// The `signatures` of the batch ID from the committee.
    pub signatures: Vec<Signature<N>>,
}

impl<N: Network> FromBytes for BatchCertificate<N> {
    /// Reads the batch certificate from the buffer.
    fn read_le<R: Read>(mut reader: R) -> IoResult<Self> {
        // Read the version.
        let version = u8::read_le(&mut reader)?;
        // Ensure the version is valid.
        if version != 0 {
            return Err(error("Invalid batch version"));
        }

        // Read the batch ID.
        let batch_id = Field::read_le(&mut reader)?;
        // Read the number of signatures.
        let num_signatures = u32::read_le(&mut reader)?;
        // Read the signatures.
        let mut signatures = Vec::new();
        for _ in 0..num_signatures {
            signatures.push(Signature::read_le(&mut reader)?);
        }

        Ok(Self { batch_id, signatures })
    }
}

impl<N: Network> ToBytes for BatchCertificate<N> {
    /// Writes the batch certificate to the buffer.
    fn write_le<W: Write>(&self, mut writer: W) -> IoResult<()> {
        // Write the version.
        0u8.write_le(&mut writer)?;
        // Write the batch ID.
        self.batch_id.write_le(&mut writer)?;
        // Write the number of signatures.
        u32::try_from(self.signatures.len()).map_err(|e| error(e.to_string()))?.write_le(&mut writer)?;
        // Write the signatures.
        for signature in &self.signatures {
            signature.write_le(&mut writer)?;
        }
        Ok(())
    }
}
