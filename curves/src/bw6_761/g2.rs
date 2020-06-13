use crate::bw6_761::{Fq, Fr};
use snarkos_models::{
    curves::{Field, ModelParameters, SWModelParameters},
    field,
};
use snarkos_utilities::biginteger::{BigInteger384, BigInteger768};

#[derive(Clone, Default, PartialEq, Eq)]
pub struct Bls12_377G2Parameters;

impl ModelParameters for Bls12_377G2Parameters {
    type BaseField = Fq;
    type ScalarField = Fr;
}

impl SWModelParameters for Bls12_377G2Parameters {
    /// AFFINE_GENERATOR_COEFFS = (G2_GENERATOR_X, G2_GENERATOR_Y)
    const AFFINE_GENERATOR_COEFFS: (Self::BaseField, Self::BaseField) = (G2_GENERATOR_X, G2_GENERATOR_Y);
    /// COEFF_A = 0
    const COEFF_A: Fq = field!(Fq, BigInteger768([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]));
    /// COEFF_B = 4
    const COEFF_B: Fq = field!(
        Fq,
        BigInteger768([
            0x136efffffffe16c9,
            0x82cf5a6dcffe3319,
            0x6458c05f1f0e0741,
            0xd10ae605e52a4eda,
            0x41ca591c0266e100,
            0x7d0fd59c3626929f,
            0x9967dc004d00c112,
            0x1ccff9c033379af5,
            0x9ad6ec10a23f63af,
            0x5cec11251a72c235,
            0x8d18b1ae789ba83e,
            10403402007434220,
        ])
    );
    /// COFACTOR =
    /// 26642435879335816683987677701488073867751118270052650655942102502312977592501693353047140953112195348280268661194869
    const COFACTOR: &'static [u64] = &[
        0x3de5800000000075,
        0x832ba4061000003b,
        0xc61c554757551c0c,
        0xc856a0853c9db94c,
        0x2c77d5ac34cb12ef,
        0xad1972339049ce76,
    ];
    /// COFACTOR^(-1) mod r =
    /// 214911522365886453591244899095480747723790054550866810551297776298664428889000553861210287833206024638187939842124
    const COFACTOR_INV: Fr = field!(
        Fr,
        BigInteger384([
            14378295991815829998,
            14586153992421458638,
            9788477762582722914,
            12654821707953664524,
            15185631607604703397,
            26723985783783076,
        ])
    );

    #[inline(always)]
    fn mul_by_a(_elem: &Self::BaseField) -> Self::BaseField {
        Self::BaseField::zero()
    }
}

/// G2_GENERATOR_X =
///  6445332910596979336035888152774071626898886139774101364933948236926875073754470830732273879639675437155036544153105017729592600560631678554299562762294743927912429096636156401171909259073181112518725201388196280039960074422214428
pub const G2_GENERATOR_X: Fq = field!(
    Fq,
    BigInteger768([
        0x3d902a84cd9f4f78,
        0x864e451b8a9c05dd,
        0xc2b3c0d6646c5673,
        0x17a7682def1ecb9d,
        0xbe31a1e0fb768fe3,
        0x4df125e09b92d1a6,
        0x0943fce635b02ee9,
        0xffc8e7ad0605e780,
        0x8165c00a39341e95,
        0x8ccc2ae90a0f094f,
        0x73a8b8cc0ad09e0c,
        0x11027e203edd9f4,
    ])
);

/// G2_GENERATOR_Y =
/// 562923658089539719386922163444547387757586534741080263946953401595155211934630598999300396317104182598044793758153214972605680357108252243146746187917218885078195819486220416605630144001533548163105316661692978285266378674355041
pub const G2_GENERATOR_Y: Fq = field!(
    Fq,
    BigInteger768([
        0x9a159be4e773f67c,
        0x6b957244aa8f4e6b,
        0xa27b70c9c945a38c,
        0xacb6a09fda11d0ab,
        0x3abbdaa9bb6b1291,
        0xdbdf642af5694c36,
        0xb6360bb9560b369f,
        0xac0bd1e822b8d6da,
        0xfa355d17afe6945f,
        0x8d6a0fc1fbcad35e,
        0x72a63c7874409840,
        0x114976e5b0db280,
    ])
);
