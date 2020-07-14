use crate::{
    edwards_bls12::{Fq, Fr},
    templates::twisted_edwards_extended::{GroupAffine, GroupProjective},
};
use snarkos_errors::curves::GroupError;
use snarkos_models::{
    curves::{pairing_engine::AffineCurve, ModelParameters, MontgomeryModelParameters, TEModelParameters},
    field,
};
use snarkos_utilities::biginteger::BigInteger256;

use std::str::FromStr;

pub type EdwardsAffine = GroupAffine<EdwardsParameters>;
pub type EdwardsProjective = GroupProjective<EdwardsParameters>;

#[derive(Clone, Default, PartialEq, Eq)]
pub struct EdwardsParameters;

impl ModelParameters for EdwardsParameters {
    type BaseField = Fq;
    type ScalarField = Fr;
}

impl TEModelParameters for EdwardsParameters {
    type MontgomeryModelParameters = EdwardsParameters;

    /// Generated randomly
    const AFFINE_GENERATOR_COEFFS: (Self::BaseField, Self::BaseField) = (GENERATOR_X, GENERATOR_Y);
    /// COEFF_A = -1
    const COEFF_A: Fq = field!(
        Fq,
        BigInteger256([
            0x8cf500000000000e,
            0xe75281ef6000000e,
            0x49dc37a90b0ba012,
            0x55f8b2c6e710ab9,
        ])
    );
    /// COEFF_D = 3021
    const COEFF_D: Fq = field!(
        Fq,
        BigInteger256([
            0xd047ffffffff5e30,
            0xf0a91026ffff57d2,
            0x9013f560d102582,
            0x9fd242ca7be5700,
        ])
    );
    /// COFACTOR = 4
    const COFACTOR: &'static [u64] = &[4];
    /// COFACTOR_INV =
    /// 527778859339273151515551558673846658209717731602102048798421311598680340096
    const COFACTOR_INV: Fr = field!(
        Fr,
        BigInteger256([
            10836190823041854989,
            14880086764632731920,
            5023208332782666747,
            239524813690824359,
        ])
    );

    /// Multiplication by `a` is just negation.
    /// Is `a` 1 or -1?
    #[inline(always)]
    fn mul_by_a(elem: &Self::BaseField) -> Self::BaseField {
        -*elem
    }
}

impl MontgomeryModelParameters for EdwardsParameters {
    type TEModelParameters = EdwardsParameters;

    /// COEFF_A = 0x8D26E3FADA9010A26949031ECE3971B93952AD84D4753DDEDB748DA37E8F552
    const COEFF_A: Fq = field!(
        Fq,
        BigInteger256([
            13800168384327121454u64,
            6841573379969807446u64,
            12529593083398462246u64,
            853978956621483129u64,
        ])
    );
    /// COEFF_B = 0x9D8F71EEC83A44C3A1FBCEC6F5418E5C6154C2682B8AC231C5A3725C8170AAD
    const COEFF_B: Fq = field!(
        Fq,
        BigInteger256([
            7239382437352637935u64,
            14509846070439283655u64,
            5083066350480839936u64,
            1265663645916442191u64,
        ])
    );
}

impl FromStr for EdwardsAffine {
    type Err = GroupError;

    fn from_str(mut s: &str) -> Result<Self, Self::Err> {
        s = s.trim();
        if s.is_empty() {
            return Err(GroupError::ParsingEmptyString);
        }
        if s.len() < 3 {
            return Err(GroupError::InvalidString);
        }
        if !(s.starts_with('(') && s.ends_with(')')) {
            return Err(GroupError::InvalidString);
        }
        let mut point = Vec::new();
        for substr in s.split(|c| c == '(' || c == ')' || c == ',' || c == ' ') {
            if !substr.is_empty() {
                point.push(Fq::from_str(substr)?);
            }
        }
        if point.len() != 2 {
            return Err(GroupError::InvalidGroupElement);
        }
        let point = EdwardsAffine::new(point[0], point[1]);

        if !point.is_on_curve() {
            Err(GroupError::InvalidGroupElement)
        } else {
            Ok(point)
        }
    }
}

/// GENERATOR_X =
/// 7810607721416582242904415504650443951498042435501746664987470571546413371306
const GENERATOR_X: Fq = field!(
    Fq,
    BigInteger256([
        0x5bbc9878d817221d,
        0xd2b03489424e720,
        0x6b66f128c16bb3c9,
        0xdd3bff78733576d,
    ])
);

/// GENERATOR_Y =
/// 1867362672570137759132108893390349941423731440336755218616442213142473202417
const GENERATOR_Y: Fq = field!(
    Fq,
    BigInteger256([
        0x471517ae5e5e979e,
        0xd9c97f6a73a7ff83,
        0x85a95b45a5494402,
        0xfad27c9b545b1f0,
    ])
);
