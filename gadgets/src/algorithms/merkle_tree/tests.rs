use crate::{
    algorithms::{crh::PedersenCompressedCRHGadget, merkle_tree::*},
    curves::edwards_bls12::EdwardsBlsGadget,
};
use snarkos_algorithms::{
    crh::{PedersenCompressedCRH, PedersenSize},
    merkle_tree::{MerkleParameters, MerkleTree},
};
use snarkos_curves::edwards_bls12::{EdwardsProjective as Edwards, Fq};
use snarkos_models::{
    algorithms::CRH,
    gadgets::{
        algorithms::CRHGadget,
        r1cs::{ConstraintSystem, TestConstraintSystem},
        utilities::{alloc::AllocGadget, uint8::UInt8},
    },
    storage::Storage,
};
use snarkos_utilities::bytes::{FromBytes, ToBytes};

use rand::{Rng, RngCore, SeedableRng};
use rand_xorshift::XorShiftRng;
use std::{
    io::{Read, Result as IoResult, Write},
    path::PathBuf,
};

#[derive(Clone)]
pub(super) struct Size;
impl PedersenSize for Size {
    const NUM_WINDOWS: usize = 256;
    const WINDOW_SIZE: usize = 4;
}

type H = PedersenCompressedCRH<Edwards, Size>;
type HG = PedersenCompressedCRHGadget<Edwards, Fq, EdwardsBlsGadget>;

macro_rules! define_merkle_tree_with_height {
    ($struct_name:ident, $height:expr) => {
        #[derive(Clone)]
        struct $struct_name(H);
        impl MerkleParameters for $struct_name {
            type H = H;

            const HEIGHT: usize = $height;

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

        impl Storage for $struct_name {
            /// Store the SNARK proof to a file at the given path.
            fn store(&self, path: &PathBuf) -> IoResult<()> {
                self.0.store(path)
            }

            /// Load the SNARK proof from a file at the given path.
            fn load(path: &PathBuf) -> IoResult<Self> {
                Ok(Self(H::load(path)?))
            }
        }
        impl Default for $struct_name {
            fn default() -> Self {
                let rng = &mut XorShiftRng::seed_from_u64(9174123u64);
                Self(H::setup(rng))
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
                let crh: H = FromBytes::read(&mut reader)?;

                Ok(Self(crh))
            }
        }
    };
}

define_merkle_tree_with_height!(EdwardsMerkleParameters, 32);
define_merkle_tree_with_height!(EdwardsMaskedMerkleParameters, 3);

fn generate_merkle_tree(leaves: &[[u8; 32]], use_bad_root: bool) -> () {
    type EdwardsMerkleTree = MerkleTree<EdwardsMerkleParameters>;

    let mut rng = XorShiftRng::seed_from_u64(9174123u64);

    let parameters = EdwardsMerkleParameters::setup(&mut rng);
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

fn generate_masked_merkle_tree(leaves: &[[u8; 32]], use_bad_root: bool) -> () {
    type EdwardsMaskedMerkleTree = MerkleTree<EdwardsMaskedMerkleParameters>;

    let mut rng = XorShiftRng::seed_from_u64(9174123u64);

    let parameters = EdwardsMaskedMerkleParameters::setup(&mut rng);
    let tree = EdwardsMaskedMerkleTree::new(parameters.clone(), leaves).unwrap();
    let root = tree.root();

    let mut cs = TestConstraintSystem::<Fq>::new();
    let leaf_gadgets = leaves.iter().map(|l| UInt8::constant_vec(l)).collect::<Vec<_>>();

    let mut nonce = [1u8; 32];
    rng.fill_bytes(&mut nonce);
    let mut nonce_bytes = vec![];
    for (byte_i, nonce_byte) in nonce.iter().enumerate() {
        let cs_nonce = cs.ns(|| format!("nonce_byte_gadget_{}", byte_i));
        nonce_bytes.push(UInt8::alloc(cs_nonce, || Ok(*nonce_byte)).unwrap());
    }

    let root = <HG as CRHGadget<H, _>>::OutputGadget::alloc(&mut cs.ns(|| "new_digest_root"), || {
        if use_bad_root {
            Ok(<H as CRH>::Output::default())
        } else {
            Ok(root)
        }
    })
    .unwrap();

    let crh_parameters = <HG as CRHGadget<H, Fq>>::ParametersGadget::alloc(&mut cs.ns(|| "new_parameters"), || {
        Ok(parameters.parameters())
    })
    .unwrap();

    check_root::<EdwardsMerkleParameters, HG, _, _, _>(
        cs.ns(|| "compute masked root"),
        &crh_parameters,
        &nonce_bytes,
        &root,
        &leaf_gadgets,
    )
    .unwrap();

    if !cs.is_satisfied() {
        println!("Unsatisfied constraint: {}", cs.which_is_unsatisfied().unwrap());
    }
    assert!(cs.is_satisfied());
}

#[test]
fn good_root_test() {
    let mut leaves = Vec::new();
    for i in 0..4u8 {
        let input = [i; 32];
        leaves.push(input);
    }
    generate_merkle_tree(&leaves, false);
}

#[should_panic]
#[test]
fn bad_root_test() {
    let mut leaves = Vec::new();
    for i in 0..4u8 {
        let input = [i; 32];
        leaves.push(input);
    }
    generate_merkle_tree(&leaves, true);
}

#[test]
fn good_masked_root_test() {
    let mut leaves = Vec::new();
    for i in 0..4u8 {
        let input = [i; 32];
        leaves.push(input);
    }
    generate_masked_merkle_tree(&leaves, false);
}

#[should_panic]
#[test]
fn bad_masked_root_test() {
    let mut leaves = Vec::new();
    for i in 0..4u8 {
        let input = [i; 32];
        leaves.push(input);
    }
    generate_masked_merkle_tree(&leaves, true);
}
