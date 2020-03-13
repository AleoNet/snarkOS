use crate::dpc::{delegable_payment_dpc::PaymentDPCComponents, Predicate};

use snarkos_models::algorithms::{CommitmentScheme, SNARK};

use std::marker::PhantomData;

pub struct PrivatePredInput<C: PaymentDPCComponents> {
    pub vk: <C::PredicateNIZK as SNARK>::VerificationParameters,
    pub proof: <C::PredicateNIZK as SNARK>::Proof,
    pub value_commitment: <C::ValueComm as CommitmentScheme>::Output,
    pub value_commitment_randomness: <C::ValueComm as CommitmentScheme>::Randomness,
}

impl<C: PaymentDPCComponents> Default for PrivatePredInput<C> {
    fn default() -> Self {
        Self {
            vk: <C::PredicateNIZK as SNARK>::VerificationParameters::default(),
            proof: <C::PredicateNIZK as SNARK>::Proof::default(),
            value_commitment: <C::ValueComm as CommitmentScheme>::Output::default(),
            value_commitment_randomness: <C::ValueComm as CommitmentScheme>::Randomness::default(),
        }
    }
}

impl<C: PaymentDPCComponents> Clone for PrivatePredInput<C> {
    fn clone(&self) -> Self {
        Self {
            vk: self.vk.clone(),
            proof: self.proof.clone(),
            value_commitment: self.value_commitment.clone(),
            value_commitment_randomness: self.value_commitment_randomness.clone(),
        }
    }
}

#[derive(Derivative)]
#[derivative(Clone(bound = "C: PaymentDPCComponents"), Default(bound = "C: PaymentDPCComponents"))]
pub struct DPCPredicate<C: PaymentDPCComponents> {
    #[derivative(Default(value = "vec![0u8; 32]"))]
    identity: Vec<u8>,
    _components: PhantomData<C>,
}

impl<C: PaymentDPCComponents> DPCPredicate<C> {
    pub fn new(identity: Vec<u8>) -> Self {
        Self {
            identity,
            _components: PhantomData,
        }
    }
}

impl<C: PaymentDPCComponents> Predicate for DPCPredicate<C> {
    type PrivateWitness = PrivatePredInput<C>;
    type PublicInput = ();

    fn evaluate(&self, _p: &Self::PublicInput, _w: &Self::PrivateWitness) -> bool {
        unimplemented!()
    }

    fn into_compact_repr(&self) -> Vec<u8> {
        self.identity.clone()
    }
}
