use crate::bls12_377::{g1::Bls12_377G1Parameters, Fq, Fq2, Fr};
use snarkos_models::{
    curves::{Field, ModelParameters, SWModelParameters},
    field,
};
use snarkvm_utilities::biginteger::{BigInteger256, BigInteger384};

#[derive(Clone, Default, PartialEq, Eq)]
pub struct Bls12_377G2Parameters;

impl ModelParameters for Bls12_377G2Parameters {
    type BaseField = Fq2;
    type ScalarField = Fr;
}

impl SWModelParameters for Bls12_377G2Parameters {
    /// AFFINE_GENERATOR_COEFFS = (G2_GENERATOR_X, G2_GENERATOR_Y)
    const AFFINE_GENERATOR_COEFFS: (Self::BaseField, Self::BaseField) = (G2_GENERATOR_X, G2_GENERATOR_Y);
    /// COEFF_A = [0, 0]
    const COEFF_A: Fq2 = field!(Fq2, Bls12_377G1Parameters::COEFF_A, Bls12_377G1Parameters::COEFF_A,);
    // As per https://eprint.iacr.org/2012/072.pdf,
    // this curve has b' = b/i, where b is the COEFF_B of G1, and x^6 -i is
    // the irreducible poly used to extend from Fp2 to Fp12.
    // In our case, i = u (App A.3, T_6).
    /// COEFF_B = [0,
    /// 155198655607781456406391640216936120121836107652948796323930557600032281009004493664981332883744016074664192874906]
    const COEFF_B: Fq2 = field!(
        Fq2,
        field!(Fq, BigInteger384([0, 0, 0, 0, 0, 0])),
        field!(
            Fq,
            BigInteger384([
                9255502405446297221,
                10229180150694123945,
                9215585410771530959,
                13357015519562362907,
                5437107869987383107,
                16259554076827459,
            ])
        ),
    );
    /// COFACTOR =
    /// 7923214915284317143930293550643874566881017850177945424769256759165301436616933228209277966774092486467289478618404761412630691835764674559376407658497
    const COFACTOR: &'static [u64] = &[
        0x0000000000000001,
        0x452217cc90000000,
        0xa0f3622fba094800,
        0xd693e8c36676bd09,
        0x8c505634fae2e189,
        0xfbb36b00e1dcc40c,
        0xddd88d99a6f6a829,
        0x26ba558ae9562a,
    ];
    /// COFACTOR_INV = COFACTOR^{-1} mod r
    /// = 6764900296503390671038341982857278410319949526107311149686707033187604810669
    const COFACTOR_INV: Fr = field!(
        Fr,
        BigInteger256([
            15499857013495546999,
            4613531467548868169,
            14546778081091178013,
            549402535258503313,
        ])
    );

    #[inline(always)]
    fn mul_by_a(_: &Self::BaseField) -> Self::BaseField {
        Self::BaseField::zero()
    }
}

pub const G2_GENERATOR_X: Fq2 = field!(Fq2, G2_GENERATOR_X_C0, G2_GENERATOR_X_C1);
pub const G2_GENERATOR_Y: Fq2 = field!(Fq2, G2_GENERATOR_Y_C0, G2_GENERATOR_Y_C1);

/// G2_GENERATOR_X_C0 =
/// 233578398248691099356572568220835526895379068987715365179118596935057653620464273615301663571204657964920925606294
pub const G2_GENERATOR_X_C0: Fq = field!(
    Fq,
    BigInteger384([
        0x68904082f268725b,
        0x668f2ea74f45328b,
        0xebca7a65802be84f,
        0x1e1850f4c1ada3e6,
        0x830dc22d588ef1e9,
        0x1862a81767c0982,
    ])
);

/// G2_GENERATOR_X_C1 =
/// 140913150380207355837477652521042157274541796891053068589147167627541651775299824604154852141315666357241556069118
pub const G2_GENERATOR_X_C1: Fq = field!(
    Fq,
    BigInteger384([
        0x5f02a915c91c7f39,
        0xf8c553ba388da2a7,
        0xd51a416dbd198850,
        0xe943c6f38ae3073a,
        0xffe24aa8259a4981,
        0x11853391e73dfdd,
    ])
);

/// G2_GENERATOR_Y_C0 =
/// 63160294768292073209381361943935198908131692476676907196754037919244929611450776219210369229519898517858833747423
pub const G2_GENERATOR_Y_C0: Fq = field!(
    Fq,
    BigInteger384([
        0xd5b19b897881430f,
        0x5be9118a5b371ed,
        0x6063f91f86c131ee,
        0x3244a61be8f4ec19,
        0xa02e425b9f9a3a12,
        0x18af8c04f3360d2,
    ])
);

/// G2_GENERATOR_Y_C1 =
/// 149157405641012693445398062341192467754805999074082136895788947234480009303640899064710353187729182149407503257491
pub const G2_GENERATOR_Y_C1: Fq = field!(
    Fq,
    BigInteger384([
        0x57601ac71a5b96f5,
        0xe99acc1714f2440e,
        0x2339612f10118ea9,
        0x8321e68a3b1cd722,
        0x2b543b050cc74917,
        0x590182b396c112,
    ])
);
