use crate::{
    curves::{Field, PairingEngine},
    gadgets::{
        curves::{FieldGadget, GroupGadget},
        r1cs::{ConstraintSystem, SynthesisError},
        utilities::ToBytesGadget,
    },
};

use std::fmt::Debug;

pub trait PairingGadget<Pairing: PairingEngine, F: Field> {
    type G1Gadget: GroupGadget<Pairing::G1Projective, F>;
    type G2Gadget: GroupGadget<Pairing::G2Projective, F>;
    type G1PreparedGadget: ToBytesGadget<F> + Clone + Debug;
    type G2PreparedGadget: ToBytesGadget<F> + Clone + Debug;
    type GTGadget: FieldGadget<Pairing::Fqk, F> + Clone;

    fn miller_loop<CS: ConstraintSystem<F>>(
        cs: CS,
        p: &[Self::G1PreparedGadget],
        q: &[Self::G2PreparedGadget],
    ) -> Result<Self::GTGadget, SynthesisError>;

    fn final_exponentiation<CS: ConstraintSystem<F>>(
        cs: CS,
        p: &Self::GTGadget,
    ) -> Result<Self::GTGadget, SynthesisError>;

    fn pairing<CS: ConstraintSystem<F>>(
        mut cs: CS,
        p: Self::G1PreparedGadget,
        q: Self::G2PreparedGadget,
    ) -> Result<Self::GTGadget, SynthesisError> {
        let tmp = Self::miller_loop(cs.ns(|| "miller loop"), &[p], &[q])?;
        Self::final_exponentiation(cs.ns(|| "final_exp"), &tmp)
    }

    /// Computes a product of pairings.
    #[must_use]
    fn product_of_pairings<CS: ConstraintSystem<F>>(
        mut cs: CS,
        p: &[Self::G1PreparedGadget],
        q: &[Self::G2PreparedGadget],
    ) -> Result<Self::GTGadget, SynthesisError> {
        let miller_result = Self::miller_loop(&mut cs.ns(|| "Miller loop"), p, q)?;
        Self::final_exponentiation(&mut cs.ns(|| "Final Exp"), &miller_result)
    }

    fn prepare_g1<CS: ConstraintSystem<F>>(
        cs: CS,
        q: &Self::G1Gadget,
    ) -> Result<Self::G1PreparedGadget, SynthesisError>;

    fn prepare_g2<CS: ConstraintSystem<F>>(
        cs: CS,
        q: &Self::G2Gadget,
    ) -> Result<Self::G2PreparedGadget, SynthesisError>;
}
