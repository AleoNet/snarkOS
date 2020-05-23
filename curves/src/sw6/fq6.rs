use crate::sw6::{Fq, Fq3, Fq3Parameters};
use snarkos_models::{
    curves::fp6_2over3::{Fp6, Fp6Parameters},
    field,
};
use snarkvm_utilities::biginteger::BigInteger832 as BigInteger;

pub type Fq6 = Fp6<Fq6Parameters>;

pub struct Fq6Parameters;

impl Fp6Parameters for Fq6Parameters {
    type Fp3Params = Fq3Parameters;

    const FROBENIUS_COEFF_FP6_C1: [Fq; 6] = [
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
                0x82e248051c9d1c4d,
                0x9364dbda272d0ed,
                0xfdcf25dede306877,
                0x53d06582e3fe7159,
                0xb431d48c27a7ce14,
                0x7741dd7a33040c05,
                0xca576276706c1de9,
                0x18cceab60052df9f,
                0x6f9ae1b18f011f6,
                0x25df1559c0ee6289,
                0x5b33ca416649679d,
                0x33f7fc08b12d9590,
                0x338f,
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
                0x3f8019015b031e78,
                0x73f4adf92ed4f7dc,
                0xcea2d139e307fa73,
                0xb1000be3461ee9f5,
                0x8005cba5148fca6b,
                0xa03b75925fcf929d,
                0x35654371493da172,
                0x5e312883cb75ad59,
                0xe48bd6f4b7b72859,
                0xc94b70f331124a9d,
                0x84f67d2da39b18,
                0xeba59af100dea197,
                0x1674,
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
                0xf33a92647f881b0d,
                0x2b900fcc0ab2bbcb,
                0xfb4c0f3fd61ea84,
                0x338e7b2dfb6aa948,
                0x172c5d7fdc53bf3,
                0x8dcaa3e2fc64879d,
                0x56ae87a9094eefc8,
                0x8f1ad1e1362b221e,
                0xe95ec2cd135d3fbf,
                0x898fa889f6d53325,
                0x76f98fbc8ab7ca11,
                0x6a06b57da5e4f118,
                0x268d,
            ])
        ),
    ];
    /// NONRESIDUE = 13
    const NONRESIDUE: Fq3 = field!(
        Fq3,
        field!(
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
        ),
        field!(
            Fq,
            BigInteger([0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0,])
        ),
        field!(
            Fq,
            BigInteger([0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0,])
        ),
    );
}
