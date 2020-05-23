use crate::{
    curves::Field,
    gadgets::{
        r1cs::ConstraintSystem,
        utilities::{alloc::AllocGadget, ToBitsGadget, ToBytesGadget},
    },
};
use snarkvm_errors::gadgets::SynthesisError;
use snarkvm_models::algorithms::SNARK;

pub trait SNARKVerifierGadget<N: SNARK, F: Field> {
    type VerificationKeyGadget: AllocGadget<N::VerificationParameters, F> + ToBytesGadget<F>;
    type ProofGadget: AllocGadget<N::Proof, F>;

    fn check_verify<'a, CS: ConstraintSystem<F>, I: Iterator<Item = &'a T>, T: 'a + ToBitsGadget<F> + ?Sized>(
        cs: CS,
        verification_key: &Self::VerificationKeyGadget,
        input: I,
        proof: &Self::ProofGadget,
    ) -> Result<(), SynthesisError>;
}
