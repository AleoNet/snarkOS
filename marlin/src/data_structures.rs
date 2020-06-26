use crate::{
    ahp::{indexer::*, prover::ProverMsg},
    Vec,
};
use core::marker::PhantomData;
use derivative::Derivative;
use snarkos_errors::serialization::SerializationError;
use snarkos_models::{curves::PrimeField, gadgets::r1cs::ConstraintSynthesizer};
use snarkos_polycommit::{BatchLCProof, PolynomialCommitment};
use snarkos_utilities::{
    bytes::{FromBytes, ToBytes},
    error,
    serialize::*,
};
use std::io::{self, Read, Write};

/* ************************************************************************* */
/* ************************************************************************* */
/* ************************************************************************* */

/// The universal public parameters for the argument system.
pub type UniversalSRS<F, PC> = <PC as PolynomialCommitment<F>>::UniversalParams;

/* ************************************************************************* */
/* ************************************************************************* */
/* ************************************************************************* */

/// Verification key for a specific index (i.e., R1CS matrices).
#[derive(Derivative)]
#[derivative(Clone(bound = ""))]
#[derive(Debug, CanonicalSerialize, CanonicalDeserialize)]
pub struct IndexVerifierKey<F: PrimeField, PC: PolynomialCommitment<F>, C: ConstraintSynthesizer<F>> {
    /// Stores information about the size of the index, as well as its field of
    /// definition.
    pub index_info: IndexInfo<F, C>,
    /// Commitments to the indexed polynomials.
    pub index_comms: Vec<PC::Commitment>,
    /// The verifier key for this index, trimmed from the universal SRS.
    pub verifier_key: PC::VerifierKey,
}

impl<F: PrimeField, PC: PolynomialCommitment<F>, C: ConstraintSynthesizer<F>> ToBytes for IndexVerifierKey<F, PC, C> {
    fn write<W: Write>(&self, mut w: W) -> io::Result<()> {
        CanonicalSerialize::serialize(self, &mut w).map_err(|_| error("could not serialize IndexVerifierKey"))
    }
}

impl<F: PrimeField, PC: PolynomialCommitment<F>, C: ConstraintSynthesizer<F>> FromBytes for IndexVerifierKey<F, PC, C> {
    fn read<R: Read>(mut r: R) -> io::Result<Self> {
        CanonicalDeserialize::deserialize(&mut r).map_err(|_| error("could not deserialize IndexVerifierKey"))
    }
}

impl<F: PrimeField, PC: PolynomialCommitment<F>, C: ConstraintSynthesizer<F>> IndexVerifierKey<F, PC, C> {
    /// Iterate over the commitments to indexed polynomials in `self`.
    pub fn iter(&self) -> impl Iterator<Item = &PC::Commitment> {
        self.index_comms.iter()
    }
}

/* ************************************************************************* */
/* ************************************************************************* */
/* ************************************************************************* */

/// Proving key for a specific index (i.e., R1CS matrices).
#[derive(Derivative)]
#[derivative(Clone(bound = "C: 'a"))]
#[derive(Debug, CanonicalSerialize, CanonicalDeserialize)]
pub struct IndexProverKey<'a, F: PrimeField, PC: PolynomialCommitment<F>, C: ConstraintSynthesizer<F>> {
    /// The index verifier key.
    pub index_vk: IndexVerifierKey<F, PC, C>,
    /// The randomness for the index polynomial commitments.
    pub index_comm_rands: Vec<PC::Randomness>,
    /// The index itself.
    pub index: Index<'a, F, C>,
    /// The committer key for this index, trimmed from the universal SRS.
    pub committer_key: PC::CommitterKey,
}

/* ************************************************************************* */
/* ************************************************************************* */
/* ************************************************************************* */

/// A zkSNARK proof.
#[derive(Derivative)]
#[derivative(Debug(bound = ""), Clone(bound = ""))]
#[derive(CanonicalSerialize, CanonicalDeserialize)]
pub struct Proof<F: PrimeField, PC: PolynomialCommitment<F>, C: ConstraintSynthesizer<F>> {
    /// Commitments to the polynomials produced by the AHP prover.
    pub commitments: Vec<Vec<PC::Commitment>>,
    /// Evaluations of these polynomials.
    pub evaluations: Vec<F>,
    /// The field elements sent by the prover.
    pub prover_messages: Vec<ProverMsg<F>>,
    /// An evaluation proof from the polynomial commitment.
    pub pc_proof: BatchLCProof<F, PC>,
    #[doc(hidden)]
    constraint_system: PhantomData<C>,
}

