use rand::{Rng, RngCore, SeedableRng};
use rand_xorshift::XorShiftRng;
use snarkos_algorithms::{
    crh::{PedersenCompressedCRH, PedersenSize},
    merkle_tree::{MerkleParameters, MerkleTree},
    prf::blake2s::Blake2s,
};
use snarkos_curves::edwards_bls12::{EdwardsProjective as Edwards, Fq};
use snarkos_errors::gadgets::SynthesisError;
use snarkos_gadgets::{
    algorithms::{crh::PedersenCompressedCRHGadget, merkle_tree::*},
    curves::edwards_bls12::EdwardsBlsGadget,
    define_merkle_tree_with_height,
};
use snarkos_models::{
    algorithms::{CRH, PRF},
    curves::{to_field_vec::ToConstraintField, PrimeField},
    gadgets::{
        algorithms::CRHGadget,
        curves::{field::FieldGadget, FpGadget},
        r1cs::{Assignment, ConstraintSynthesizer, ConstraintSystem},
        utilities::{alloc::AllocGadget, eq::EqGadget, uint8::UInt8, ToBytesGadget},
    },
    storage::Storage,
};
use snarkos_utilities::bytes::{FromBytes, ToBytes};
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
define_merkle_tree_with_height!(EdwardsMaskedMerkleParameters, 5);

type H = PedersenCompressedCRH<Edwards, Size>;
type HG = PedersenCompressedCRHGadget<Edwards, Fq, EdwardsBlsGadget>;

pub struct POSWCircuit {
    leaves: Vec<[Option<u8>; 32]>,
    merkle_parameters: EdwardsMaskedMerkleParameters,
    mask: Option<[u8; 32]>,
}

impl ConstraintSynthesizer<Fq> for POSWCircuit {
    fn generate_constraints<CS: ConstraintSystem<Fq>>(self, cs: &mut CS) -> Result<(), SynthesisError> {
        assert_eq!(self.leaves.len(), 1 << EdwardsMaskedMerkleParameters::HEIGHT - 1);
        let mask = match self.mask {
            Some(mask) => mask,
            _ => [0; 32],
        };
        let mask_bytes = UInt8::alloc_input_vec(cs.ns(|| "mask"), &mask[..])?;

        let crh_parameters =
            <HG as CRHGadget<H, Fq>>::ParametersGadget::alloc(&mut cs.ns(|| "new_parameters"), || {
                Ok(self.merkle_parameters.parameters())
            })?;
        let leaf_gadgets = self
            .leaves
            .into_iter()
            .enumerate()
            .map(|(i, l)| UInt8::alloc_vec(cs.ns(|| format!("leaf {}", i)), &l[..]))
            .collect::<Result<Vec<_>, _>>()?;

        let computed_root = compute_root::<EdwardsMaskedMerkleParameters, HG, _, _, _>(
            cs.ns(|| "compute masked root"),
            &crh_parameters,
            &mask_bytes,
            &leaf_gadgets,
        )?;

        let public_computed_root =
            FpGadget::alloc_input(cs.ns(|| "public computed root"), || computed_root.get_value().get())?;
        public_computed_root.enforce_equal(cs.ns(|| "inputize computed root"), &computed_root)?;

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use rand::{thread_rng, Rng};
    use snarkos_algorithms::snark::{
        create_random_proof,
        generate_random_parameters,
        prepare_verifying_key,
        verify_proof,
    };
    use snarkos_curves::bls12_377::{Bls12_377, Fq, Fr};
    use snarkos_utilities::to_bytes;

    #[test]
    fn test_tree_proof() {
        let mut rng = thread_rng();

        let parameters = EdwardsMaskedMerkleParameters::setup(&mut rng);
        let params = generate_random_parameters::<Bls12_377, _, _>(
            POSWCircuit {
                leaves: vec![[None; 32]; 16],
                merkle_parameters: parameters.clone(),
                mask: None,
            },
            &mut rng,
        )
        .unwrap();

        let nonce = [1; 32];
        let root = [2; 32];
        let mask = Blake2s::evaluate(&nonce, &root).unwrap();
        let leaves = vec![[3; 32]; 16];
        let mut snark_leaves = vec![[Some(0); 32]; 16];
        snark_leaves
            .iter_mut()
            .zip(leaves.iter())
            .for_each(|(a, b)| a.iter_mut().zip(b.iter()).for_each(|(aa, bb)| *aa = Some(*bb)));
        let proof = create_random_proof(
            POSWCircuit {
                leaves: snark_leaves,
                merkle_parameters: parameters.clone(),
                mask: Some(mask),
            },
            &params,
            &mut rng,
        )
        .unwrap();

        type EdwardsMaskedMerkleTree = MerkleTree<EdwardsMaskedMerkleParameters>;
        let tree = EdwardsMaskedMerkleTree::new(parameters, &leaves).unwrap();
        let root = tree.root();

        let inputs = [ToConstraintField::<Fr>::to_field_elements(&mask[..]).unwrap(), vec![
            root,
        ]]
        .concat();

        assert!(verify_proof(&prepare_verifying_key(&params.vk), &proof, &inputs,).unwrap());
    }
}
