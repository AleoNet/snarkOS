use crate::*;

use snarkos_algorithms::merkle_tree::MerkleParameters;
use snarkos_errors::storage::StorageError;
use snarkos_objects::dpc::Transaction;
use snarkos_utilities::bytes::FromBytes;

use std::collections::HashSet;

impl<T: Transaction, P: MerkleParameters> BlockStorage<T, P> {
    /// Get the genesis commitment
    pub fn genesis_cm(&self) -> Result<T::Commitment, StorageError> {
        match self.storage.get(COL_META, KEY_GENESIS_CM.as_bytes())? {
            Some(cm_bytes) => Ok(FromBytes::read(&cm_bytes[..])?),
            None => Err(StorageError::Message("Missing genesis cm".to_string())),
        }
    }

    /// Get the genesis serial number
    pub fn genesis_sn(&self) -> Result<T::SerialNumber, StorageError> {
        match self.storage.get(COL_META, KEY_GENESIS_SN.as_bytes())? {
            Some(genesis_sn_bytes) => Ok(FromBytes::read(&genesis_sn_bytes[..])?),
            None => Err(StorageError::Message("Missing genesis sn".to_string())),
        }
    }

    /// Get the genesis memo
    pub fn genesis_memo(&self) -> Result<T::Memorandum, StorageError> {
        match self.storage.get(COL_META, KEY_GENESIS_MEMO.as_bytes())? {
            Some(genesis_memo_bytes) => Ok(FromBytes::read(&genesis_memo_bytes[..])?),
            None => Err(StorageError::Message("Missing genesis memo".to_string())),
        }
    }

    /// Get the genesis predicate vk bytes
    pub fn genesis_pred_vk_bytes(&self) -> Result<Vec<u8>, StorageError> {
        match self.storage.get(COL_META, KEY_GENESIS_PRED_VK.as_bytes())? {
            Some(genesis_pred_vk_bytes) => Ok(genesis_pred_vk_bytes),
            None => Err(StorageError::Message("Missing genesis predicate vk".to_string())),
        }
    }

    /// Get the genesis address pair bytes
    pub fn genesis_address_pair_bytes(&self) -> Result<Vec<u8>, StorageError> {
        match self.storage.get(COL_META, KEY_GENESIS_ADDRESS_PAIR.as_bytes())? {
            Some(genesis_address_pair_bytes) => Ok(genesis_address_pair_bytes),
            None => Err(StorageError::Message("Missing genesis address pair".to_string())),
        }
    }

    /// Get the current commitment index
    pub fn current_cm_index(&self) -> Result<usize, StorageError> {
        match self.storage.get(COL_META, KEY_CURR_CM_INDEX.as_bytes())? {
            Some(cm_index_bytes) => {
                let mut curr_cm_index = [0u8; 4];
                curr_cm_index.copy_from_slice(&cm_index_bytes[0..4]);

                Ok(u32::from_le_bytes(curr_cm_index) as usize)
            }
            None => Err(StorageError::Message("Missing current cm index".to_string())),
        }
    }

    /// Get the current serial number index
    pub fn current_sn_index(&self) -> Result<usize, StorageError> {
        match self.storage.get(COL_META, KEY_CURR_SN_INDEX.as_bytes())? {
            Some(sn_index_bytes) => Ok(bytes_to_u32(sn_index_bytes) as usize),
            None => Err(StorageError::Message("Missing current sn index".to_string())),
        }
    }

    /// Get the current memo index
    pub fn current_memo_index(&self) -> Result<usize, StorageError> {
        match self.storage.get(COL_META, KEY_CURR_MEMO_INDEX.as_bytes())? {
            Some(memo_index_bytes) => Ok(bytes_to_u32(memo_index_bytes) as usize),
            None => Err(StorageError::Message("Missing current memo index".to_string())),
        }
    }

    /// Get the current ledger digest
    pub fn current_digest(&self) -> Result<Vec<u8>, StorageError> {
        match self.storage.get(COL_META, KEY_CURR_DIGEST.as_bytes())? {
            Some(current_digest) => Ok(current_digest),
            None => Err(StorageError::Message("Missing current digest".to_string())),
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
}
