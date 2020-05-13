use snarkos_curves::edwards_bls12::EdwardsProjective as EdwardsBls;
use snarkos_algorithms::crh::PedersenCompressedCRH;
use snarkos_curves::bls12_377::Fr;
use snarkos_utilities::to_bytes;
use crate::define_merkle_tree_with_height;

use serde::{Deserialize, Serialize};
use std::fmt::{self, Display, Formatter};
use rand_chacha::ChaChaRng;
use once_cell::sync::Lazy;

// TODO: How should this seed be chosen?
const PRNG_SEED: [u8; 32] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];

// PRNG to instantiate the Merkle Tree parameters
pub fn prng() -> impl Rng {
    ChaChaRng::from_seed(PRNG_SEED)
}

#[macro_export]
/// Defines a Merkle Tree using the provided hash and height.
macro_rules! define_merkle_tree_with_height {
    ($struct_name:ident, $hash:ty, $height:expr) => {
        use rand::{Rng, SeedableRng};
        use snarkos_algorithms::merkle_tree::{MerkleParameters, MerkleTree};
        use snarkos_models::storage::Storage;
        use snarkos_models::algorithms::crh::CRH;
        use snarkos_utilities::bytes::{FromBytes, ToBytes};
        use std::{
            io::{Read, Result as IoResult, Write},
            path::PathBuf,
        };

        #[derive(Clone)]
        pub struct $struct_name($hash);

        impl MerkleParameters for $struct_name {
            type H = $hash;

            const HEIGHT: usize = $height;

            fn setup<R: Rng>(rng: &mut R) -> Self {
                Self(Self::H::setup(rng))
            }

            fn crh(&self) -> &Self::H {
                &self.0
            }

            fn parameters(&self) -> &<Self::H as CRH>::Parameters {
                self.crh().parameters()
            }
        }

        impl Storage for $struct_name {
            /// Store the SNARK proof to a file at the given path.
            fn store(&self, path: &PathBuf) -> IoResult<()> {
                self.0.store(path)
            }

            /// Load the SNARK proof from a file at the given path.
            fn load(path: &PathBuf) -> IoResult<Self> {
                Ok(Self(<Self as MerkleParameters>::H::load(path)?))
            }
        }

        impl Default for $struct_name {
            fn default() -> Self {
                Self(<Self as MerkleParameters>::H::setup(&mut $crate::pedersen_merkle_tree::prng()))
            }
        }

        impl ToBytes for $struct_name {
            #[inline]
            fn write<W: Write>(&self, mut writer: W) -> IoResult<()> {
                self.0.write(&mut writer)
            }
        }

        impl FromBytes for $struct_name {
            #[inline]
            fn read<R: Read>(mut reader: R) -> IoResult<Self> {
                let crh: <Self as MerkleParameters>::H = FromBytes::read(&mut reader)?;

                Ok(Self(crh))
            }
        }
    };
}

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

define_merkle_tree_with_height!(MaskedMerkleTreeParameters, MerkleTreeCRH, TREE_HEIGHT);

/// A Merkle Tree instantiated with the Masked Pedersen hasher over BLS12-377
pub type EdwardsMaskedMerkleTree = MerkleTree<MaskedMerkleTreeParameters>;

/// Lazily evaluated parameters for the Masked Merkle tree
pub static PARAMS: Lazy<MaskedMerkleTreeParameters> =
    Lazy::new(|| MaskedMerkleTreeParameters::setup(&mut prng()));


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
    let tree = MerkleTree::new(PARAMS.clone(), hashes).expect("could not create merkle tree");
    tree.root()
}

/// Calculates the root of the Merkle tree using a Pedersen Hash instantiated with a PRNG and the
/// base layer hashes leaved
pub fn pedersen_merkle_root_hash_with_leaves(hashes: &[Vec<u8>]) -> (Fr, Vec<Fr>) {
    let tree = MerkleTree::new(PARAMS.clone(), hashes).expect("could not create merkle tree");
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
