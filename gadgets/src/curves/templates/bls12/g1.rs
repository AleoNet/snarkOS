use crate::curves::templates::bls12::AffineGadget;
use snarkos_curves::templates::bls12::{Bls12Parameters, G1Prepared};
use snarkos_models::{
    curves::ProjectiveCurve,
    gadgets::{
        curves::{FpGadget, GroupGadget},
        r1cs::ConstraintSystem,
        utilities::{uint8::UInt8, ToBytesGadget},
    },
};
use snarkvm_errors::gadgets::SynthesisError;

use std::fmt::Debug;

pub type G1Gadget<P> = AffineGadget<
    <P as Bls12Parameters>::G1Parameters,
    <P as Bls12Parameters>::Fp,
    FpGadget<<P as Bls12Parameters>::Fp>,
>;

#[derive(Derivative)]
#[derivative(Clone(bound = "G1Gadget<P>: Clone"), Debug(bound = "G1Gadget<P>: Debug"))]
pub struct G1PreparedGadget<P: Bls12Parameters>(pub G1Gadget<P>);

impl<P: Bls12Parameters> G1PreparedGadget<P> {
    pub fn get_value(&self) -> Option<G1Prepared<P>> {
        Some(G1Prepared::from_affine(self.0.get_value().unwrap().into_affine()))
    }

    pub fn from_affine<CS: ConstraintSystem<P::Fp>>(_cs: CS, q: &G1Gadget<P>) -> Result<Self, SynthesisError> {
        Ok(G1PreparedGadget(q.clone()))
    }
}

impl<P: Bls12Parameters> ToBytesGadget<P::Fp> for G1PreparedGadget<P> {
    #[inline]
    fn to_bytes<CS: ConstraintSystem<P::Fp>>(&self, mut cs: CS) -> Result<Vec<UInt8>, SynthesisError> {
        self.0.to_bytes(&mut cs.ns(|| "g_alpha to bytes"))
    }

    fn to_bytes_strict<CS: ConstraintSystem<P::Fp>>(&self, cs: CS) -> Result<Vec<UInt8>, SynthesisError> {
        self.to_bytes(cs)
    }
}
