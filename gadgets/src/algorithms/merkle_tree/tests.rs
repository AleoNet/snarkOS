use crate::{
    algorithms::{crh::PedersenCRHGadget, merkle_tree::*},
    curves::edwards_bls12::EdwardsBlsGadget,
};
use snarkos_algorithms::{
    define_merkle_tree_parameters,
    crh::{PedersenCRH, PedersenSize},
};
use snarkos_curves::edwards_bls12::{EdwardsAffine as Edwards, Fq};
use snarkos_models::{
    gadgets::{
        algorithms::CRHGadget,
        r1cs::{ConstraintSystem, TestConstraintSystem},
        utilities::{alloc::AllocGadget, uint8::UInt8},
    },
};

#[derive(Clone, PartialEq, Eq)]
pub struct Size;
impl PedersenSize for Size {
    const NUM_WINDOWS: usize = 256;
    const WINDOW_SIZE: usize = 4;
}

type H = PedersenCRH<Edwards, Size>;
type HG = PedersenCRHGadget<Edwards, Fq, EdwardsBlsGadget>;
define_merkle_tree_parameters!(EdwardsMerkleParameters, H, 32);

type EdwardsMerkleTree = MerkleTree<EdwardsMerkleParameters>;

fn generate_merkle_tree(leaves: &[[u8; 30]], use_bad_root: bool) -> () {
    let parameters = EdwardsMerkleParameters::default();
    let tree = EdwardsMerkleTree::new(parameters.clone(), leaves).unwrap();
    let root = tree.root();
    let mut satisfied = true;
    for (i, leaf) in leaves.iter().enumerate() {
        let mut cs = TestConstraintSystem::<Fq>::new();
        let proof = tree.generate_proof(i, &leaf).unwrap();
        assert!(proof.verify(&root, &leaf).unwrap());

        // Allocate Merkle tree root
        let root = <HG as CRHGadget<H, _>>::OutputGadget::alloc(&mut cs.ns(|| format!("new_digest_{}", i)), || {
            if use_bad_root {
                Ok(<H as CRH>::Output::default())
            } else {
                Ok(root)
            }
        })
        .unwrap();

        let constraints_from_digest = cs.num_constraints();
        println!("constraints from digest: {}", constraints_from_digest);

        // Allocate Parameters for CRH
        let crh_parameters =
            <HG as CRHGadget<H, Fq>>::ParametersGadget::alloc(&mut cs.ns(|| format!("new_parameters_{}", i)), || {
                Ok(parameters.parameters())
            })
            .unwrap();

        let constraints_from_parameters = cs.num_constraints() - constraints_from_digest;
        println!("constraints from parameters: {}", constraints_from_parameters);

        // Allocate Leaf
        let leaf_g = UInt8::constant_vec(leaf);

        let constraints_from_leaf = cs.num_constraints() - constraints_from_parameters - constraints_from_digest;
        println!("constraints from leaf: {}", constraints_from_leaf);

        // Allocate Merkle tree path
        let cw =
            MerklePathGadget::<_, HG, _>::alloc(&mut cs.ns(|| format!("new_witness_{}", i)), || Ok(proof)).unwrap();

        let constraints_from_path =
            cs.num_constraints() - constraints_from_parameters - constraints_from_digest - constraints_from_leaf;
        println!("constraints from path: {}", constraints_from_path);
        let leaf_g: &[UInt8] = leaf_g.as_slice();
        cw.check_membership(
            &mut cs.ns(|| format!("new_witness_check_{}", i)),
            &crh_parameters,
            &root,
            &leaf_g,
        )
        .unwrap();
        if !cs.is_satisfied() {
            satisfied = false;
            println!("Unsatisfied constraint: {}", cs.which_is_unsatisfied().unwrap());
        }
        let setup_constraints =
            constraints_from_leaf + constraints_from_digest + constraints_from_parameters + constraints_from_path;
        println!("number of constraints: {}", cs.num_constraints() - setup_constraints);
    }

    assert!(satisfied);
}

#[test]
fn good_root_test() {
    let mut leaves = Vec::new();
    for i in 0..4u8 {
        let input = [i; 30];
        leaves.push(input);
    }
    generate_merkle_tree(&leaves, false);
}

#[should_panic]
#[test]
fn bad_root_test() {
    let mut leaves = Vec::new();
    for i in 0..4u8 {
        let input = [i; 30];
        leaves.push(input);
    }
    generate_merkle_tree(&leaves, true);
}
