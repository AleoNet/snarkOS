// Copyright (C) 2019-2020 Aleo Systems Inc.
// This file is part of the snarkOS library.

// The snarkOS library is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// The snarkOS library is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with the snarkOS library. If not, see <https://www.gnu.org/licenses/>.

use crate::{
    sw6::{Fq, Fq3, Fq6, Fr, G1Affine, FQ_ZERO, SW6},
    templates::short_weierstrass::short_weierstrass_jacobian::{GroupAffine, GroupProjective},
};
use snarkos_models::{
    curves::{ModelParameters, PairingCurve, PairingEngine, SWModelParameters},
    field,
};
use snarkos_utilities::biginteger::{BigInteger384, BigInteger832};

pub type G2Affine = GroupAffine<SW6G2Parameters>;
pub type G2Projective = GroupProjective<SW6G2Parameters>;

impl PairingCurve for G2Affine {
    type Engine = SW6;
    type PairWith = G1Affine;
    type PairingResult = Fq6;
    type Prepared = Self;

    fn prepare(&self) -> Self::Prepared {
        self.clone()
    }

    fn pairing_with(&self, other: &Self::PairWith) -> Self::PairingResult {
        SW6::pairing(*other, *self)
    }
}

#[derive(Copy, Clone, Default, PartialEq, Eq)]
pub struct SW6G2Parameters;

impl ModelParameters for SW6G2Parameters {
    type BaseField = Fq3;
    type ScalarField = Fr;
}

impl SWModelParameters for SW6G2Parameters {
    /// AFFINE_GENERATOR_COEFFS = (G2_GENERATOR_X, G2_GENERATOR_Y)
    const AFFINE_GENERATOR_COEFFS: (Self::BaseField, Self::BaseField) = (G2_GENERATOR_X, G2_GENERATOR_Y);
    /// COEFF_A = (0, 0, COEFF_A * TWIST^2) = (0, 0, 5)
    const COEFF_A: Fq3 = field!(
        Fq3,
        FQ_ZERO,
        FQ_ZERO,
        field!(
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
        ),
    );
    /// COEFF_B = (G1::COEFF_B * TWIST^3, 0, 0) =
    /// (7237353553714858194254855835825640240663090882935418626687402315497764195116318527743248304684159666286416318482685337633828994152723793439622384740540789612754127688659139509552568164770448654259255628317166934203899992395064470477612,
    /// 0, 0)
    const COEFF_B: Fq3 = field!(
        Fq3,
        field!(
            Fq,
            BigInteger832([
                0xc00a9afc5cbce615,
                0x0260c2b730644102,
                0x9051e955661691ec,
                0x15f9af8514839e37,
                0xfa62826ca407172b,
                0x37043dc868f48874,
                0x876b5588d132b025,
                0x481952128335562a,
                0x4ffa729aeddd7dcd,
                0xe181a5dae94a399f,
                0x671fb50145b255d8,
                0xbc3860730482d728,
                0x00000000000023dd,
            ])
        ),
        FQ_ZERO,
        FQ_ZERO,
    );
    /// COFACTOR =
    /// 43276679045916726782882096851503554444292580777869919574700824986947162516693702667493938255647666346010819253090121562084993205202476199057555142869892665220155573207800985012241638987472334344174208389303164492698303448192856551557283997344470334833850065978668184377503856699635686872344035470027430053642178229054516302338812152178131995800255516474185251732445975837621097393375441662426280154371264547168198834382681059556891327702516519955053315674076980350109237328216856859758931256208439575383786363605925879337208599843910819433766160937121108797819223653884174994325142959644019600
    const COFACTOR: &'static [u64] = &[
        0x4b77fca151d50b90,
        0x8c98a12bd486d2fb,
        0x1f0c9a51593693f8,
        0x1d6f388069c063c1,
        0x556e918748f06793,
        0x2cea7dc01aae2140,
        0x4216f0595cee44d0,
        0x7a5e400154f633cf,
        0xbb74eb9b6630846b,
        0x8eb48c92998f3358,
        0xbedd37f629e8e634,
        0xc541018fe4d10cc7,
        0x574956a099ace2c3,
        0xa597504275948226,
        0x7ecaaf050acb91f3,
        0x0f25b044f4e9c932,
        0xf8c39cbf0df97780,
        0xd8f9eda95d6abf3e,
        0xd1d80da227dd39c1,
        0x8b589c61531dbce7,
        0xfee4439281455474,
        0x9eea59baa2aeb4a1,
        0xa3b8a42c4e1e6f5a,
        0xc4b99b0d9b077d21,
        0xd09033887d09b4d2,
        0x4a86d8ebb7fdf52a,
        0xbe7ce44dd084e05d,
        0x4ed25f7ebe6c44b3,
        0xd7f8e3ef00255961,
        0xa1ad2ad61580ef78,
        0x19e70d3618ca3,
    ];
    /// COFACTOR^(-1) mod r =
    /// 45586359457219724873147353901735745013467692594291916855200979604570630929674383405372210802279573887880950375598
    const COFACTOR_INV: Fr = field!(
        Fr,
        BigInteger384([
            7373687189387546408,
            11284009518041539892,
            301575489693670883,
            13203058298476577559,
            18441611830097862156,
            4115759498196698,
        ])
    );
}

