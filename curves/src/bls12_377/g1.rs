use crate::bls12_377::{Fq, Fr};
use snarkos_models::{
    curves::{Field, ModelParameters, SWModelParameters},
    field,
};
use snarkos_utilities::biginteger::{BigInteger256, BigInteger384};

#[derive(Clone, Default, PartialEq, Eq)]
pub struct Bls12_377G1Parameters;

impl ModelParameters for Bls12_377G1Parameters {
    type BaseField = Fq;
    type ScalarField = Fr;
}

impl SWModelParameters for Bls12_377G1Parameters {
    /// AFFINE_GENERATOR_COEFFS = (G1_GENERATOR_X, G1_GENERATOR_Y)
    const AFFINE_GENERATOR_COEFFS: (Self::BaseField, Self::BaseField) = (G1_GENERATOR_X, G1_GENERATOR_Y);
    /// COEFF_A = 0
    const COEFF_A: Fq = field!(Fq, BigInteger384([0x0, 0x0, 0x0, 0x0, 0x0, 0x0]));
    /// COEFF_B = 1
    const COEFF_B: Fq = field!(
        Fq,
        BigInteger384([
            0x2cdffffffffff68,
            0x51409f837fffffb1,
            0x9f7db3a98a7d3ff2,
            0x7b4e97b76e7c6305,
            0x4cf495bf803c84e8,
            0x8d6661e2fdf49a,
        ])
    );
    /// COFACTOR = (x - 1)^2 / 3  = 30631250834960419227450344600217059328
    const COFACTOR: &'static [u64] = &[0x0, 0x170b5d4430000000];
    /// COFACTOR_INV = COFACTOR^{-1} mod r
    /// = 5285428838741532253824584287042945485047145357130994810877
    const COFACTOR_INV: Fr = field!(
        Fr,
        BigInteger256([
            2013239619100046060,
            4201184776506987597,
            2526766393982337036,
            1114629510922847535,
        ])
    );

    #[inline(always)]
    fn mul_by_a(_: &Self::BaseField) -> Self::BaseField {
        Self::BaseField::zero()
    }
}

/// G1_GENERATOR_X =
/// 81937999373150964239938255573465948239988671502647976594219695644855304257327692006745978603320413799295628339695
pub const G1_GENERATOR_X: Fq = field!(
    Fq,
    BigInteger384([
        0x260f33b9772451f4,
        0xc54dd773169d5658,
        0x5c1551c469a510dd,
        0x761662e4425e1698,
        0xc97d78cc6f065272,
        0xa41206b361fd4d,
    ])
);

/// G1_GENERATOR_Y =
/// 241266749859715473739788878240585681733927191168601896383759122102112907357779751001206799952863815012735208165030
pub const G1_GENERATOR_Y: Fq = field!(
    Fq,
    BigInteger384([
        0x8193961fb8cb81f3,
        0x638d4c5f44adb8,
        0xfafaf3dad4daf54a,
        0xc27849e2d655cd18,
        0x2ec3ddb401d52814,
        0x7da93326303c71,
    ])
);
