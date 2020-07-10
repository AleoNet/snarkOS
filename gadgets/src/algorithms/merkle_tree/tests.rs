use crate::{
    algorithms::{
        crh::{BoweHopwoodPedersenCompressedCRHGadget, PedersenCRHGadget, PedersenCompressedCRHGadget},
        merkle_tree::*,
    },
    curves::edwards_bls12::EdwardsBlsGadget,
};
use snarkos_algorithms::{
    crh::{BoweHopwoodPedersenCompressedCRH, PedersenCRH, PedersenCompressedCRH, PedersenSize},
    define_masked_merkle_tree_parameters,
    merkle_tree::MerkleTree,
};
use snarkos_curves::{
    bls12_377::Fr,
    edwards_bls12::{EdwardsAffine, EdwardsProjective},
};
use snarkos_models::{
    algorithms::{MaskedMerkleParameters, MerkleParameters, CRH},
    curves::PrimeField,
    gadgets::{
        algorithms::{CRHGadget, MaskedCRHGadget},
        r1cs::{ConstraintSystem, TestConstraintSystem},
        utilities::{alloc::AllocGadget, eq::EqGadget, uint::UInt8},
    },
};
use snarkos_utilities::ToBytes;

use blake2::{digest::Digest, Blake2s};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Size;
impl PedersenSize for Size {
    const NUM_WINDOWS: usize = 256;
    const WINDOW_SIZE: usize = 4;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoweHopwoodSize;
impl PedersenSize for BoweHopwoodSize {
    const NUM_WINDOWS: usize = 32;
    const WINDOW_SIZE: usize = 60;
}

fn generate_merkle_tree<P: MerkleParameters, F: PrimeField, HG: CRHGadget<P::H, F>>(
    leaves: &[[u8; 30]],
    use_bad_root: bool,
) -> () {
    let parameters = P::default();
    let tree = MerkleTree::<P>::new(parameters.clone(), leaves).unwrap();
    let root = tree.root();
    let mut satisfied = true;
    for (i, leaf) in leaves.iter().enumerate() {
        let mut cs = TestConstraintSystem::<F>::new();
        let proof = tree.generate_proof(i, &leaf).unwrap();
        assert!(proof.verify(&root, &leaf).unwrap());

        // Allocate Merkle tree root
        let root = <HG as CRHGadget<_, _>>::OutputGadget::alloc(&mut cs.ns(|| format!("new_digest_{}", i)), || {
            if use_bad_root {
                Ok(<P::H as CRH>::Output::default())
            } else {
                Ok(root.clone())
            }
        })
        .unwrap();

        let constraints_from_digest = cs.num_constraints();
        println!("constraints from digest: {}", constraints_from_digest);

        // Allocate Parameters for CRH
        let crh_parameters =
            <HG as CRHGadget<_, _>>::ParametersGadget::alloc(&mut cs.ns(|| format!("new_parameters_{}", i)), || {
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

fn generate_masked_merkle_tree<P: MaskedMerkleParameters, F: PrimeField, HG: MaskedCRHGadget<P::H, F>>(
    leaves: &[[u8; 30]],
    use_bad_root: bool,
) -> () {
    let parameters = P::default();
    let tree = MerkleTree::<P>::new(parameters.clone(), leaves).unwrap();
    let root = tree.root();

    let mut cs = TestConstraintSystem::<F>::new();
    let leaf_gadgets = tree
        .hashed_leaves()
        .iter()
        .enumerate()
        .map(|(i, l)| <HG as CRHGadget<_, _>>::OutputGadget::alloc(cs.ns(|| format!("leaf {}", i)), || Ok(l)))
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    let nonce: [u8; 4] = rand::random();
    let mut root_bytes = [0u8; 32];
    root.write(&mut root_bytes[..]).unwrap();

    let mut h = Blake2s::new();
    h.input(nonce.as_ref());
    h.input(&root_bytes);
    let mask = h.result().to_vec();
    let mask_bytes = UInt8::alloc_vec(cs.ns(|| "mask"), &mask).unwrap();

    let crh_parameters = <HG as CRHGadget<_, _>>::ParametersGadget::alloc(&mut cs.ns(|| "new_parameters"), || {
        Ok(parameters.parameters())
    })
    .unwrap();

    let mask_crh_parameters =
        <HG as CRHGadget<_, _>>::ParametersGadget::alloc(&mut cs.ns(|| "new_mask_parameters"), || {
            Ok(parameters.mask_parameters())
        })
        .unwrap();

    let computed_root = compute_root::<_, HG, _, _, _>(
        cs.ns(|| "compute masked root"),
        &crh_parameters,
        &mask_crh_parameters,
        &mask_bytes,
        &leaf_gadgets,
    )
    .unwrap();

    let given_root = if use_bad_root {
        <P::H as CRH>::Output::default()
    } else {
        root
    };

    let given_root_gadget =
        <HG as CRHGadget<_, _>>::OutputGadget::alloc(&mut cs.ns(|| "given root"), || Ok(given_root)).unwrap();

    computed_root
        .enforce_equal(
            &mut cs.ns(|| "Check that computed root matches provided root"),
            &given_root_gadget,
        )
        .unwrap();

    if !cs.is_satisfied() {
        println!("Unsatisfied constraint: {}", cs.which_is_unsatisfied().unwrap());
    }
    assert!(cs.is_satisfied());
}

mod merkle_tree_pedersen_crh_on_affine {
    use super::*;

    define_masked_merkle_tree_parameters!(EdwardsMerkleParameters, H, 4);

    type H = PedersenCRH<EdwardsAffine, Size>;
    type HG = PedersenCRHGadget<EdwardsAffine, Fr, EdwardsBlsGadget>;

    #[test]
    fn good_root_test() {
        let mut leaves = Vec::new();
        for i in 0..1 << EdwardsMerkleParameters::DEPTH {
            let input = [i; 30];
            leaves.push(input);
        }
        generate_merkle_tree::<EdwardsMerkleParameters, Fr, HG>(&leaves, false);
    }

    #[should_panic]
    #[test]
    fn bad_root_test() {
        let mut leaves = Vec::new();
        for i in 0..1 << EdwardsMerkleParameters::DEPTH {
            let input = [i; 30];
            leaves.push(input);
        }
        generate_merkle_tree::<EdwardsMerkleParameters, Fr, HG>(&leaves, true);
    }
}

mod merkle_tree_compressed_pedersen_crh_on_projective {
    use super::*;

    define_masked_merkle_tree_parameters!(EdwardsMerkleParameters, H, 4);

    type H = PedersenCompressedCRH<EdwardsProjective, Size>;
    type HG = PedersenCompressedCRHGadget<EdwardsProjective, Fr, EdwardsBlsGadget>;

    #[test]
    fn good_root_test() {
        let mut leaves = Vec::new();
        for i in 0..1 << EdwardsMerkleParameters::DEPTH {
            let input = [i; 30];
            leaves.push(input);
        }
        generate_merkle_tree::<EdwardsMerkleParameters, Fr, HG>(&leaves, false);
    }

    #[should_panic]
    #[test]
    fn bad_root_test() {
        let mut leaves = Vec::new();
        for i in 0..1 << EdwardsMerkleParameters::DEPTH {
            let input = [i; 30];
            leaves.push(input);
        }
        generate_merkle_tree::<EdwardsMerkleParameters, Fr, HG>(&leaves, true);
    }

    #[test]
    fn good_masked_root_test() {
        let mut leaves = Vec::new();
        for i in 0..1 << EdwardsMerkleParameters::DEPTH {
            let input = [i; 30];
            leaves.push(input);
        }
        generate_masked_merkle_tree::<EdwardsMerkleParameters, Fr, HG>(&leaves, false);
    }

    #[should_panic]
    #[test]
    fn bad_masked_root_test() {
        let mut leaves = Vec::new();
        for i in 0..1 << EdwardsMerkleParameters::DEPTH {
            let input = [i; 30];
            leaves.push(input);
        }
        generate_masked_merkle_tree::<EdwardsMerkleParameters, Fr, HG>(&leaves, true);
    }
}

mod merkle_tree_bowe_hopwood_pedersen_compressed_crh_on_projective {
    use super::*;

    define_masked_merkle_tree_parameters!(EdwardsMerkleParameters, H, 4);

    type H = BoweHopwoodPedersenCompressedCRH<EdwardsProjective, BoweHopwoodSize>;
    type HG = BoweHopwoodPedersenCompressedCRHGadget<EdwardsProjective, Fr, EdwardsBlsGadget>;

    #[test]
    fn good_root_test() {
        let mut leaves = Vec::new();
        for i in 0..1 << EdwardsMerkleParameters::DEPTH {
            let input = [i; 30];
            leaves.push(input);
        }
        generate_merkle_tree::<EdwardsMerkleParameters, Fr, HG>(&leaves, false);
    }

    #[should_panic]
    #[test]
    fn bad_root_test() {
        let mut leaves = Vec::new();
        for i in 0..1 << EdwardsMerkleParameters::DEPTH {
            let input = [i; 30];
            leaves.push(input);
        }
        generate_merkle_tree::<EdwardsMerkleParameters, Fr, HG>(&leaves, true);
    }
}
