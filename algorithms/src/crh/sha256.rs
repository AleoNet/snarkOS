use sha2::{Digest, Sha256};

pub fn sha256(data: &[u8]) -> Vec<u8> {
    Sha256::digest(&data).to_vec()
}

pub fn double_sha256(data: &[u8]) -> Vec<u8> {
    Sha256::digest(&Sha256::digest(&data)).to_vec()
}

pub fn sha256d_to_u64(data: &[u8]) -> u64 {
    let hash_slice = double_sha256(data);
    let mut hash = [0u8; 8];
    hash[..].copy_from_slice(&hash_slice[..8]);
    u64::from_le_bytes(hash)
}
