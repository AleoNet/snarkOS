// Copyright (C) 2019-2020 Aleo Systems Inc.
// This file is part of the snarkOS library.

// The snarkOS library is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// The snarkOS library is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with the snarkOS library. If not, see <https://www.gnu.org/licenses/>.

//! Generic PoSW Miner and Verifier, compatible with any implementer of the SNARK trait.

use crate::circuit::{POSWCircuit, POSWCircuitParameters};
use snarkos_algorithms::crh::sha256d_to_u64;
use snarkos_curves::{
    bls12_377::Fr,
    edwards_bls12::{EdwardsProjective, Fq},
};
use snarkos_errors::posw::PoswError;
use snarkos_gadgets::{algorithms::crh::PedersenCompressedCRHGadget, curves::edwards_bls12::EdwardsBlsGadget};
use snarkos_marlin::snark::SRS;
use snarkos_models::{
    algorithms::{MaskedMerkleParameters, SNARK},
    curves::{to_field_vec::ToConstraintField, PairingEngine, PrimeField},
    gadgets::algorithms::MaskedCRHGadget,
    parameters::Parameters,
};
use snarkos_objects::{
    pedersen_merkle_tree::{pedersen_merkle_root_hash_with_leaves, PedersenMerkleRootHash, PARAMS},
    MaskedMerkleTreeParameters,
};
use snarkos_parameters::{PoswSNARKPKParameters, PoswSNARKVKParameters};
use snarkos_polycommit::optional_rng::OptionalRng;
use snarkos_profiler::{end_timer, start_timer};
use snarkos_utilities::{
    bytes::{FromBytes, ToBytes},
    to_bytes,
};

use blake2::{digest::Digest, Blake2s};
use rand::{rngs::OsRng, Rng};
use std::marker::PhantomData;

// We need to instantiate the Merkle Tree and the Gadget, but these should not be
// proving system specific
pub type M = MaskedMerkleTreeParameters;
pub type HG = PedersenCompressedCRHGadget<EdwardsProjective, Fq, EdwardsBlsGadget>;
pub type F = Fr;

/// A Proof of Succinct Work miner and verifier
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Posw<S, F, M, HG, CP>
where
    S: SNARK,
    F: PrimeField,
    M: MaskedMerkleParameters,
    HG: MaskedCRHGadget<M::H, F>,
    CP: POSWCircuitParameters,
{
    circuit: PhantomData<POSWCircuit<F, M, HG, CP>>,

    /// The (prepared) verifying key.
    pub vk: S::PreparedVerificationParameters,

    /// The proving key. If not provided, the PoSW runner will work in verify-only
    /// mode and the `mine` function will panic.
    pub pk: Option<S::ProvingParameters>,
}

