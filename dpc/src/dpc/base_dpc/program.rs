use crate::dpc::base_dpc::BaseDPCComponents;

use snarkos_models::{algorithms::SNARK, dpc::Program};

use std::marker::PhantomData;

pub struct PrivateProgramInput<C: BaseDPCComponents> {
    pub verification_key: <C::ProgramSNARK as SNARK>::VerificationParameters,
    pub proof: <C::ProgramSNARK as SNARK>::Proof,
}

impl<C: BaseDPCComponents> Clone for PrivateProgramInput<C> {
    fn clone(&self) -> Self {
        Self {
            verification_key: self.verification_key.clone(),
            proof: self.proof.clone(),
        }
    }
}

#[derive(Derivative)]
#[derivative(Clone(bound = "C: BaseDPCComponents"), Default(bound = "C: BaseDPCComponents"))]
pub struct DPCProgram<C: BaseDPCComponents> {
    #[derivative(Default(value = "vec![0u8; 32]"))]
    identity: Vec<u8>,
    _components: PhantomData<C>,
}

impl<C: BaseDPCComponents> DPCProgram<C> {
    pub fn new(identity: Vec<u8>) -> Self {
        Self {
            identity,
            _components: PhantomData,
        }
    }
}

impl<C: BaseDPCComponents> Program for DPCProgram<C> {
    type PrivateWitness = PrivateProgramInput<C>;
    type PublicInput = ();

    fn evaluate(&self, _p: &Self::PublicInput, _w: &Self::PrivateWitness) -> bool {
        unimplemented!()
    }

    fn into_compact_repr(&self) -> Vec<u8> {
        self.identity.clone()
    }
}
