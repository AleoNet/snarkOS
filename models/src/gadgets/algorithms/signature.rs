use crate::{
    algorithms::SignatureScheme,
    curves::Field,
    gadgets::{
        r1cs::ConstraintSystem,
        utilities::{alloc::AllocGadget, eq::EqGadget, uint::UInt8, ToBytesGadget},
    },
};
use snarkos_errors::gadgets::SynthesisError;

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
