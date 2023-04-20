// Copyright (C) 2019-2023 Aleo Systems Inc.
// This file is part of the snarkOS library.

// The snarkOS library is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// The snarkOS library is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with the snarkOS library. If not, see <https://www.gnu.org/licenses/>.

use super::*;
use fastcrypto::{
    bls12381::min_sig::{BLS12381PublicKey, BLS12381Signature},
    traits::ToFromBytes,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConsensusId {
    pub public_key: BLS12381PublicKey,
    pub signature: BLS12381Signature,
}

impl MessageTrait for Box<ConsensusId> {
    fn name(&self) -> String {
        "ConsensusId".to_string()
    }

    fn serialize<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_all(self.public_key.as_bytes())?;
        writer.write_all(self.signature.as_bytes())?;

        Ok(())
    }

    fn deserialize(bytes: BytesMut) -> Result<Self> {
        let (public_key, signature) = bincode::deserialize_from(&mut bytes.reader())?;

        Ok(Box::new(ConsensusId { public_key, signature }))
    }
}
