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
    type PrivateKeyGadget: EqGadget<F> + ToBytesGadget<F> + AllocGadget<E::PrivateKey, F> + Clone + Sized + Debug;
    type PublicKeyGadget: EqGadget<F> + ToBytesGadget<F> + AllocGadget<E::PublicKey, F> + Clone + Sized + Debug;
    type CiphertextGadget: EqGadget<F> + ToBytesGadget<F> + AllocGadget<E::Ciphertext, F> + Clone + Sized + Debug;
    type PlaintextGadget: EqGadget<F> + ToBytesGadget<F> + AllocGadget<E::Plaintext, F> + Clone + Sized + Debug;

    fn check_encryption_gadget<CS: ConstraintSystem<F>>(
        cs: CS,
        parameters: &Self::ParametersGadget,
        public_key: &Self::PublicKeyGadget,
        input: &Self::PlaintextGadget,
    ) -> Result<Self::CiphertextGadget, SynthesisError>;

    fn check_decryption_gadget<CS: ConstraintSystem<F>>(
        cs: CS,
        parameters: &Self::ParametersGadget,
        private_key: &Self::PrivateKeyGadget,
        input: &Self::CiphertextGadget,
    ) -> Result<Self::PlaintextGadget, SynthesisError>;
}
