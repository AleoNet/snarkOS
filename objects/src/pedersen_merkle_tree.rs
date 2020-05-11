use snarkos_algorithms::merkle_tree::{MerkleParameters, MerkleTree};
use snarkos_utilities::{bytes::ToBytes, to_bytes};

use serde::{Deserialize, Serialize};
use std::fmt::{self, Display, Formatter};

use rand::{Rng, SeedableRng};
use rand_chacha::ChaChaRng;

// TODO: How should this seed be chosen?
const PRNG_SEED: [u8; 32] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];

// PRNG to instantiate the Merkle Tree parameters
fn prng() -> impl Rng {
    ChaChaRng::from_seed(PRNG_SEED)
}

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
pub fn pedersen_merkle_root(hashes: &[Vec<u8>]) -> PedersenMerkleRootHash {
    pedersen_merkle_root_hash(hashes).into()
}

use snarkos_curves::bls12_377::Fr;
use once_cell::sync::Lazy;

pub static PARAMS: Lazy<mtree::CommitmentMerkleParameters> = Lazy::new(|| {
    mtree::CommitmentMerkleParameters::setup(&mut prng())
});

/// Calculates the root of the Merkle tree using a Pedersen Hash instantiated with a PRNG
pub fn pedersen_merkle_root_hash(hashes: &[Vec<u8>]) -> Fr {
    let params = mtree::CommitmentMerkleParameters::setup(&mut prng());
    let tree = MerkleTree::new(params, hashes).expect("could not create merkle tree");
    tree.root()
}

impl From<Fr> for PedersenMerkleRootHash {
    fn from(src: Fr) -> PedersenMerkleRootHash {
        let root_bytes = to_bytes![src].expect("could not convert merkle root to bytes");
        let mut pedersen_merkle_root_bytes = [0u8; 32];
        pedersen_merkle_root_bytes[..].copy_from_slice(&root_bytes);
        PedersenMerkleRootHash(pedersen_merkle_root_bytes)
    }
}

pub type EdwardsMaskedMerkleTree = MerkleTree<mtree::CommitmentMerkleParameters>;

pub mod mtree {
    use rand::Rng;
    use snarkos_algorithms::{
        crh::{PedersenCompressedCRH, PedersenSize},
        merkle_tree::MerkleParameters,
    };
    use snarkos_curves::edwards_bls12::EdwardsProjective as EdwardsBls;
    use snarkos_models::{algorithms::crh::CRH, storage::Storage};
    use snarkos_utilities::bytes::{FromBytes, ToBytes};
    use std::{
        io::{Read, Result as IoResult, Write},
        path::PathBuf,
    };

    pub type MerkleTreeCRH = PedersenCompressedCRH<EdwardsBls, TwoToOneWindow>;

    pub type H = MerkleTreeCRH;

    #[derive(Clone, PartialEq, Eq)]
    pub struct CommitmentMerkleParameters(H);

    impl Default for CommitmentMerkleParameters {
        fn default() -> Self {
            let mut rng = rand::thread_rng();
            Self(H::setup(&mut rng))
        }
    }

    impl MerkleParameters for CommitmentMerkleParameters {
        type H = H;

        const HEIGHT: usize = 32;

        fn setup<R: Rng>(rng: &mut R) -> Self {
            Self(H::setup(rng))
        }

        fn crh(&self) -> &Self::H {
            &self.0
        }

        fn parameters(&self) -> &<<Self as MerkleParameters>::H as CRH>::Parameters {
            self.crh().parameters()
        }
    }

    impl Storage for CommitmentMerkleParameters {
        /// Store the SNARK proof to a file at the given path.
        fn store(&self, path: &PathBuf) -> IoResult<()> {
            self.0.store(path)
        }

        /// Load the SNARK proof from a file at the given path.
        fn load(path: &PathBuf) -> IoResult<Self> {
            Ok(Self(H::load(path)?))
        }
    }

    impl ToBytes for CommitmentMerkleParameters {
        #[inline]
        fn write<W: Write>(&self, mut writer: W) -> IoResult<()> {
            self.0.write(&mut writer)
        }
    }

    impl FromBytes for CommitmentMerkleParameters {
        #[inline]
        fn read<R: Read>(mut reader: R) -> IoResult<Self> {
            let crh: H = FromBytes::read(&mut reader)?;

            Ok(Self(crh))
        }
    }

    #[derive(Clone, PartialEq, Eq, Hash)]
    pub struct TwoToOneWindow;

    impl PedersenSize for TwoToOneWindow {
        const NUM_WINDOWS: usize = 4;
        const WINDOW_SIZE: usize = 128;
    }
}
