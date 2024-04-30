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

use snarkvm::{
    console::{
        account::{Address, Signature},
        network::Network,
        types::Field,
    },
    prelude::{error, FromBytes, IoResult, Read, ToBytes, Write},
};

use std::{collections::HashMap, ops::Deref};

/// The recently-signed batch proposals.
/// A map of `address` to (`round`, `batch ID`, `signature`).
#[derive(Clone, PartialEq, Eq)]
pub struct SignedProposals<N: Network>(pub HashMap<Address<N>, (u64, Field<N>, Signature<N>)>);

impl<N: Network> SignedProposals<N> {
    /// Ensure that every signed proposal is associated with the `expected_signer`.
    pub fn is_valid(&self, expected_signer: Address<N>) -> bool {
        self.0.iter().all(|(_, (_, _, signature))| signature.to_address() == expected_signer)
    }
}

impl<N: Network> ToBytes for SignedProposals<N> {
    fn write_le<W: Write>(&self, mut writer: W) -> IoResult<()> {
        // Write the number of signed proposals.
        u32::try_from(self.0.len()).map_err(error)?.write_le(&mut writer)?;
        // Serialize the signed proposals.
        for (address, (round, batch_id, signature)) in &self.0 {
            // Write the address.
            address.write_le(&mut writer)?;
            // Write the round.
            round.write_le(&mut writer)?;
            // Write the batch id.
            batch_id.write_le(&mut writer)?;
            // Write the signature.
            signature.write_le(&mut writer)?;
        }

        Ok(())
    }
}

impl<N: Network> FromBytes for SignedProposals<N> {
    fn read_le<R: Read>(mut reader: R) -> IoResult<Self> {
        // Read the number of signed proposals.
        let num_signed_proposals = u32::read_le(&mut reader)?;
        // Deserialize the signed proposals.
        let mut signed_proposals = HashMap::default();
        for _ in 0..num_signed_proposals {
            // Read the address.
            let address = FromBytes::read_le(&mut reader)?;
            // Read the round.
            let round = FromBytes::read_le(&mut reader)?;
            // Read the batch id.
            let batch_id = FromBytes::read_le(&mut reader)?;
            // Read the signature.
            let signature = FromBytes::read_le(&mut reader)?;
            // Insert the signed proposal.
            signed_proposals.insert(address, (round, batch_id, signature));
        }

        Ok(Self(signed_proposals))
    }
}

impl<N: Network> Deref for SignedProposals<N> {
    type Target = HashMap<Address<N>, (u64, Field<N>, Signature<N>)>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<N: Network> Default for SignedProposals<N> {
    /// Initializes a new instance of the signed proposals.
    fn default() -> Self {
        Self(Default::default())
    }
}
