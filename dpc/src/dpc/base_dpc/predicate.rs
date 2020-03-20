use crate::dpc::{base_dpc::BaseDPCComponents, Predicate};

use snarkos_models::algorithms::{CommitmentScheme, SNARK};

use std::marker::PhantomData;

pub struct PrivatePredicateInput<C: BaseDPCComponents> {
    pub verification_key: <C::PredicateSNARK as SNARK>::VerificationParameters,
    pub proof: <C::PredicateSNARK as SNARK>::Proof,
    pub value_commitment: <C::ValueCommitment as CommitmentScheme>::Output,
    pub value_commitment_randomness: <C::ValueCommitment as CommitmentScheme>::Randomness,
}

impl<C: BaseDPCComponents> Default for PrivatePredicateInput<C> {
    fn default() -> Self {
        Self {
            verification_key: <C::PredicateSNARK as SNARK>::VerificationParameters::default(),
            proof: <C::PredicateSNARK as SNARK>::Proof::default(),
            value_commitment: <C::ValueCommitment as CommitmentScheme>::Output::default(),
            value_commitment_randomness: <C::ValueCommitment as CommitmentScheme>::Randomness::default(),
        }
    }
}

impl<C: BaseDPCComponents> Clone for PrivatePredicateInput<C> {
    fn clone(&self) -> Self {
        Self {
            verification_key: self.verification_key.clone(),
            proof: self.proof.clone(),
            value_commitment: self.value_commitment.clone(),
            value_commitment_randomness: self.value_commitment_randomness.clone(),
        }
    }
}

#[derive(Derivative)]
#[derivative(Clone(bound = "C: BaseDPCComponents"), Default(bound = "C: BaseDPCComponents"))]
pub struct DPCPredicate<C: BaseDPCComponents> {
    #[derivative(Default(value = "vec![0u8; 32]"))]
    identity: Vec<u8>,
    _components: PhantomData<C>,
}

impl<C: BaseDPCComponents> DPCPredicate<C> {
    pub fn new(identity: Vec<u8>) -> Self {
        Self {
            identity,
            _components: PhantomData,
        }
    }
}

impl<C: BaseDPCComponents> Predicate for DPCPredicate<C> {
    type PrivateWitness = PrivatePredicateInput<C>;
    type PublicInput = ();

    fn evaluate(&self, _p: &Self::PublicInput, _w: &Self::PrivateWitness) -> bool {
        unimplemented!()
    }

    fn into_compact_repr(&self) -> Vec<u8> {
        self.identity.clone()
    }
}
