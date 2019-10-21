use crate::{
    sw6::{Fq, Fq6, Fr, G2Affine, SW6},
    templates::short_weierstrass::short_weierstrass_jacobian::{GroupAffine, GroupProjective},
};
use snarkos_models::{
    curves::{ModelParameters, PairingCurve, PairingEngine, SWModelParameters},
    field,
};
use snarkos_utilities::biginteger::{BigInteger384, BigInteger832};

pub type G1Affine = GroupAffine<SW6G1Parameters>;
pub type G1Projective = GroupProjective<SW6G1Parameters>;

impl PairingCurve for G1Affine {
    type Engine = SW6;
    type PairWith = G2Affine;
    type PairingResult = Fq6;
    type Prepared = Self;

    fn prepare(&self) -> Self::Prepared {
        self.clone()
    }

    fn pairing_with(&self, other: &Self::PairWith) -> Self::PairingResult {
        SW6::pairing(*self, *other)
    }
}

#[derive(Copy, Clone, Default, PartialEq, Eq)]
pub struct SW6G1Parameters;

impl ModelParameters for SW6G1Parameters {
    type BaseField = Fq;
    type ScalarField = Fr;
}

impl SWModelParameters for SW6G1Parameters {
    /// AFFINE_GENERATOR_COEFFS = (G1_GENERATOR_X, G1_GENERATOR_Y)
    const AFFINE_GENERATOR_COEFFS: (Self::BaseField, Self::BaseField) = (G1_GENERATOR_X, G1_GENERATOR_Y);
    /// COEFF_A = 5
    const COEFF_A: Fq = field!(
        Fq,
        BigInteger832([
            0x781c76643018bd7a,
            0x64f3a5a4f1d1ad48,
            0xd2f8a1eb4f72692d,
            0xc35eb123c6ed72ca,
            0xb58d6bcfd32de058,
            0x841eab13b02a492c,
            0x4b70dc5a54c487e7,
            0x2f231a8808a74c59,
            0x5e2915154d70b050,
            0x8a40fa16f37a6b37,
            0xd01980093a72c54b,
            0xef6845c25398004c,
            0x48,
        ])
    );
    /// COEFF_B = 17764315118651679038286329069295091506801468118146712649886336045535808055361274148466772191243305528312843236347777260247138934336850548243151534538734724191505953341403463040067571652261229308333392040104884438208594329793895206056414
    const COEFF_B: Fq = field!(
        Fq,
        BigInteger832([
            0xec5bd271ad37429,
            0x9db8ac843ecca28a,
            0x94f29bcb7e01bc74,
            0x1b0bebb77bb5af0,
            0x75b8cef4aa27ee17,
            0xb5767ae80812cf6b,
            0x592fa41e377a0d8c,
            0xb6c6deedbb52df3e,
            0xcb1343e488737fd4,
            0x878020734d05b5a9,
            0x2f51354eddfa069a,
            0x498e2ecdc545243e,
            0x2c2,
        ])
    );
    /// COFACTOR =
    /// 86482221941698704497288378992285180119495364068003923046442785886272123124361700722982503222189455144364945735564951561028
    const COFACTOR: &'static [u64] = &[
        0x5657b9b57b942344,
        0x84f9a65f3bd54eaf,
        0x5ea4214e35cd127,
        0xe3cbcbc14ec1501d,
        0xf196cb845a3092ab,
        0x7e14627ad0e19017,
        0x217db4,
    ];
    /// COFACTOR^(-1) mod r =
    /// 163276846538158998893990986356139314746223949404500031940624325017036397274793417940375498603127780919653358641788
    const COFACTOR_INV: Fr = field!(
        Fr,
        BigInteger384([
            4179837108212676264,
            15545810469293120493,
            13202863094424182470,
            9506285060796071546,
            9248558385029790142,
            87030208545296111,
        ])
    );
}

/// G1_GENERATOR_X =
/// 5511163824921585887915590525772884263960974614921003940645351443740084257508990841338974915037175497689287870585840954231884082785026301437744745393958283053278991955159266640440849940136976927372133743626748847559939620888818486853646
pub const G1_GENERATOR_X: Fq = field!(
    Fq,
    BigInteger832([
        0x5901480e5bc22290,
        0x20024afcdb9bd3a9,
        0x12dc18ff416e8138,
        0x28c69aa0ea223e18,
        0xafb1524a1eb7efe6,
        0x3d5c34edc3764ca2,
        0x736c2230c8466ce9,
        0xacfaa04e051014f1,
        0x5d5ff82f00ff2964,
        0x64c13ba270a26eaf,
        0x50e9864b56ab172e,
        0xd8370826a322499e,
        0x00000000000006f1,
    ])
);

/// G1_GENERATOR_Y =
/// 7913123550914612057135582061699117755797758113868200992327595317370485234417808273674357776714522052694559358668442301647906991623400754234679697332299689255516547752391831738454121261248793568285885897998257357202903170202349380518443
pub const G1_GENERATOR_Y: Fq = field!(
    Fq,
    BigInteger832([
        0x8af8b64b402e1953,
        0xd1bbceb3a258ea51,
        0xdca9efa3140aaa0d,
        0x807a610058ddedb2,
        0xeb898562fe88076c,
        0x0e4342ca56dd8ce2,
        0x4f5528d29f1bde9a,
        0xf18b0c6c19feb372,
        0x94503ac2fac9199c,
        0xffc86a8aff08ea34,
        0xf7b1295214735d8c,
        0x44eda9e0f55edd10,
        0x0000000000000ef3,
    ])
);
