use crate::dpc::base_dpc::BaseDPCComponents;

use snarkos_models::{algorithms::SNARK, dpc::Predicate};

use std::marker::PhantomData;

pub struct PrivatePredicateInput<C: BaseDPCComponents> {
    pub verification_key: <C::PredicateSNARK as SNARK>::VerificationParameters,
    pub proof: <C::PredicateSNARK as SNARK>::Proof,
}

impl<C: BaseDPCComponents> Default for PrivatePredicateInput<C> {
    fn default() -> Self {
        Self {
            verification_key: <C::PredicateSNARK as SNARK>::VerificationParameters::default(),
            proof: <C::PredicateSNARK as SNARK>::Proof::default(),
        }
    }
}

impl<C: BaseDPCComponents> Clone for PrivatePredicateInput<C> {
    fn clone(&self) -> Self {
        Self {
            verification_key: self.verification_key.clone(),
            proof: self.proof.clone(),
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
