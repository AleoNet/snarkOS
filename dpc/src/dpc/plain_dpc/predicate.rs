use crate::dpc::{plain_dpc::PlainDPCComponents, Predicate};

use snarkos_models::algorithms::SNARK;

use std::marker::PhantomData;

pub struct PrivatePredInput<C: PlainDPCComponents> {
    pub vk: <C::PredicateNIZK as SNARK>::VerificationParameters,
    pub proof: <C::PredicateNIZK as SNARK>::Proof,
}

impl<C: PlainDPCComponents> Default for PrivatePredInput<C> {
    fn default() -> Self {
        Self {
            vk: <C::PredicateNIZK as SNARK>::VerificationParameters::default(),
            proof: <C::PredicateNIZK as SNARK>::Proof::default(),
        }
    }
}

impl<C: PlainDPCComponents> Clone for PrivatePredInput<C> {
    fn clone(&self) -> Self {
        Self {
            vk: self.vk.clone(),
            proof: self.proof.clone(),
        }
    }
}

#[derive(Derivative)]
#[derivative(Clone(bound = "C: PlainDPCComponents"), Default(bound = "C: PlainDPCComponents"))]
pub struct DPCPredicate<C: PlainDPCComponents> {
    #[derivative(Default(value = "vec![0u8; 32]"))]
    identity: Vec<u8>,
    _components: PhantomData<C>,
}

impl<C: PlainDPCComponents> DPCPredicate<C> {
    pub fn new(identity: Vec<u8>) -> Self {
        Self {
            identity,
            _components: PhantomData,
        }
    }
}

impl<C: PlainDPCComponents> Predicate for DPCPredicate<C> {
    type PrivateWitness = PrivatePredInput<C>;
    type PublicInput = ();

    fn evaluate(&self, _p: &Self::PublicInput, _w: &Self::PrivateWitness) -> bool {
        unimplemented!()
    }

    fn into_compact_repr(&self) -> Vec<u8> {
        self.identity.clone()
    }
}
