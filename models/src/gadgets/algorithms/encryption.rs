use crate::{
    algorithms::EncryptionScheme,
    curves::Field,
    gadgets::{
        r1cs::ConstraintSystem,
        utilities::{alloc::AllocGadget, eq::EqGadget, ToBytesGadget},
    },
};
use snarkos_errors::gadgets::SynthesisError;

use std::fmt::Debug;

pub trait EncryptionGadget<E: EncryptionScheme, F: Field> {
    type ParametersGadget: AllocGadget<E::Parameters, F> + Clone;
    type PrivateKeyGadget: AllocGadget<E::PrivateKey, F> + ToBytesGadget<F> + Clone + Sized + Debug;
    type PublicKeyGadget: AllocGadget<E::PublicKey, F> + EqGadget<F> + ToBytesGadget<F> + Clone + Sized + Debug;
    type CiphertextGadget: AllocGadget<Vec<E::Text>, F> + EqGadget<F> + Clone + Sized + Debug;
    type PlaintextGadget: AllocGadget<Vec<E::Text>, F> + EqGadget<F> + Clone + Sized + Debug;
    type RandomnessGadget: AllocGadget<E::Randomness, F> + Clone + Sized + Debug;
    type BlindingExponentGadget: AllocGadget<Vec<E::BlindingExponent>, F> + Clone + Sized + Debug;

    fn check_public_key_gadget<CS: ConstraintSystem<F>>(
        cs: CS,
        parameters: &Self::ParametersGadget,
        private_key: &Self::PrivateKeyGadget,
    ) -> Result<Self::PublicKeyGadget, SynthesisError>;

    fn check_encryption_gadget<CS: ConstraintSystem<F>>(
        cs: CS,
        parameters: &Self::ParametersGadget,
        randomness: &Self::RandomnessGadget,
        public_key: &Self::PublicKeyGadget,
        input: &Self::PlaintextGadget,
        blinding_exponents: &Self::BlindingExponentGadget,
    ) -> Result<Self::CiphertextGadget, SynthesisError>;
}
