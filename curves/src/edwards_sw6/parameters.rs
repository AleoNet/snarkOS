use crate::{
    edwards_sw6::{Fq, Fr},
    templates::twisted_edwards_extended::{GroupAffine, GroupProjective},
};
use snarkos_models::{
    curves::{ModelParameters, MontgomeryModelParameters, TEModelParameters},
    field,
};
use snarkos_utilities::biginteger::BigInteger384 as BigInteger;

use std::str::FromStr;

pub type EdwardsAffine = GroupAffine<EdwardsParameters>;
pub type EdwardsProjective = GroupProjective<EdwardsParameters>;

#[derive(Clone, Default, PartialEq, Eq)]
pub struct EdwardsParameters;

impl ModelParameters for EdwardsParameters {
    type BaseField = Fq;
    type ScalarField = Fr;
}

impl TEModelParameters for EdwardsParameters {
    type MontgomeryModelParameters = EdwardsParameters;

    /// AFFINE_GENERATOR_COEFFS = (GENERATOR_X, GENERATOR_Y)
    const AFFINE_GENERATOR_COEFFS: (Self::BaseField, Self::BaseField) = (GENERATOR_X, GENERATOR_Y);
    /// COEFF_A = -1 =
    /// 258664426012969094010652733694893533536393512754914660539884262666720468348340822774968888139573360124440321458176
    const COEFF_A: Fq = field!(
        Fq,
        BigInteger([
            9384023879812382873,
            14252412606051516495,
            9184438906438551565,
            11444845376683159689,
            8738795276227363922,
            81297770384137296,
        ])
    );
    /// COEFF_D = 79743
    const COEFF_D: Fq = field!(
        Fq,
        BigInteger([
            0x4669ffffff46a638,
            0xa56bbe0a7f9fae05,
            0x403b425466a710b4,
            0xf6648db6ea4e988b,
            0x74d51b5923d35a8d,
            0xf8ed90b17fe903,
        ])
    );
    /// COFACTOR = 8
    const COFACTOR: &'static [u64] = &[8];
    /// COFACTOR^(-1) mod r =
    /// 12124894969357926281749346891948134384518445910386624712788431705725441736421489799867521238554906438478484045560
    const COFACTOR_INV: Fr = field!(
        Fr,
        BigInteger([
            7353538464571651976,
            2030910049503177537,
            16726103313845754033,
            1110650741117127777,
            5304838729792721053,
            4975067790294675,
        ])
    );

    /// Multiplication by `a` is just negation.
    #[inline(always)]
    fn mul_by_a(elem: &Self::BaseField) -> Self::BaseField {
        -*elem
    }
}

impl MontgomeryModelParameters for EdwardsParameters {
    type TEModelParameters = EdwardsParameters;

    /// COEFF_A = 0x95D53EB3F6AC3F7A53C26020144439DC6073BCAE513E03FD06B6B3BAA390F25E51534B26719E33F4CD906D4DA9B535
    const COEFF_A: Fq = field!(
        Fq,
        BigInteger([
            7594254284108454966u64,
            14287343397973578077u64,
            6490358977072726023u64,
            8023375322051995268u64,
            8242802613686040715u64,
            100541941146122331u64,
        ])
    );
    /// COEFF_B = 0x118650763CE64AB4BE743604C8D05013DC2663652A3D58B21ECAB7BFF65B70DB8BA09F9098E61CC903B2F92B2564ACA
    const COEFF_B: Fq = field!(
        Fq,
        BigInteger([
            11173793475516310780u64,
            14217481814129454913u64,
            11878518835804377107u64,
            14866315431314324110u64,
            9234787938768687129u64,
            62053599622152261u64,
        ])
    );
}

impl FromStr for EdwardsAffine {
    type Err = ();

    fn from_str(mut s: &str) -> Result<Self, Self::Err> {
        s = s.trim();
        if s.is_empty() {
            return Err(());
        }
        if s.len() < 3 {
            return Err(());
        }
        if !(s.starts_with('(') && s.ends_with(')')) {
            return Err(());
        }
        let mut point = Vec::new();
        for substr in s.split(|c| c == '(' || c == ')' || c == ',' || c == ' ') {
            if !substr.is_empty() {
                point.push(Fq::from_str(substr)?);
            }
        }
        if point.len() != 2 {
            return Err(());
        }
        let point = EdwardsAffine::new(point[0], point[1]);

        if !point.is_on_curve() { Err(()) } else { Ok(point) }
    }
}

/// GENERATOR_X =
/// 174701772324485506941690903512423551998294352968833659960042362742684869862495746426366187462669992073196420267127
const GENERATOR_X: Fq = field!(
    Fq,
    BigInteger([
        3737364149926089590,
        13002967008679663837,
        9954144214462864555,
        3365719140389487049,
        8643066672427471196,
        120355578793479865,
    ])
);

/// GENERATOR_Y =
/// 208487200052258845495340374451540775445408439654930191324011635560142523886549663106522691296420655144190624954833
const GENERATOR_Y: Fq = field!(
    Fq,
    BigInteger([
        6027299446526298157,
        12854429557810467099,
        11207279014226687864,
        17040621363687352702,
        6112671509202865855,
        44040319652922447,
    ])
);
