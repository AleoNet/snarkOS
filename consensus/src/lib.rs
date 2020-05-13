#![warn(unused_extern_crates)]
#![forbid(unsafe_code)]

#[macro_use]
extern crate snarkos_profiler;

pub mod consensus;
pub use self::consensus::*;

pub mod difficulty;
pub use self::difficulty::*;

pub mod miner;

#[allow(dead_code)]
pub mod test_data;

// Instantiate the SNARK
pub mod posw {
    use blake2::{digest::Digest, Blake2s};
    use snarkos_algorithms::snark;
    use snarkos_curves::{
        bls12_377::{Bls12_377, Fr},
        edwards_bls12::{EdwardsProjective as Edwards, Fq},
    };
    use snarkos_gadgets::{algorithms::crh::PedersenCompressedCRHGadget, curves::edwards_bls12::EdwardsBlsGadget};
    use snarkos_objects::pedersen_merkle_tree::{
        mtree::CommitmentMerkleParameters,
        pedersen_merkle_root_hash_with_leaves,
        PedersenMerkleRootHash,
        PARAMS,
    };
    use snarkos_posw::circuit::{POSWCircuit, POSWCircuitParameters};
    use std::marker::PhantomData;

    pub type Curve = Bls12_377;
    pub type Field = Fr;

    pub type VerifyingKey = snark::VerifyingKey<Curve>;
    pub type ProvingKey = snark::Parameters<Curve>;
    pub type Proof = snark::Proof<Curve>;

    // We use 32 byte leaves and 32 byte nonces in PoSW.
    pub struct PoSWParams;
    impl POSWCircuitParameters for PoSWParams {
        const MASK_LENGTH: usize = 32;
    }

    pub fn commit(nonce: u32, root: PedersenMerkleRootHash) -> Vec<u8> {
        let mut h = Blake2s::new();
        h.input(nonce.to_le_bytes());
        h.input(root.0.as_ref());
        h.result().to_vec()
    }

    type HG = PedersenCompressedCRHGadget<Edwards, Fq, EdwardsBlsGadget>;

    pub type POSW = POSWCircuit<Fr, CommitmentMerkleParameters, HG, PoSWParams>;

    pub fn instantiate_posw(nonce: u32, leaves: &[Vec<u8>]) -> POSW {
        let (root, leaves) = pedersen_merkle_root_hash_with_leaves(leaves);
        let mask = commit(nonce, root.into());

        // Convert the leaves to Options for the SNARK
        let leaves = leaves.into_iter().map(|l| Some(l)).collect();

        // Hash the nonce and the merkle root
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
