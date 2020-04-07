use crate::{
    algorithms::{crh::PedersenCRHGadget, merkle_tree::*},
    curves::edwards_bls12::EdwardsBlsGadget,
};
use snarkos_algorithms::{
    crh::{PedersenCRH, PedersenSize},
    merkle_tree::{MerkleParameters, MerkleTree},
};
use snarkos_curves::edwards_bls12::{EdwardsAffine as Edwards, Fq};
use snarkos_models::{
    algorithms::CRH,
    gadgets::{
        algorithms::CRHGadget,
        r1cs::{ConstraintSystem, TestConstraintSystem},
        utilities::{alloc::AllocGadget, uint8::UInt8},
    },
};
use snarkos_utilities::storage::Storage;

use rand::{Rng, SeedableRng};
use rand_xorshift::XorShiftRng;
use std::{io::Result as IoResult, path::PathBuf, rc::Rc};

#[derive(Clone)]
pub(super) struct Size;
impl PedersenSize for Size {
    const NUM_WINDOWS: usize = 256;
    const WINDOW_SIZE: usize = 4;
}

type H = PedersenCRH<Edwards, Size>;
type HG = PedersenCRHGadget<Edwards, Fq, EdwardsBlsGadget>;

#[derive(Clone)]
struct EdwardsMerkleParameters(H);
impl MerkleParameters for EdwardsMerkleParameters {
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

impl Storage for EdwardsMerkleParameters {
    /// Store the SNARK proof to a file at the given path.
    fn store(&self, path: &PathBuf) -> IoResult<()> {
        self.0.store(path)
    }

    /// Load the SNARK proof from a file at the given path.
    fn load(path: &PathBuf) -> IoResult<Self> {
        Ok(Self(H::load(path)?))
    }
}
impl Default for EdwardsMerkleParameters {
    fn default() -> Self {
        let rng = &mut XorShiftRng::seed_from_u64(9174123u64);
        Self(H::setup(rng))
    }
}

type EdwardsMerkleTree = MerkleTree<EdwardsMerkleParameters>;

fn generate_merkle_tree(leaves: &[[u8; 30]], use_bad_root: bool) -> () {
    let mut rng = XorShiftRng::seed_from_u64(9174123u64);

    let parameters = EdwardsMerkleParameters::setup(&mut rng);
    let crh = Rc::new(parameters.0);
    let tree = EdwardsMerkleTree::new(&parameters, leaves).unwrap();
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
                Ok(crh_parameters.clone())
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
