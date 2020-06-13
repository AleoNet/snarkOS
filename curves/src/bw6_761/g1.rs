use crate::bw6_761::{Fq, Fr};
use snarkos_models::{
    curves::{Field, ModelParameters, SWModelParameters},
    field,
};
use snarkos_utilities::biginteger::{BigInteger384, BigInteger768};

#[derive(Clone, Default, PartialEq, Eq)]
pub struct BW6_761G1Parameters;

impl ModelParameters for BW6_761G1Parameters {
    type BaseField = Fq;
    type ScalarField = Fr;
}

impl SWModelParameters for BW6_761G1Parameters {
    /// AFFINE_GENERATOR_COEFFS = (G1_GENERATOR_X, G1_GENERATOR_Y)
    const AFFINE_GENERATOR_COEFFS: (Self::BaseField, Self::BaseField) = (G1_GENERATOR_X, G1_GENERATOR_Y);
    /// COEFF_A = 0
    const COEFF_A: Fq = field!(Fq, BigInteger768([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]));
    /// COEFF_B = -1
    const COEFF_B: Fq = field!(
        Fq,
        BigInteger768([
            0xf29a000000007ab6,
            0x8c391832e000739b,
            0x77738a6b6870f959,
            0xbe36179047832b03,
            0x84f3089e56574722,
            0xc5a3614ac0b1d984,
            0x5c81153f4906e9fe,
            0x4d28be3a9f55c815,
            0xd72c1d6f77d5f5c5,
            0x73a18e069ac04458,
            0xf9dfaa846595555f,
            0xd0f0a60a5be58c,
        ])
    );
    /// COFACTOR =
    /// 26642435879335816683987677701488073867751118270052650655942102502312977592501693353047140953112195348280268661194876
    const COFACTOR: &'static [u64] = &[
        0x3de580000000007c,
        0x832ba4061000003b,
        0xc61c554757551c0c,
        0xc856a0853c9db94c,
        0x2c77d5ac34cb12ef,
        0xad1972339049ce76,
    ];
    /// COFACTOR^(-1) mod r =
    /// 91141326767669940707819291241958318717982251277713150053234367522357946997763584490607453720072232540829942217804
    const COFACTOR_INV: Fr = field!(
        Fr,
        BigInteger384([
            489703175600125849,
            3883341943836920852,
            1678256062427438196,
            5848789333018172718,
            7127967896440782320,
            71512347676739162,
        ])
    );

    #[inline(always)]
    fn mul_by_a(_elem: &Self::BaseField) -> Self::BaseField {
        Self::BaseField::zero()
    }
}

/// G1_GENERATOR_X =
/// 6238772257594679368032145693622812838779005809760824733138787810501188623461307351759238099287535516224314149266511977132140828635950940021790489507611754366317801811090811367945064510304504157188661901055903167026722666149426237
pub const G1_GENERATOR_X: Fq = field!(
    Fq,
    BigInteger768([
        0xd6e42d7614c2d770,
        0x4bb886eddbc3fc21,
        0x64648b044098b4d2,
        0x1a585c895a422985,
        0xf1a9ac17cf8685c9,
        0x352785830727aea5,
        0xddf8cb12306266fe,
        0x6913b4bfbc9e949a,
        0x3a4b78d67ba5f6ab,
        0x0f481c06a8d02a04,
        0x91d4e7365c43edac,
        0xf4d17cd48beca5,
    ])
);

/// G1_GENERATOR_Y =
/// 2101735126520897423911504562215834951148127555913367997162789335052900271653517958562461315794228241561913734371411178226936527683203879553093934185950470971848972085321797958124416462268292467002957525517188485984766314758624099
pub const G1_GENERATOR_Y: Fq = field!(
    Fq,
    BigInteger768([
        0x97e805c4bd16411f,
        0x870d844e1ee6dd08,
        0x1eba7a37cb9eab4d,
        0xd544c4df10b9889a,
        0x8fe37f21a33897be,
        0xe9bf99a43a0885d2,
        0xd7ee0c9e273de139,
        0xaa6a9ec7a38dd791,
        0x8f95d3fcf765da8e,
        0x42326e7db7357c99,
        0xe217e407e218695f,
        0x9d1eb23b7cf684,
    ])
);
