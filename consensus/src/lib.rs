#![warn(unused_extern_crates)]
#![forbid(unsafe_code)]

pub mod consensus;
pub use self::consensus::*;

pub mod difficulty;
pub use self::difficulty::*;

pub mod miner;

#[allow(dead_code)]
pub mod test_data;


// Instantiate the SNARK
pub mod posw {
    use snarkos_curves::{
        bls12_377::{Bls12_377, Fr},
        edwards_bls12::{EdwardsProjective as Edwards, Fq},
    };
    use snarkos_gadgets::{
        algorithms::crh::PedersenCompressedCRHGadget,
        curves::edwards_bls12::EdwardsBlsGadget,
    };
    use snarkos_objects::pedersen_merkle_tree::{PARAMS, pedersen_merkle_root_hash, PedersenMerkleRootHash, mtree::MerkleTreeCRH};
    use snarkos_algorithms::{snark, merkle_tree::MerkleParameters};
    use snarkos_posw::circuit::{POSWCircuitParameters, POSWCircuit};
    use std::marker::PhantomData;
    use blake2::{digest::Digest, Blake2s};

    pub type Curve = Bls12_377;
    pub type Field = Fr;

    pub type VerifyingKey = snark::VerifyingKey<Curve>;
    pub type ProvingKey = snark::Parameters<Curve>;
    pub type Proof = snark::Proof<Curve>;

    // We use 32 byte leaves and 32 byte nonces in PoSW.
    pub struct PoSWParams;
    impl POSWCircuitParameters for PoSWParams {
        const LEAF_LENGTH: usize = 32;
        const MASK_LENGTH: usize = 32;
    }

    fn commit(nonce: u32, root: PedersenMerkleRootHash) -> Vec<u8> {
        let mut h = Blake2s::new();
        h.input(nonce.to_le_bytes());
        h.input(root.0.as_ref());
        h.result().to_vec()
    }

    type HG = PedersenCompressedCRHGadget<Edwards, Fq, EdwardsBlsGadget>;

    pub type POSW = POSWCircuit<Fr, MerkleTreeCRH, HG, PoSWParams>;

    pub fn instantiate_posw(nonce: u32, leaves: &[Vec<u8>]) -> POSW {
        let root = pedersen_merkle_root_hash(leaves);
        let mask = commit(nonce, root.into());

        // Convert the leaves to Options for the SNARK
        let leaves = leaves
            .iter()
            .map(|l| l.iter().map(|i| Some(*i)).collect())
            .collect();

        // Hash the nonce and the merkle root
        POSWCircuit {
            leaves,
            crh_parameters: PARAMS.parameters().clone(),
            mask: Some(mask),
            root: Some(root),
            field_type: PhantomData,
            crh_gadget_type: PhantomData,
            circuit_parameters_type: PhantomData,
        }
    }
}
