use crate::bls12_377::Fq;
use serde::{Deserialize, Serialize};
use snarkos_models::{
    curves::{Field, Fp2, Fp2Parameters},
    field,
};
use snarkos_utilities::biginteger::BigInteger384 as BigInteger;

pub type Fq2 = Fp2<Fq2Parameters>;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fq2Parameters;

impl Fp2Parameters for Fq2Parameters {
    type Fp = Fq;

    /// Coefficients for the Frobenius automorphism.
    const FROBENIUS_COEFF_FP2_C1: [Fq; 2] = [
        // NONRESIDUE**(((q^0) - 1) / 2)
        field!(
            Fq,
            BigInteger([
                0x2cdffffffffff68,
                0x51409f837fffffb1,
                0x9f7db3a98a7d3ff2,
                0x7b4e97b76e7c6305,
                0x4cf495bf803c84e8,
                0x8d6661e2fdf49a,
            ])
        ),
        // NONRESIDUE**(((q^1) - 1) / 2)
        field!(
            Fq,
            BigInteger([
                0x823ac00000000099,
                0xc5cabdc0b000004f,
                0x7f75ae862f8c080d,
                0x9ed4423b9278b089,
                0x79467000ec64c452,
                0x120d3e434c71c50,
            ])
        ),
    ];
    /// NONRESIDUE = -5
    const NONRESIDUE: Fq = field!(
        Fq,
        BigInteger([
            0xfc0b8000000002fa,
            0x97d39cf6e000018b,
            0x2072420fbfa05044,
            0xcbbcbd50d97c3802,
            0xbaf1ec35813f9eb,
            0x9974a2c0945ad2,
        ])
    );
    /// QUADRATIC_NONRESIDUE = U
    const QUADRATIC_NONRESIDUE: (Fq, Fq) = (
        field!(Fq, BigInteger([0, 0, 0, 0, 0, 0])),
        field!(
            Fq,
            BigInteger([
                202099033278250856u64,
                5854854902718660529u64,
                11492539364873682930u64,
                8885205928937022213u64,
                5545221690922665192u64,
                39800542322357402u64,
            ])
        ),
    );

    #[inline(always)]
    fn mul_fp_by_nonresidue(fe: &Self::Fp) -> Self::Fp {
        let original = fe;
        let mut fe = -fe.double();
        fe.double_in_place();
        fe - original
    }
}
