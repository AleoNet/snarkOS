use crate::*;
use snarkos_algorithms::merkle_tree::MerkleTree;
use snarkos_errors::storage::StorageError;
use snarkos_models::{algorithms::MerkleParameters, objects::Transaction};
use snarkos_utilities::{
    bytes::{FromBytes, ToBytes},
    to_bytes,
};

use std::collections::HashSet;

impl<T: Transaction, P: MerkleParameters> Ledger<T, P> {
    /// Get the current commitment index
    pub fn current_cm_index(&self) -> Result<usize, StorageError> {
        match self.storage.get(COL_META, KEY_CURR_CM_INDEX.as_bytes())? {
            Some(cm_index_bytes) => Ok(bytes_to_u32(cm_index_bytes) as usize),
            None => Ok(0),
        }
    }

    /// Get the current serial number index
    pub fn current_sn_index(&self) -> Result<usize, StorageError> {
        match self.storage.get(COL_META, KEY_CURR_SN_INDEX.as_bytes())? {
            Some(sn_index_bytes) => Ok(bytes_to_u32(sn_index_bytes) as usize),
            None => Ok(0),
        }
    }

    /// Get the current memo index
    pub fn current_memo_index(&self) -> Result<usize, StorageError> {
        match self.storage.get(COL_META, KEY_CURR_MEMO_INDEX.as_bytes())? {
            Some(memo_index_bytes) => Ok(bytes_to_u32(memo_index_bytes) as usize),
            None => Ok(0),
        }
    }

    /// Get the current ledger digest
    pub fn current_digest(&self) -> Result<Vec<u8>, StorageError> {
        match self.storage.get(COL_META, KEY_CURR_DIGEST.as_bytes())? {
            Some(current_digest) => Ok(current_digest),
            None => Ok(to_bytes![self.cm_merkle_tree.read().root()].unwrap()),
        }
    }

    /// Get the set of past ledger digests
    pub fn past_digests(&self) -> Result<HashSet<Vec<u8>>, StorageError> {
        let mut digests = HashSet::new();
        for (key, _value) in self.storage.get_iter(COL_DIGEST)? {
            digests.insert(key.to_vec());
        }

        Ok(digests)
    }

    /// Get serial number index.
    pub fn get_sn_index(&self, sn_bytes: &[u8]) -> Result<Option<usize>, StorageError> {
        match self.storage.get(COL_SERIAL_NUMBER, sn_bytes)? {
            Some(sn_index_bytes) => {
                let mut sn_index = [0u8; 4];
                sn_index.copy_from_slice(&sn_index_bytes[0..4]);

                Ok(Some(u32::from_le_bytes(sn_index) as usize))
            }
            None => Ok(None),
        }
    }

    /// Get commitment index
    pub fn get_cm_index(&self, cm_bytes: &[u8]) -> Result<Option<usize>, StorageError> {
        match self.storage.get(COL_COMMITMENT, cm_bytes)? {
            Some(cm_index_bytes) => {
                let mut cm_index = [0u8; 4];
                cm_index.copy_from_slice(&cm_index_bytes[0..4]);

                Ok(Some(u32::from_le_bytes(cm_index) as usize))
            }
            None => Ok(None),
        }
    }

    /// Get memo index
    pub fn get_memo_index(&self, memo_bytes: &[u8]) -> Result<Option<usize>, StorageError> {
        match self.storage.get(COL_MEMO, memo_bytes)? {
            Some(memo_index_bytes) => {
                let mut memo_index = [0u8; 4];
                memo_index.copy_from_slice(&memo_index_bytes[0..4]);

                Ok(Some(u32::from_le_bytes(memo_index) as usize))
            }
            None => Ok(None),
        }
    }

    /// Get memo index
    pub fn build_merkle_tree(
        &self,
        additional_cms: Vec<(T::Commitment, usize)>,
    ) -> Result<MerkleTree<P>, StorageError> {
        let mut cm_and_indices = additional_cms;

        for (commitment_key, index_value) in self.storage.get_iter(COL_COMMITMENT)? {
            let commitment: T::Commitment = FromBytes::read(&commitment_key[..])?;
            let index = bytes_to_u32(index_value.to_vec()) as usize;

            cm_and_indices.push((commitment, index));
        }

        cm_and_indices.sort_by(|&(_, i), &(_, j)| i.cmp(&j));
        let commitments = cm_and_indices.into_iter().map(|(cm, _)| cm).collect::<Vec<_>>();

        Ok(MerkleTree::new(self.ledger_parameters.clone(), &commitments)?)
    }
}
