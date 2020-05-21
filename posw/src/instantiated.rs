use crate::circuit::POSWCircuit;
use snarkos_algorithms::snark;
use snarkos_curves::{
    bls12_377::{Bls12_377, Fr},
    edwards_bls12::{EdwardsProjective as Edwards, Fq},
};
use snarkos_gadgets::{algorithms::crh::PedersenCompressedCRHGadget, curves::edwards_bls12::EdwardsBlsGadget};
use snarkos_objects::pedersen_merkle_tree::{
    pedersen_merkle_root_hash_with_leaves,
    MaskedMerkleTreeParameters,
    PedersenMerkleRootHash,
    PARAMS,
};

use blake2::{digest::Digest, Blake2s};
use std::marker::PhantomData;

pub type Curve = Bls12_377;
pub type Field = Fr;

pub type VerifyingKey = snark::VerifyingKey<Curve>;
pub type ProvingKey = snark::Parameters<Curve>;
pub type Proof = snark::Proof<Curve>;

// Do not leak private type
mod params {
    pub struct PoSWParams;
    impl crate::circuit::POSWCircuitParameters for PoSWParams {
        // A 32 byte mask is sufficient for Pedersen hashes on BLS12-377, leaves and the root
        const MASK_LENGTH: usize = 32;
    }
}

pub fn commit(nonce: u32, root: PedersenMerkleRootHash) -> Vec<u8> {
    let mut h = Blake2s::new();
    h.input(&nonce.to_le_bytes());
    h.input(root.0.as_ref());
    h.result().to_vec()
}

type HashGadget = PedersenCompressedCRHGadget<Edwards, Fq, EdwardsBlsGadget>;

pub type POSW = POSWCircuit<Fr, MaskedMerkleTreeParameters, HashGadget, params::PoSWParams>;

impl POSW {
    pub fn new(nonce: u32, leaves: &[Vec<u8>]) -> Self {
        let (root, leaves) = pedersen_merkle_root_hash_with_leaves(leaves);

        // Generate the mask by committing to the nonce and the root
        let mask = commit(nonce, root.into());

        // Convert the leaves to Options for the SNARK
        let leaves = leaves.into_iter().map(|l| Some(l)).collect();

        POSWCircuit {
            leaves,
            merkle_parameters: PARAMS.clone(),
            mask: Some(mask),
            root: Some(root),
            field_type: PhantomData,
            crh_gadget_type: PhantomData,
            circuit_parameters_type: PhantomData,
        }
    }
}
