use crate::sw6::Fq;
use snarkos_models::{
    curves::{Field, Fp3, Fp3Parameters},
    field,
};
use snarkvm_utilities::biginteger::BigInteger832 as BigInteger;

pub type Fq3 = Fp3<Fq3Parameters>;

pub struct Fq3Parameters;

impl Fp3Parameters for Fq3Parameters {
    type Fp = Fq;

    const FROBENIUS_COEFF_FP3_C1: [Fq; 3] = [
        field!(
            Fq,
            BigInteger([
                0x9b4e60b420910c71,
                0xe068d7c83f284a6e,
                0x1f708acc7c452c43,
                0xeb2f6a66cca51856,
                0x9acf675f886e9fcd,
                0xb26885e567cc8082,
                0x75d05357183eb61f,
                0x24db4a09b5842a32,
                0x85e64cf9ba4b14ae,
                0xf311a6784358a588,
                0xe8d431c061aecb4a,
                0xd92c8b4aab19f288,
                0x21d3,
            ])
        ),
        field!(
            Fq,
            BigInteger([
                0xe793e750fc0c0fdc,
                0x28cd75f5634a867e,
                0xde5e9b1261eb3c33,
                0x68a0fb1c17595903,
                0x19626d2c9f392e46,
                0xc4d95794cb378b83,
                0x54870f1f582d67c9,
                0xf3f1a0ac4aceb56d,
                0x811361215ea4fd47,
                0x32cd6ee17d95bd00,
                0x725f9881049a9c52,
                0x5acb70be0613a307,
                0x11bb,
            ])
        ),
        field!(
            Fq,
            BigInteger([
                0x57ec31b05ef70e9c,
                0x4b273803cb8a715d,
                0xf0443627811cbe40,
                0x485f10c72ec590f1,
                0x66a35e7875569c25,
                0xdb621dfd9498071a,
                0xe0de3451f11039a8,
                0x6a3f87d780a6f7eb,
                0x637875d359122b11,
                0x967e0211b37c8d9d,
                0x8e255dfc2908fec6,
                0x90da2a32facafe8f,
                0x4b9,
            ])
        ),
    ];
    const FROBENIUS_COEFF_FP3_C2: [Fq; 3] = [
        field!(
            Fq,
            BigInteger([
                0x9b4e60b420910c71,
                0xe068d7c83f284a6e,
                0x1f708acc7c452c43,
                0xeb2f6a66cca51856,
                0x9acf675f886e9fcd,
                0xb26885e567cc8082,
                0x75d05357183eb61f,
                0x24db4a09b5842a32,
                0x85e64cf9ba4b14ae,
                0xf311a6784358a588,
                0xe8d431c061aecb4a,
                0xd92c8b4aab19f288,
                0x21d3,
            ])
        ),
        field!(
            Fq,
            BigInteger([
                0x57ec31b05ef70e9c,
                0x4b273803cb8a715d,
                0xf0443627811cbe40,
                0x485f10c72ec590f1,
                0x66a35e7875569c25,
                0xdb621dfd9498071a,
                0xe0de3451f11039a8,
                0x6a3f87d780a6f7eb,
                0x637875d359122b11,
                0x967e0211b37c8d9d,
                0x8e255dfc2908fec6,
                0x90da2a32facafe8f,
                0x4b9,
            ])
        ),
        field!(
            Fq,
            BigInteger([
                0xe793e750fc0c0fdc,
                0x28cd75f5634a867e,
                0xde5e9b1261eb3c33,
                0x68a0fb1c17595903,
                0x19626d2c9f392e46,
                0xc4d95794cb378b83,
                0x54870f1f582d67c9,
                0xf3f1a0ac4aceb56d,
                0x811361215ea4fd47,
                0x32cd6ee17d95bd00,
                0x725f9881049a9c52,
                0x5acb70be0613a307,
                0x11bb,
            ])
        ),
    ];
    /// NONRESIDUE = 13
    const NONRESIDUE: Fq = field!(
        Fq,
        BigInteger([
            0xe755952f4650755e,
            0x16c44ce1331ef791,
            0x162f8835b467306f,
            0xac1c2b31e1062c4c,
            0x20b3dab9a2a935e1,
            0xccd2ec5fd01e00c1,
            0x4d1d1bf190c8da9b,
            0x49cba09fb0e13fbe,
            0xe392ed2957c061a3,
            0x3159d02b3c93d6e1,
            0x71566d160a9f8614,
            0xa5840728fc854414,
            0x2dc4,
        ])
    );
    const QUADRATIC_NONRESIDUE_TO_T: (Fq, Fq, Fq) = (
        field!(
            Fq,
            BigInteger([
                0x59987c0ef8e31739,
                0x59578d750d6f57dd,
                0x9672547570dddab8,
                0x1a1f630e1d6dbdd5,
                0xde15f46e52d7613e,
                0x6a1b6e4f80179926,
                0x461ad119d93123b,
                0x12054e3654907ed9,
                0x85ea06b12bf811a0,
                0xc01d53d07347f9ec,
                0x70c424eb666c3922,
                0x1796ce4ed605d49e,
                0x68b,
            ])
        ),
        field!(Fq, BigInteger([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0])),
        field!(Fq, BigInteger([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0])),
    );
    const TWO_ADICITY: u32 = 3;
    const T_MINUS_ONE_DIV_TWO: &'static [u64] = &[
        0x62730e2cd2029617,
        0x660647f735cb88cf,
        0x274359d60784f69d,
        0x83067194eb102629,
        0x54ea4a12a9381160,
        0xade0b24e398dac25,
        0xb476ae9f927e81cb,
        0x220fd4a9178adc3b,
        0x57e0cb9b0569745b,
        0xba15024addc8f52e,
        0x145b9bc116144ab6,
        0x6bc2260726e88b15,
        0x51da6bf151066474,
        0x9fd1b3190f6320cf,
        0x2097bfb7bf4167b0,
        0x27c35b1e7e628e09,
        0x94f80c9d623dd9bb,
        0x20bfa6d5bf31e7d3,
        0x19fb862c049d3a8,
        0xdf4c5efe04c0cec1,
        0x32c9a8abe9b50297,
        0x268d5c2076b44f0a,
        0x76027ec67b23ca21,
        0x248d61e0c45d270,
        0x419cd0d1d6be027e,
        0xbcd8dc3b1986ef18,
        0x73093d8719c862c2,
        0x651d60f8f9f6fcd9,
        0x8dabebe38a09b261,
        0xfa85b5a9e180cd3f,
        0x6a97fc618f319fb7,
        0xce08b93a5652a8e1,
        0x37525cbc4ba24cf9,
        0xb104c580df9d2150,
        0x1407c1bfe240a89d,
        0x34c96a73372daf9a,
        0x2b87fda171,
    ];

    #[inline(always)]
    fn mul_fp_by_nonresidue(fe: &Self::Fp) -> Self::Fp {
        let original = *fe;
        let mut four_fe = fe.double();
        four_fe.double_in_place();
        let eight_fe = four_fe.double();
        eight_fe + &four_fe + &original
    }
}
