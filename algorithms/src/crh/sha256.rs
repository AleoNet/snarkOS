use sha2::{Digest, Sha256};

pub fn sha256(data: &[u8]) -> Vec<u8> {
    Sha256::digest(&data).to_vec()
}

pub fn double_sha256(data: &[u8]) -> Vec<u8> {
    Sha256::digest(&Sha256::digest(&data)).to_vec()
}
