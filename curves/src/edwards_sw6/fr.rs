use snarkos_models::curves::{Fp384, Fp384Parameters, FpParameters};
use snarkvm_utilities::biginteger::BigInteger384 as BigInteger;

pub type Fr = Fp384<FrParameters>;

pub struct FrParameters;

impl Fp384Parameters for FrParameters {}

impl FpParameters for FrParameters {
    type BigInt = BigInteger;

    const CAPACITY: u32 = Self::MODULUS_BITS - 1;
    // 2
    const GENERATOR: BigInteger = BigInteger([
        1999556893213776791u64,
        13750542494830678672u64,
        1782306145063399878u64,
        452443773434042853u64,
        15997990832658725900u64,
        3914639203155617u64,
    ]);
    const INV: u64 = 16242011933465909059u64;
    // MODULUS = 32333053251621136751331591711861691692049189094364332567435817881934511297123972799646723302813083835942624121493
    const MODULUS: BigInteger = BigInteger([
        4684667634276979349u64,
        3748803659444032385u64,
        16273581227874629698u64,
        7152942431629910641u64,
        6397188139321141543u64,
        15137289088311837u64,
    ]);
    const MODULUS_BITS: u32 = 374;
    const MODULUS_MINUS_ONE_DIV_TWO: BigInteger = BigInteger([
        11565705853993265482u64,
        1874401829722016192u64,
        17360162650792090657u64,
        12799843252669731128u64,
        12421966106515346579u64,
        7568644544155918u64,
    ]);
    const R: BigInteger = BigInteger([
        12565484300600153878u64,
        8749673077137355528u64,
        9027943686469014788u64,
        13026065139386752555u64,
        11197589485989933721u64,
        9525964145733727u64,
    ]);
    const R2: BigInteger = BigInteger([
        17257035094703902127u64,
        16096159112880350050u64,
        3498553494623421763u64,
        333405339929360058u64,
        1125865524035793947u64,
        1586246138566285u64,
    ]);
    const REPR_SHAVE_BITS: u32 = 10;
    const ROOT_OF_UNITY: BigInteger = BigInteger([
        12119792640622387781u64,
        8318439284650634613u64,
        6931324077796168275u64,
        12851391603681523141u64,
        6881015057611215092u64,
        1893962574900431u64,
    ]);
    const T: BigInteger = BigInteger([0x0, 0x0, 0x0, 0x0, 0x0, 0x0]);
    const TWO_ADICITY: u32 = 2u32;
    const T_MINUS_ONE_DIV_TWO: BigInteger = BigInteger([0x0, 0x0, 0x0, 0x0, 0x0, 0x0]);
}
