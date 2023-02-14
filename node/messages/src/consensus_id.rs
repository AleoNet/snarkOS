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
use narwhal_crypto::{PublicKey, Signature};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConsensusId {
    pub public_key: PublicKey,
    pub signature: Signature,
    pub last_executed_sub_dag_index: u64,
}

impl MessageTrait for Box<ConsensusId> {
    fn name(&self) -> String {
        "ConsensusId".to_string()
    }

    fn serialize<W: Write>(&self, writer: &mut W) -> Result<()> {
        bincode::serialize_into(writer, &(&self.public_key, &self.signature, self.last_executed_sub_dag_index))?;

        Ok(())
    }

    fn deserialize(bytes: BytesMut) -> Result<Self> {
        let mut reader = bytes.reader();
        let mut dst = [0; 1024];
        let num = reader.read(&mut dst).unwrap();
        let (public_key, signature, last_executed_sub_dag_index) = bincode::deserialize(&dst[..num])?;

        Ok(Box::new(ConsensusId { public_key, signature, last_executed_sub_dag_index }))
    }
}

#[cfg(test)]
mod test {
    use bytes::BufMut;
    use narwhal_crypto::KeyPair as NarwhalKeyPair;

    use super::*;

    #[test]
    fn consensus_id_serialization() {
        let mut rng = rand::thread_rng();
        let keypair = NarwhalKeyPair::new(&mut rng).unwrap();
        let public = keypair.public();
        let private = keypair.private();

        let message = &[0u8; 32];
        let signature = private.sign_bytes(message, &mut rng).unwrap();

        let id = Box::new(ConsensusId { public_key: public.clone(), signature, last_executed_sub_dag_index: 0 });
        let mut buf = BytesMut::with_capacity(128).writer();
        id.serialize(&mut buf).unwrap();
        let bytes = buf.into_inner();
        let deserialized = MessageTrait::deserialize(bytes).unwrap();
        assert_eq!(id, deserialized);
    }

    #[test]
    fn signature_serialization() {
        let mut rng = rand::thread_rng();
        let keypair = NarwhalKeyPair::new(&mut rng).unwrap();
        let private = keypair.private();

        let message = &[0u8; 32];
        let signature = private.sign_bytes(message, &mut rng).unwrap();
        let json = serde_json::to_string(&signature).unwrap();
        let deserialized: Signature = serde_json::from_str(&json).unwrap();
        assert_eq!(signature, deserialized);

        // TODO: why does the below fail?
        // let mut buf = BytesMut::with_capacity(256).writer();
        // bincode::serialize_into(&mut buf.by_ref(), &signature).unwrap();
        // let bytes = buf.into_inner();
        // let deserialized: Signature = bincode::deserialize_from(&mut bytes.reader()).unwrap();
        // assert_eq!(signature, deserialized);
    }
}
