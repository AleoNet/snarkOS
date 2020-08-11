use crate::{
    algorithms::{
        commitment::PedersenCompressedCommitmentGadget,
        commitment_tree::*,
        crh::BoweHopwoodPedersenCompressedCRHGadget,
    },
    curves::edwards_bls12::EdwardsBlsGadget,
};
use snarkos_algorithms::{
    commitment::PedersenCompressedCommitment,
    commitment_tree::*,
    crh::{BoweHopwoodPedersenCompressedCRH, PedersenSize},
};
use snarkos_curves::{bls12_377::Fr, edwards_bls12::EdwardsProjective as EdwardsBls};
use snarkos_models::{
    algorithms::{CommitmentScheme, CRH},
    curves::Field,
    gadgets::{
        algorithms::{CRHGadget, CommitmentGadget},
        r1cs::{ConstraintSystem, TestConstraintSystem},
        utilities::alloc::AllocGadget,
    },
};
use snarkos_utilities::rand::UniformRand;

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
pub type HG = BoweHopwoodPedersenCompressedCRHGadget<EdwardsBls, Fr, EdwardsBlsGadget>;

pub type C = PedersenCompressedCommitment<EdwardsBls, CommitmentWindow>;
pub type CG = PedersenCompressedCommitmentGadget<EdwardsBls, Fr, EdwardsBlsGadget>;

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

fn commitment_tree_test<
    C: CommitmentScheme,
    H: CRH,
    CG: CommitmentGadget<C, F>,
    HG: CRHGadget<H, F>,
    F: Field,
    R: Rng,
>(
    use_bad_root: bool,
    rng: &mut R,
) {
    let commitment = C::setup(rng);
    let crh = H::setup(rng);

    let merkle_tree = generate_merkle_tree(&commitment, &crh, rng);

    let mut satisfied = true;
    for (i, leaf) in merkle_tree.leaves().iter().enumerate() {
        let proof = merkle_tree.generate_proof(&leaf).unwrap();
        assert!(proof.verify(&merkle_tree.root(), &leaf).unwrap());

        let mut num_constraints = 0;

        let mut cs = TestConstraintSystem::<F>::new();

        // Allocate Merkle tree root
        let root_gadget =
            <HG as CRHGadget<H, _>>::OutputGadget::alloc(&mut cs.ns(|| format!("new_root_{}", i)), || {
                if use_bad_root {
                    Ok(<H as CRH>::Output::default())
                } else {
                    Ok(merkle_tree.root())
                }
            })
            .unwrap();

        println!("constraints from root: {}", cs.num_constraints() - num_constraints);
        num_constraints = cs.num_constraints();

        // Allocate Parameters for CRH
        let crh_parameters = <HG as CRHGadget<_, _>>::ParametersGadget::alloc(
            &mut cs.ns(|| format!("new_crh_parameters_{}", i)),
            || Ok(crh.parameters()),
        )
        .unwrap();

        println!(
            "constraints from crh parameters: {}",
            cs.num_constraints() - num_constraints
        );
        num_constraints = cs.num_constraints();

        // Allocate Leaf
        let leaf_gadget =
            <CG as CommitmentGadget<_, _>>::OutputGadget::alloc(&mut cs.ns(|| format!("leaf_{}", i)), || Ok(leaf))
                .unwrap();

        println!("constraints from leaf: {}", cs.num_constraints() - num_constraints);
        num_constraints = cs.num_constraints();

        // Allocate Merkle tree path
        let commitment_witness =
            CommitmentMerklePathGadget::<_, _, CG, HG, _>::alloc(&mut cs.ns(|| format!("new_witness_{}", i)), || {
                Ok(proof)
            })
            .unwrap();

        println!("constraints from path: {}", cs.num_constraints() - num_constraints);
        num_constraints = cs.num_constraints();

        commitment_witness
            .check_membership(
                &mut cs.ns(|| format!("new_witness_check_{}", i)),
                &crh_parameters,
                &root_gadget,
                &leaf_gadget,
            )
            .unwrap();

        if !cs.is_satisfied() {
            satisfied = false;
            println!("Unsatisfied constraint: {}", cs.which_is_unsatisfied().unwrap());
        }

        println!(
            "constraints from witness_check: {}",
            cs.num_constraints() - num_constraints
        );
    }

    assert!(satisfied);
}

#[test]
fn commitment_tree_good_root_test() {
    let rng = &mut XorShiftRng::seed_from_u64(1231275789u64);

    commitment_tree_test::<C, H, CG, HG, _, _>(false, rng);
}

#[should_panic]
#[test]
fn commitment_tree_bad_root_test() {
    let rng = &mut XorShiftRng::seed_from_u64(1231275789u64);

    commitment_tree_test::<C, H, CG, HG, _, _>(true, rng);
}