const G2_GENERATOR_X: Fq3 = field!(Fq3, G2_GENERATOR_X_C0, G2_GENERATOR_X_C1, G2_GENERATOR_X_C2);
const G2_GENERATOR_Y: Fq3 = field!(Fq3, G2_GENERATOR_Y_C0, G2_GENERATOR_Y_C1, G2_GENERATOR_Y_C2);

/// G2_GENERATOR_X_C0 =
/// 13426761183630949215425595811885033211332897733228446437546263564078445562454176776915160094418980045665397361295624472103734543457352048745726512354895954850428989867542989474136256025045975283415690491751906307188562464175510373683338
pub const G2_GENERATOR_X_C0: Fq = field!(
    Fq,
    BigInteger832([
        0x03b3fe4c8d4ecac7,
        0x9568212677524d1e,
        0xf5de3f2228d187c1,
        0x7bac772e31a420ef,
        0x0255cf59968a612b,
        0x991d4676f6b5d605,
        0x02dd2ae4831d29ea,
        0xbeca7c9a62e392c2,
        0xfc1d0633d48d2fc5,
        0x7867813be5f7d2a1,
        0x6f567b6617030028,
        0xf08c9fa6ca6809df,
        0x0000000000000de9,
    ])
);

/// G2_GENERATOR_X_C1 =
/// 20471601555918880743198170952645906008198510944268658573129351735028343217532386920456705632337352161031960990613816401042894531220068552819818037605513359562118363589199569321421558696125646867661360498323171027455638052943806292028610
pub const G2_GENERATOR_X_C1: Fq = field!(
    Fq,
    BigInteger832([
        0xefd1b506e5fbe05f,
        0xad27d47a4975140c,
        0xfa11540132dbc27a,
        0x8dca42b6da7c4717,
        0x66d30fd7fd76207a,
        0xb8e4f65c68932b1d,
        0x3b7f971e93ad14be,
        0xf860a89f4e582f9f,
        0x7d438aaa3986f73b,
        0xa37ec0c18c6e106a,
        0x9f2dfb98b5185b54,
        0x19995e421ca939bc,
        0x0000000000002f4f,
    ])
);

/// G2_GENERATOR_X_C2 =
/// 3905053196875761830053608605277158152930144841844497593936739534395003062685449846381431331169369910535935138116320442345524758217411779027270883193856999691582831339845600938304719916501940381093815781408183227875600753651697934495980
pub const G2_GENERATOR_X_C2: Fq = field!(
    Fq,
    BigInteger832([
        0xc081ed832bdf911e,
        0xb85ff7aeebdfe7b3,
        0x96dce6bb307b14eb,
        0x578f7ded84bd824c,
        0xb799305a9971d184,
        0x0116ad33c2874b90,
        0x862dce68efdca245,
        0x4190947c70534c1d,
        0x1b1aa80334248d03,
        0xb13b07aff63fcf27,
        0x5727687b73ab4fff,
        0xf559a7f4eb8d180a,
        0x0000000000002d37,
    ])
);

/// G2_GENERATOR_Y_C0 =
/// 8567517639523571619872938228644013584947463594196306323477160496987712111576624702939472765993995586889532559039169098780892505598589581147768095093536988446010255611523736706017580686335404469207486594272103717837888228343074699140243
pub const G2_GENERATOR_Y_C0: Fq = field!(
    Fq,
    BigInteger832([
        0x3f680b59e26b33d1,
        0x720fdf65b9e15b17,
        0x0f0b56def11247b1,
        0x5ea05417c8a4a52c,
        0x4ad59dc4f7c47a09,
        0xf393e0db62107115,
        0xde3b16404a53d2bb,
        0xeaa74961636280e0,
        0x2d16ccd14cf5a88c,
        0x5667565a06187d0e,
        0xb446fdc7565d0261,
        0xd3ad395d6fd0faab,
        0x0000000000000655,
    ])
);

/// G2_GENERATOR_Y_C1 =
/// 3890537069205870914984502594450293167889863914413852788876350245583932846980126025043974070704295857226211547108005650399870458089721518559480870503159804530091559886149680718531004778697982910253701559194337987238111062202037698927752
pub const G2_GENERATOR_Y_C1: Fq = field!(
    Fq,
    BigInteger832([
        0x9e86cc63207679dd,
        0x4e16d9a9d87c3e47,
        0xdbee3524db80627d,
        0x137322b87d93befc,
        0x24a7ca2f9aae90a0,
        0x44abea538df3e854,
        0xc01d176c6e042eee,
        0xf5fcc4caabc75699,
        0x1f99972699a38960,
        0x30d4cc8256bf963d,
        0xa3634826edcfefff,
        0x34f3bd0c8e5a4b38,
        0x0000000000001d28,
    ])
);

/// G2_GENERATOR_Y_C2 =
/// 10936269922612615564271188303104593362724754284143779051599749016735041389483971486958818324356025479751246744831831158558101688599198721653921723013062333636402617118847009085485166284126970598561393411916461254016145116183331671450721
pub const G2_GENERATOR_Y_C2: Fq = field!(
    Fq,
    BigInteger832([
        0xfc478105dedf3654,
        0xa6fcfcfdd2710d6a,
        0x05a68c283d5d4c65,
        0x9fab8d94c667a679,
        0x009b0a616ea54ff9,
        0xf0df517bc7bc6382,
        0xdb44338e7491f5b7,
        0xcd192a7e53453f45,
        0xa041a7a60982d92c,
        0x4dd01c62bae4c7ff,
        0x79a69a54e6b66178,
        0xd47b0bfe832b05f8,
        0x00000000000000ef,
    ])
);