impl<S, CP> Posw<S, F, M, HG, CP>
where
    S: SNARK,
    CP: POSWCircuitParameters,
{
    /// Loads the PoSW runner from the locally stored parameters.
    pub fn verify_only() -> Result<Self, PoswError> {
        let params = PoswSNARKVKParameters::load_bytes()?;
        let vk = S::VerificationParameters::read(&params[..])?;

        Ok(Self {
            pk: None,
            vk: vk.into(),
            circuit: PhantomData,
        })
    }

    /// Loads the PoSW runner from the locally stored parameters.
    pub fn load() -> Result<Self, PoswError> {
        let params = PoswSNARKVKParameters::load_bytes()?;
        let vk = S::VerificationParameters::read(&params[..])?;

        let params = PoswSNARKPKParameters::load_bytes()?;
        let pk = S::ProvingParameters::read(&params[..])?;

        Ok(Self {
            pk: Some(pk),
            vk: vk.into(),
            circuit: PhantomData,
        })
    }

    /// Creates a POSW circuit from the provided transaction ids and nonce.
    fn circuit_from(nonce: u32, leaves: &[Vec<u8>]) -> POSWCircuit<F, M, HG, CP> {
        let (root, leaves) = pedersen_merkle_root_hash_with_leaves(leaves);

        // Generate the mask by committing to the nonce and the root
        let mask = commit(nonce, &root.into());

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

    /// Hashes the proof and checks it against the difficulty
    fn check_difficulty(&self, proof: &[u8], difficulty_target: u64) -> bool {
        let hash_result = sha256d_to_u64(proof);
        hash_result <= difficulty_target
    }
}

impl<S, CP> Posw<S, F, M, HG, CP>
where
    S: SNARK<VerifierInput = Vec<F>, AssignedCircuit = POSWCircuit<F, M, HG, CP>>,
    CP: POSWCircuitParameters,
{
    /// Performs a trusted setup for the PoSW circuit and returns an instance of the runner
    #[cfg(any(test, feature = "test-helpers"))]
    pub fn setup<R: Rng>(rng: &mut R) -> Result<Self, PoswError>
    where
        S: SNARK<Circuit = POSWCircuit<F, M, HG, CP>>,
    {
        let params = S::setup(
            POSWCircuit {
                // the circuit will be padded internally
                leaves: vec![None; 0],
                merkle_parameters: PARAMS.clone(),
                mask: None,
                root: None,
                field_type: PhantomData,
                crh_gadget_type: PhantomData,
                circuit_parameters_type: PhantomData,
            },
            rng,
        )?;

        Ok(Self {
            pk: Some(params.0),
            vk: params.1,
            circuit: PhantomData,
        })
    }

    /// Performs a deterministic setup for systems with universal setups
    pub fn index<E>(srs: SRS<E>) -> Result<Self, PoswError>
    where
        E: PairingEngine,
        S: SNARK<Circuit = (POSWCircuit<F, M, HG, CP>, SRS<E>)>,
    {
        let params = S::setup(
            (
                POSWCircuit {
                    // the circuit will be padded internally
                    leaves: vec![None; 0],
                    merkle_parameters: PARAMS.clone(),
                    mask: None,
                    root: None,
                    field_type: PhantomData,
                    crh_gadget_type: PhantomData,
                    circuit_parameters_type: PhantomData,
                },
                srs,
            ),
            // we need to specify the RNG type, but it is guaranteed to panic if used
            &mut OptionalRng(None::<OsRng>),
        )?;

        Ok(Self {
            pk: Some(params.0),
            vk: params.1,
            circuit: PhantomData,
        })
    }

    /// Given the subroots of the block, it will calculate a POSW and a nonce such that they are
    /// under the difficulty target. These can then be used in the block header's field.
    pub fn mine<R: Rng>(
        &self,
        subroots: &[Vec<u8>],
        difficulty_target: u64, // TODO: Change to Bignum?
        rng: &mut R,
        max_nonce: u32,
    ) -> Result<(u32, Vec<u8>), PoswError> {
        let pk = self.pk.as_ref().expect("tried to mine without a PK set up");

        let mut nonce;
        let mut proof;
        let mut serialized_proof;
        loop {
            nonce = rng.gen_range(0, max_nonce);
            proof = Self::prove(&pk, nonce, subroots, rng)?;

            serialized_proof = to_bytes!(proof)?;
            if self.check_difficulty(&serialized_proof, difficulty_target) {
                break;
            }
        }

        Ok((nonce, serialized_proof))
    }

    /// Runs the internal SNARK `prove` function on the POSW circuit and returns
    /// the proof serialized as bytes
    fn prove<R: Rng>(
        pk: &S::ProvingParameters,
        nonce: u32,
        subroots: &[Vec<u8>],
        rng: &mut R,
    ) -> Result<S::Proof, PoswError> {
        // instantiate the circuit with the nonce
        let circuit = Self::circuit_from(nonce, subroots);

        // generate the proof
        let proof_timer = start_timer!(|| "POSW proof");
        let proof = S::prove(pk, circuit, rng)?;
        end_timer!(proof_timer);

        Ok(proof)
    }

    /// Verifies the Proof of Succinct Work against the nonce and pedersen merkle
    /// root hash (produced by running a pedersen hash over the roots of the subtrees
    /// created by the block's transaction ids)
    pub fn verify(
        &self,
        nonce: u32,
        proof: &S::Proof,
        pedersen_merkle_root: &PedersenMerkleRootHash,
    ) -> Result<(), PoswError> {
        // commit to it and the nonce
        let mask = commit(nonce, pedersen_merkle_root);

        // get the mask and the root in public inputs format
        let merkle_root = F::read(&pedersen_merkle_root.0[..])?;
        let inputs = [mask.to_field_elements()?, vec![merkle_root]].concat();

        let res = S::verify(&self.vk, &inputs, &proof)?;
        if !res {
            return Err(PoswError::PoswVerificationFailed);
        }

        Ok(())
    }
}

/// Commits to the nonce and pedersen merkle root
pub fn commit(nonce: u32, root: &PedersenMerkleRootHash) -> Vec<u8> {
    let mut h = Blake2s::new();
    h.input(&nonce.to_le_bytes());
    h.input(root.0.as_ref());
    h.result().to_vec()
}
