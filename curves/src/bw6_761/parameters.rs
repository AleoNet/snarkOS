use crate::{
    bw6_761::{g1::BW6_761G1Parameters, g2::Bls12_377G2Parameters, Fq, Fq3Parameters, Fq6, Fq6Parameters},
    templates::bw6::{
        BW6Parameters,
        G1Affine as BW6G1Affine,
        G1Prepared,
        G1Projective as BW6G1Projective,
        G2Affine as BW6G2Affine,
        G2Prepared,
        G2Projective as BW6G2Projective,
        TwistType,
        BW6,
    },
};
use snarkos_models::curves::{PairingCurve, PairingEngine};
use snarkos_utilities::biginteger::BigInteger768 as BigInteger;

pub struct BW6_761Parameters;

impl BW6Parameters for BW6_761Parameters {
    type Fp = Fq;
    type Fp3Params = Fq3Parameters;
    type Fp6Params = Fq6Parameters;
    type G1Parameters = BW6_761G1Parameters;
    type G2Parameters = Bls12_377G2Parameters;

    // X+1
    const ATE_LOOP_COUNT_1: &'static [u64] = &[0x8508c00000000002];
    const ATE_LOOP_COUNT_1_IS_NEGATIVE: bool = false;
    // X^3-X^2-X
    const ATE_LOOP_COUNT_2: &'static [i8] = &[
        -1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 1, 0, 0, -1, 0, 1, 0, -1, 0, 0, 0, 0, -1, 0,
        1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 1, 0, 1, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, -1, 0, -1, 0, 0, 0, 0, -1,
        0, 0, 1, 0, 0, 0, -1, 0, 0, -1, 0, 1, 0, -1, 0, 0, 0, 1, 0, 0, 1, 0, -1, 0, 1, 0, 1, 0, 0, 0, 1, 0, -1, 0, -1,
        0, 0, 0, 0, 0, 1, 0, 0, 1,
    ];
    const ATE_LOOP_COUNT_2_IS_NEGATIVE: bool = false;
    const TWIST_TYPE: TwistType = TwistType::M;
    const X: BigInteger = BigInteger([
        0x8508c00000000001,
        0x0,
        0x0,
        0x0,
        0x0,
        0x0,
        0x0,
        0x0,
        0x0,
        0x0,
        0x0,
        0x0,
    ]);
    /// `x` is positive.
    const X_IS_NEGATIVE: bool = false;
}

pub type BW6_761 = BW6<BW6_761Parameters>;

pub type G1Affine = BW6G1Affine<BW6_761Parameters>;
pub type G1Projective = BW6G1Projective<BW6_761Parameters>;

pub type G2Affine = BW6G2Affine<BW6_761Parameters>;
pub type G2Projective = BW6G2Projective<BW6_761Parameters>;

impl PairingCurve for G1Affine {
    type Engine = BW6_761;
    type PairWith = G2Affine;
    type PairingResult = Fq6;
    type Prepared = G1Prepared<BW6_761Parameters>;

    fn prepare(&self) -> Self::Prepared {
        Self::Prepared::from(*self)
    }

    fn pairing_with(&self, other: &Self::PairWith) -> Self::PairingResult {
        BW6_761::pairing(*self, *other)
    }
}

impl PairingCurve for G2Affine {
    type Engine = BW6_761;
    type PairWith = G1Affine;
    type PairingResult = Fq6;
    type Prepared = G2Prepared<BW6_761Parameters>;

    fn prepare(&self) -> Self::Prepared {
        Self::Prepared::from(*self)
    }

    fn pairing_with(&self, other: &Self::PairWith) -> Self::PairingResult {
        BW6_761::pairing(*other, *self)
    }
}
