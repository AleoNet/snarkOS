use crate::templates::{
    bls12::Bls12Parameters,
    short_weierstrass::short_weierstrass_jacobian::{GroupAffine, GroupProjective},
};
use snarkos_models::curves::{pairing_engine::AffineCurve, Zero};
use snarkos_utilities::bytes::ToBytes;

use std::io::{Result as IoResult, Write};

pub type G1Affine<P> = GroupAffine<<P as Bls12Parameters>::G1Parameters>;
pub type G1Projective<P> = GroupProjective<<P as Bls12Parameters>::G1Parameters>;

#[derive(Derivative)]
#[derivative(
    Clone(bound = "P: Bls12Parameters"),
    Debug(bound = "P: Bls12Parameters"),
    PartialEq(bound = "P: Bls12Parameters"),
    Eq(bound = "P: Bls12Parameters")
)]
pub struct G1Prepared<P: Bls12Parameters>(pub G1Affine<P>);

impl<P: Bls12Parameters> G1Prepared<P> {
    pub fn is_zero(&self) -> bool {
        self.0.is_zero()
    }

    pub fn from_affine(p: G1Affine<P>) -> Self {
        G1Prepared(p)
    }
}

impl<P: Bls12Parameters> Default for G1Prepared<P> {
    fn default() -> Self {
        G1Prepared(G1Affine::<P>::prime_subgroup_generator())
    }
}

impl<P: Bls12Parameters> ToBytes for G1Prepared<P> {
    fn write<W: Write>(&self, writer: W) -> IoResult<()> {
        self.0.write(writer)
    }
}