impl<F: PrimeField, PC: PolynomialCommitment<F>, C: ConstraintSynthesizer<F>> ToBytes for Proof<F, PC, C> {
    fn write<W: Write>(&self, mut w: W) -> io::Result<()> {
        CanonicalSerialize::serialize(self, &mut w).map_err(|_| error("could not serialize IndexVerifierKey"))
    }
}

impl<F: PrimeField, PC: PolynomialCommitment<F>, C: ConstraintSynthesizer<F>> FromBytes for Proof<F, PC, C> {
    fn read<R: Read>(mut r: R) -> io::Result<Self> {
        CanonicalDeserialize::deserialize(&mut r).map_err(|_| error("could not deserialize Proof"))
    }
}

impl<F: PrimeField, PC: PolynomialCommitment<F>, C: ConstraintSynthesizer<F>> Proof<F, PC, C> {
    /// Construct a new proof.
    pub fn new(
        commitments: Vec<Vec<PC::Commitment>>,
        evaluations: Vec<F>,
        prover_messages: Vec<ProverMsg<F>>,
        pc_proof: BatchLCProof<F, PC>,
    ) -> Self {
        Self {
            commitments,
            evaluations,
            prover_messages,
            pc_proof,
            constraint_system: PhantomData,
        }
    }

    /// Prints information about the size of the proof.
    pub fn print_size_info(&self) {
        use snarkos_polycommit::PCCommitment;

        let size_of_fe_in_bytes = F::zero().into_repr().as_ref().len() * 8;
        let mut num_comms_without_degree_bounds = 0;
        let mut num_comms_with_degree_bounds = 0;
        let mut size_bytes_comms_without_degree_bounds = 0;
        let mut size_bytes_comms_with_degree_bounds = 0;
        let mut size_bytes_proofs = 0;
        for c in self.commitments.iter().flat_map(|c| c) {
            if !c.has_degree_bound() {
                num_comms_without_degree_bounds += 1;
                size_bytes_comms_without_degree_bounds += c.serialized_size();
            } else {
                num_comms_with_degree_bounds += 1;
                size_bytes_comms_with_degree_bounds += c.serialized_size();
            }
        }

        let proofs: Vec<PC::Proof> = self.pc_proof.proof.clone().into();
        let num_proofs = proofs.len();
        for proof in &proofs {
            size_bytes_proofs += proof.serialized_size();
        }

        let num_evals = self.evaluations.len();
        let evals_size_in_bytes = num_evals * size_of_fe_in_bytes;
        let num_prover_messages: usize = self.prover_messages.iter().map(|v| v.field_elements.len()).sum();
        let prover_msg_size_in_bytes = num_prover_messages * size_of_fe_in_bytes;
        let arg_size = size_bytes_comms_with_degree_bounds
            + size_bytes_comms_without_degree_bounds
            + size_bytes_proofs
            + prover_msg_size_in_bytes
            + evals_size_in_bytes;
        let stats = format!(
            "Argument size in bytes: {}\n\n\
             Number of commitments without degree bounds: {}\n\
             Size (in bytes) of commitments without degree bounds: {}\n\
             Number of commitments with degree bounds: {}\n\
             Size (in bytes) of commitments with degree bounds: {}\n\n\
             Number of evaluation proofs: {}\n\
             Size (in bytes) of evaluation proofs: {}\n\n\
             Number of evaluations: {}\n\
             Size (in bytes) of evaluations: {}\n\n\
             Number of field elements in prover messages: {}\n\
             Size (in bytes) of prover message: {}\n",
            arg_size,
            num_comms_without_degree_bounds,
            size_bytes_comms_without_degree_bounds,
            num_comms_with_degree_bounds,
            size_bytes_comms_with_degree_bounds,
            num_proofs,
            size_bytes_proofs,
            num_evals,
            evals_size_in_bytes,
            num_prover_messages,
            prover_msg_size_in_bytes,
        );
        add_to_trace!(|| "Statistics about proof", || stats);
    }
}
