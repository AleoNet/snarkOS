use hex;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display, Formatter};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BlockHeaderHash(pub [u8; 32]);

impl BlockHeaderHash {
    pub fn new(hash: Vec<u8>) -> Self {
        let mut block_hash = [0u8; 32];
        block_hash.copy_from_slice(&hash);

        Self(block_hash)
    }

    pub const fn size() -> usize { 32 }
}

impl Display for BlockHeaderHash {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", hex::encode(self.0))
    }
}
