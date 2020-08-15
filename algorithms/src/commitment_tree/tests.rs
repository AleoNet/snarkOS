use crate::{
    commitment::PedersenCompressedCommitment,
    commitment_tree::*,
    crh::{BoweHopwoodPedersenCompressedCRH, PedersenSize},
};
use snarkos_curves::edwards_bls12::EdwardsProjective as EdwardsBls;
use snarkos_models::algorithms::{CommitmentScheme, CRH};
use snarkos_utilities::{
    bytes::{FromBytes, ToBytes},
    rand::UniformRand,
    to_bytes,
};

use rand::{Rng, SeedableRng};
use rand_xorshift::XorShiftRng;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CRHWindow;

impl PedersenSize for CRHWindow {
    const NUM_WINDOWS: usize = 16;
    const WINDOW_SIZE: usize = 32;
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CommitmentWindow;

impl PedersenSize for CommitmentWindow {
    const NUM_WINDOWS: usize = 8;
    const WINDOW_SIZE: usize = 32;
}

pub type H = BoweHopwoodPedersenCompressedCRH<EdwardsBls, CRHWindow>;
pub type C = PedersenCompressedCommitment<EdwardsBls, CommitmentWindow>;
pub type CM = CommitmentMerklePath<C, H>;

/// Generates a valid Merkle tree and verifies the Merkle path witness for each leaf.
fn generate_merkle_tree<C: CommitmentScheme, H: CRH, R: Rng>(
    commitment: &C,
    crh: &H,
    rng: &mut R,
) -> CommitmentMerkleTree<C, H> {
    let default = <C as CommitmentScheme>::Output::default();
    let mut leaves = [default.clone(), default.clone(), default.clone(), default];

    for i in 0..4 {
        let leaf_input: [u8; 32] = rng.gen();
        let randomness = <C as CommitmentScheme>::Randomness::rand(rng);

        let leaf = commitment.commit(&leaf_input, &randomness).unwrap();
        leaves[i] = leaf;
    }

    CommitmentMerkleTree::new(crh.clone(), &leaves).unwrap()
}

#[test]
fn commitment_tree_good_root_test() {
    let rng = &mut XorShiftRng::seed_from_u64(1231275789u64);

    let commitment = C::setup(rng);
    let crh = H::setup(rng);

    let merkle_tree = generate_merkle_tree(&commitment, &crh, rng);

    for leaf in merkle_tree.leaves().iter() {
        let proof = merkle_tree.generate_proof(&leaf).unwrap();
        assert!(proof.verify(&crh, &merkle_tree.root(), &leaf).unwrap());
    }
}

#[should_panic]
#[test]
fn commitment_tree_bad_root_test() {
    let rng = &mut XorShiftRng::seed_from_u64(1231275789u64);

    let commitment = C::setup(rng);
    let crh = H::setup(rng);

    let merkle_tree = generate_merkle_tree(&commitment, &crh, rng);

    for leaf in merkle_tree.leaves().iter() {
        let proof = merkle_tree.generate_proof(&leaf).unwrap();
        assert!(proof.verify(&crh, &<H as CRH>::Output::default(), &leaf).unwrap());
    }
}

#[test]
fn test_serialize_commitment_path() {
    let rng = &mut XorShiftRng::seed_from_u64(1231275789u64);

    let commitment = C::setup(rng);
    let crh = H::setup(rng);

    let merkle_tree = generate_merkle_tree(&commitment, &crh, rng);

    for leaf in merkle_tree.leaves().iter() {
        let proof = merkle_tree.generate_proof(&leaf).unwrap();

        let proof_bytes = to_bytes![proof].unwrap();
        let recovered_proof = CM::read(&proof_bytes[..]).unwrap();

        assert!(proof == recovered_proof);

        assert!(recovered_proof.verify(&crh, &merkle_tree.root(), &leaf).unwrap());
    }
}
