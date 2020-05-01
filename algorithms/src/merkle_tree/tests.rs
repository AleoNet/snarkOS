use crate::{
    crh::{PedersenCRH, PedersenSize},
    merkle_tree::*,
};
use snarkos_curves::edwards_bls12::EdwardsAffine as Edwards;
use snarkos_models::{algorithms::CRH, curves::pairing_engine::AffineCurve};
use snarkos_utilities::{
    bytes::{FromBytes, ToBytes},
    storage::Storage,
};

use rand::{Rng, SeedableRng};
use rand_xorshift::XorShiftRng;
use std::{
    io::{Read, Result as IoResult, Write},
    path::PathBuf,
};

#[derive(Clone)]
pub(crate) struct Size;
impl PedersenSize for Size {
    const NUM_WINDOWS: usize = 256;
    const WINDOW_SIZE: usize = 4;
}

type H = PedersenCRH<Edwards, Size>;

#[derive(Clone)]
pub(crate) struct MTParameters(PedersenCRH<Edwards, Size>);
impl MerkleParameters for MTParameters {
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

impl Default for MTParameters {
    fn default() -> Self {
        let mut rng = XorShiftRng::seed_from_u64(9174123u64);
        Self(H::setup(&mut rng))
    }
}

impl Storage for MTParameters {
    /// Store the SNARK proof to a file at the given path.
    fn store(&self, path: &PathBuf) -> IoResult<()> {
        self.0.store(path)
    }

    /// Load the SNARK proof from a file at the given path.
    fn load(path: &PathBuf) -> IoResult<Self> {
        Ok(Self(H::load(path)?))
    }
}

impl ToBytes for MTParameters {
    #[inline]
    fn write<W: Write>(&self, mut writer: W) -> IoResult<()> {
        self.0.write(&mut writer)
    }
}

impl FromBytes for MTParameters {
    #[inline]
    fn read<R: Read>(mut reader: R) -> IoResult<Self> {
        let crh: H = FromBytes::read(&mut reader)?;

        Ok(Self(crh))
    }
}

type EdwardsMerkleTree = MerkleTree<MTParameters>;

fn generate_merkle_tree<L: ToBytes + Clone + Eq>(leaves: &[L]) -> () {
    let mut rng = XorShiftRng::seed_from_u64(9174123u64);

    let parameters = MTParameters::setup(&mut rng);
    let tree = EdwardsMerkleTree::new(&parameters, leaves).unwrap();
    for (i, leaf) in leaves.iter().enumerate() {
        let proof = tree.generate_proof(i, &leaf).unwrap();
        assert!(proof.verify(&tree.root(), &leaf).unwrap());
    }
}

fn bad_merkle_tree_verify<L: ToBytes + Clone + Eq>(leaves: &[L]) -> () {
    let mut rng = XorShiftRng::seed_from_u64(9174123u64);

    let parameters = MTParameters::setup(&mut rng);
    let tree = EdwardsMerkleTree::new(&parameters, leaves).unwrap();
    for (i, leaf) in leaves.iter().enumerate() {
        let proof = tree.generate_proof(i, &leaf).unwrap();
        assert!(proof.verify(&Edwards::zero(), &leaf).unwrap());
    }
}

#[test]
fn good_root_test() {
    let mut leaves = vec![];
    for i in 0..4u8 {
        leaves.push([i, i, i, i, i, i, i, i]);
    }
    generate_merkle_tree(&leaves);

    let mut leaves = vec![];
    for i in 0..15u8 {
        leaves.push([i, i, i, i, i, i, i, i]);
    }
    generate_merkle_tree(&leaves);
}

#[should_panic]
#[test]
fn bad_root_test() {
    let mut leaves = vec![];
    for i in 0..4u8 {
        leaves.push([i, i, i, i, i, i, i, i]);
    }
    generate_merkle_tree(&leaves);

    let mut leaves = vec![];
    for i in 0..15u8 {
        leaves.push([i, i, i, i, i, i, i, i]);
    }
    bad_merkle_tree_verify(&leaves);
}
