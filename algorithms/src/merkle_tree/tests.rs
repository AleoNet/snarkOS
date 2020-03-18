use crate::{
    crh::{PedersenCRH, PedersenSize},
    merkle_tree::*,
};
use snarkos_curves::edwards_bls12::EdwardsAffine as Edwards;
use snarkos_models::{algorithms::CRH, curves::pairing_engine::AffineCurve};
use snarkos_utilities::bytes::ToBytes;

use rand::SeedableRng;
use rand_xorshift::XorShiftRng;

#[derive(Clone)]
pub(crate) struct Size;
impl PedersenSize for Size {
    const NUM_WINDOWS: usize = 256;
    const WINDOW_SIZE: usize = 4;
}

#[derive(Clone)]
pub(crate) struct MTParameters(PedersenCRH<Edwards, Size>);
impl MerkleParameters for MTParameters {
    type H = PedersenCRH<Edwards, Size>;

    const HEIGHT: usize = 32;

    fn crh(&self) -> &Self::H {
        &self.0
    }
}
impl Default for MTParameters {
    fn default() -> Self {
        let mut rng = XorShiftRng::seed_from_u64(9174123u64);
        Self(PedersenCRH::<Edwards, Size>::setup(&mut rng))
    }
}

type EdwardsMerkleTree = MerkleTree<MTParameters>;

fn generate_merkle_tree<L: ToBytes + Clone + Eq>(leaves: &[L]) -> () {
    let tree = EdwardsMerkleTree::new(&leaves).unwrap();
    for (i, leaf) in leaves.iter().enumerate() {
        let proof = tree.generate_proof(i, &leaf).unwrap();
        assert!(proof.verify(&tree.root(), &leaf).unwrap());
    }
}

fn bad_merkle_tree_verify<L: ToBytes + Clone + Eq>(leaves: &[L]) -> () {
    let tree = EdwardsMerkleTree::new(&leaves).unwrap();
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
