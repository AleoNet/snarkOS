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
