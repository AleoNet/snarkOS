use crate::{
    crh::{PedersenCRH, PedersenSize},
    define_merkle_tree_parameters,
};
use snarkos_curves::edwards_bls12::EdwardsAffine as Edwards;
use snarkos_models::curves::pairing_engine::AffineCurve;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Size;
impl PedersenSize for Size {
    const NUM_WINDOWS: usize = 256;
    const WINDOW_SIZE: usize = 4;
}

define_merkle_tree_parameters!(MTParameters, PedersenCRH<Edwards, Size>, 32);

type EdwardsMerkleTree = MerkleTree<MTParameters>;

fn generate_merkle_tree<L: ToBytes + Clone + Eq>(leaves: &[L]) -> () {
    let parameters = MTParameters::default();
    let tree = EdwardsMerkleTree::new(parameters.clone(), leaves).unwrap();
    for (i, leaf) in leaves.iter().enumerate() {
        let proof = tree.generate_proof(i, &leaf).unwrap();
        assert!(proof.verify(&tree.root(), &leaf).unwrap());
    }
}

fn bad_merkle_tree_verify<L: ToBytes + Clone + Eq>(leaves: &[L]) -> () {
    let parameters = MTParameters::default();
    let tree = EdwardsMerkleTree::new(parameters, leaves).unwrap();
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
