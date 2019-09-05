use crate::{
    algorithms::SignatureScheme,
    curves::Field,
    gadgets::{
        r1cs::{ConstraintSystem, SynthesisError},
        utilities::{alloc::AllocGadget, eq::EqGadget, uint8::UInt8, ToBytesGadget},
    },
};

pub trait SignaturePublicKeyRandomizationGadget<S: SignatureScheme, F: Field> {
    type ParametersGadget: AllocGadget<S::Parameters, F> + Clone;
    type PublicKeyGadget: ToBytesGadget<F> + EqGadget<F> + AllocGadget<S::PublicKey, F> + Clone;

    fn check_randomization_gadget<CS: ConstraintSystem<F>>(
        cs: CS,
        parameters: &Self::ParametersGadget,
        public_key: &Self::PublicKeyGadget,
        randomness: &[UInt8],
    ) -> Result<Self::PublicKeyGadget, SynthesisError>;
}
