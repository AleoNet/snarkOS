use snarkos_algorithms::{crh::PedersenCompressedCRH, define_merkle_tree_parameters, merkle_tree::prng};
use snarkos_curves::{bls12_377::Fr, edwards_bls12::EdwardsProjective as EdwardsBls};
use snarkos_utilities::to_bytes;

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display, Formatter};

// Do not leak the type
mod window {
    use snarkos_algorithms::crh::PedersenSize;

    #[derive(Clone, PartialEq, Eq, Hash)]
    pub struct TwoToOneWindow;

    impl PedersenSize for TwoToOneWindow {
        const NUM_WINDOWS: usize = 4;
        const WINDOW_SIZE: usize = 128;
    }
}

pub type MerkleTreeCRH = PedersenCompressedCRH<EdwardsBls, window::TwoToOneWindow>;

// We instantiate the tree here with height = 9. This may change in the future.
const TREE_HEIGHT: usize = 9;

define_merkle_tree_parameters!(MaskedMerkleTreeParameters, MerkleTreeCRH, TREE_HEIGHT);

/// A Merkle Tree instantiated with the Masked Pedersen hasher over BLS12-377
pub type EdwardsMaskedMerkleTree = MerkleTree<MaskedMerkleTreeParameters>;

/// Lazily evaluated parameters for the Masked Merkle tree
pub static PARAMS: Lazy<MaskedMerkleTreeParameters> = Lazy::new(|| MaskedMerkleTreeParameters::setup(&mut prng()));

/// A Pedersen Merkle Root Hash
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PedersenMerkleRootHash(pub [u8; 32]);

impl PedersenMerkleRootHash {
    pub const fn size() -> usize {
        32
    }
}

impl Display for PedersenMerkleRootHash {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", hex::encode(self.0))
    }
}

/// Calculates the root of the Merkle tree using a Pedersen Hash instantiated with a PRNG
/// and returns it serialized
pub fn pedersen_merkle_root(hashes: &[Vec<u8>]) -> PedersenMerkleRootHash {
    pedersen_merkle_root_hash(hashes).into()
}

/// Calculates the root of the Merkle tree using a Pedersen Hash instantiated with a PRNG
pub fn pedersen_merkle_root_hash(hashes: &[Vec<u8>]) -> Fr {
    let tree = EdwardsMaskedMerkleTree::new(PARAMS.clone(), hashes).expect("could not create merkle tree");
    tree.root()
}

/// Calculates the root of the Merkle tree using a Pedersen Hash instantiated with a PRNG and the
/// base layer hashes leaved
pub fn pedersen_merkle_root_hash_with_leaves(hashes: &[Vec<u8>]) -> (Fr, Vec<Fr>) {
    let tree = EdwardsMaskedMerkleTree::new(PARAMS.clone(), hashes).expect("could not create merkle tree");
    (tree.root(), tree.leaves_hashed())
}

impl From<Fr> for PedersenMerkleRootHash {
    fn from(src: Fr) -> PedersenMerkleRootHash {
        let root_bytes = to_bytes![src].expect("could not convert merkle root to bytes");
        let mut pedersen_merkle_root_bytes = [0u8; 32];
        pedersen_merkle_root_bytes[..].copy_from_slice(&root_bytes);
        PedersenMerkleRootHash(pedersen_merkle_root_bytes)
    }
}
