//! Implements a Proof of Succinct work circuit. The inputs are transaction IDs as the leaves,
//! which are then used as leaves in a tree instantiated with a masked Pedersen hash. The prover
//! inputs a mask computed as Blake2s(nonce || root), which the verifier also checks.

use snarkos_errors::gadgets::SynthesisError;
use snarkos_gadgets::algorithms::merkle_tree::compute_root;
use snarkos_models::{
    algorithms::CRH,
    curves::PrimeField,
    gadgets::{
        algorithms::{CRHGadget, MaskedCRHGadget},
        r1cs::{Assignment, ConstraintSynthesizer, ConstraintSystem},
        utilities::{alloc::AllocGadget, eq::EqGadget, uint8::UInt8},
    },
};
use std::marker::PhantomData;

/// Enforces sizes of the mask and leaves.
pub trait POSWCircuitParameters {
    const LEAF_LENGTH: usize;
    const MASK_LENGTH: usize;
}

pub struct POSWCircuit<F: PrimeField, H: CRH, HG: MaskedCRHGadget<H, F>, CP: POSWCircuitParameters> {
    pub leaves: Vec<Vec<Option<u8>>>,
    pub crh_parameters: H::Parameters,
    pub mask: Option<Vec<u8>>,
    pub root: Option<H::Output>,

    pub field_type: PhantomData<F>,
    pub crh_gadget_type: PhantomData<HG>,
    pub circuit_parameters_type: PhantomData<CP>,
}

impl<F: PrimeField, H: CRH, HG: MaskedCRHGadget<H, F>, CP: POSWCircuitParameters> ConstraintSynthesizer<F>
    for POSWCircuit<F, H, HG, CP>
{
    fn generate_constraints<CS: ConstraintSystem<F>>(self, cs: &mut CS) -> Result<(), SynthesisError> {
        // Compute the mask if it exists.
        let mask = self.mask.clone().unwrap_or(vec![0; CP::MASK_LENGTH]);
        if mask.len() != CP::MASK_LENGTH {
            return Err(SynthesisError::Unsatisfiable)
        }
        let mask_bytes = UInt8::alloc_input_vec(cs.ns(|| "mask"), &mask)?;

        let crh_parameters = <HG as CRHGadget<H, F>>::ParametersGadget::alloc(&mut cs.ns(|| "new_parameters"), || {
            Ok(self.crh_parameters.clone())
        })?;

        // Initialize the leaves.
        let leaf_gadgets = self
            .leaves
            .iter()
            .enumerate()
            .map(|(i, l)| {
                if l.len() != CP::LEAF_LENGTH {
                    Err(SynthesisError::Unsatisfiable)
                } else {
                    Ok(UInt8::alloc_vec(cs.ns(|| format!("leaf {}", i)), &l)?)
                }
            })
            .collect::<Result<Vec<_>, _>>()?;

        // Compute the root using the masked tree.
        let computed_root = compute_root::<H, HG, _, _, _>(
            cs.ns(|| "compute masked root"),
            &crh_parameters,
            &mask_bytes,
            &leaf_gadgets,
        )?;

        // Enforce the input root is the same as the computed root.
        let public_computed_root = HG::OutputGadget::alloc_input(cs.ns(|| "public computed root"), || self.root.get())?;
        computed_root.enforce_equal(cs.ns(|| "inputize computed root"), &public_computed_root)?;

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::{POSWCircuit, POSWCircuitParameters};
    use blake2::{digest::Digest, Blake2s};
    use rand::thread_rng;
    use snarkos_algorithms::{
        crh::{PedersenCompressedCRH, PedersenSize},
        snark::{create_random_proof, generate_random_parameters, prepare_verifying_key, verify_proof},
    };
    use snarkos_curves::{
        bls12_377::{Bls12_377, Fr},
        edwards_bls12::{EdwardsProjective as Edwards, Fq},
    };
    use snarkos_gadgets::{
        algorithms::crh::PedersenCompressedCRHGadget,
        curves::edwards_bls12::EdwardsBlsGadget,
        define_test_merkle_tree_with_height,
    };
    use snarkos_models::{algorithms::CRH, curves::to_field_vec::ToConstraintField};
    use std::marker::PhantomData;

    #[derive(Clone)]
    pub(super) struct Size;
    impl PedersenSize for Size {
        const NUM_WINDOWS: usize = 256;
        const WINDOW_SIZE: usize = 4;
    }

    type H = PedersenCompressedCRH<Edwards, Size>;
    type HG = PedersenCompressedCRHGadget<Edwards, Fq, EdwardsBlsGadget>;

    struct TestPOSWCircuitParameters {}
    impl POSWCircuitParameters for TestPOSWCircuitParameters {
        const LEAF_LENGTH: usize = 32;
        const MASK_LENGTH: usize = 32;
    }

    define_test_merkle_tree_with_height!(EdwardsMaskedMerkleParameters, 5);
    #[test]
    fn test_tree_proof() {
        let mut rng = thread_rng();

        let parameters = EdwardsMaskedMerkleParameters::setup(&mut rng);
        let params = generate_random_parameters::<Bls12_377, _, _>(
            POSWCircuit::<_, H, HG, TestPOSWCircuitParameters> {
                leaves: vec![vec![None; 32]; 16],
                crh_parameters: parameters.parameters().clone(),
                mask: None,
                root: None,
                field_type: PhantomData,
                crh_gadget_type: PhantomData,
                circuit_parameters_type: PhantomData,
            },
            &mut rng,
        )
        .unwrap();

        let nonce = [1; 32];
        let leaves = vec![vec![3; 32]; 16];
        type EdwardsMaskedMerkleTree = MerkleTree<EdwardsMaskedMerkleParameters>;
        let tree = EdwardsMaskedMerkleTree::new(parameters.clone(), &leaves).unwrap();
        let root = tree.root();
        let mut root_bytes = [0; 32];
        root.write(&mut root_bytes[..]).unwrap();

        let mut h = Blake2s::new();
        h.input(nonce.as_ref());
        h.input(root_bytes.as_ref());
        let mask = h.result().to_vec();

        let snark_leaves = leaves.iter().map(|l| l.iter().map(|b| Some(*b)).collect()).collect();
        let proof = create_random_proof(
            POSWCircuit::<_, H, HG, TestPOSWCircuitParameters> {
                leaves: snark_leaves,
                crh_parameters: parameters.parameters().clone(),
                mask: Some(mask.clone()),
                root: Some(root),
                field_type: PhantomData,
                crh_gadget_type: PhantomData,
                circuit_parameters_type: PhantomData,
            },
            &params,
            &mut rng,
        )
        .unwrap();

        let inputs = [ToConstraintField::<Fr>::to_field_elements(&mask[..]).unwrap(), vec![
            root,
        ]]
        .concat();

        assert!(verify_proof(&prepare_verifying_key(&params.vk), &proof, &inputs,).unwrap());
    }
}
