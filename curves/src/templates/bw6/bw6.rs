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

use crate::templates::bw6::{
    g1::{G1Affine, G1Prepared, G1Projective},
    g2::{G2Affine, G2Prepared, G2Projective},
};
use serde::{Deserialize, Serialize};
use snarkos_models::curves::{
    fp3::Fp3Parameters,
    fp6_2over3::{Fp6, Fp6Parameters},
    Field,
    ModelParameters,
    One,
    PairingCurve,
    PairingEngine,
    PrimeField,
    SWModelParameters,
    SquareRootField,
};
use snarkos_utilities::bititerator::BitIterator;

use std::marker::PhantomData;

pub enum TwistType {
    M,
    D,
}

pub trait BW6Parameters: 'static {
    const X: <Self::Fp as PrimeField>::BigInteger;
    const X_IS_NEGATIVE: bool;
    const ATE_LOOP_COUNT_1: &'static [u64];
    const ATE_LOOP_COUNT_1_IS_NEGATIVE: bool;
    const ATE_LOOP_COUNT_2: &'static [i8];
    const ATE_LOOP_COUNT_2_IS_NEGATIVE: bool;
    const TWIST_TYPE: TwistType;
    type Fp: PrimeField + SquareRootField + Into<<Self::Fp as PrimeField>::BigInteger>;
    type Fp3Params: Fp3Parameters<Fp = Self::Fp>;
    type Fp6Params: Fp6Parameters<Fp3Params = Self::Fp3Params>;
    type G1Parameters: SWModelParameters<BaseField = Self::Fp>;
    type G2Parameters: SWModelParameters<
        BaseField = Self::Fp,
        ScalarField = <Self::G1Parameters as ModelParameters>::ScalarField,
    >;
}

#[derive(Derivative, Serialize, Deserialize)]
#[derivative(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub struct BW6<P: BW6Parameters>(PhantomData<fn() -> P>);

impl<P: BW6Parameters> BW6<P> {
    // Evaluate the line function at point p.
    fn ell(f: &mut Fp6<P::Fp6Params>, coeffs: &(P::Fp, P::Fp, P::Fp), p: &G1Affine<P>) {
        let mut c0 = coeffs.0;
        let mut c1 = coeffs.1;
        let mut c2 = coeffs.2;

        match P::TWIST_TYPE {
            TwistType::M => {
                c2 *= &p.y;
                c1 *= &p.x;
                f.mul_by_014(&c0, &c1, &c2);
            }
            TwistType::D => {
                c0 *= &p.y;
                c1 *= &p.x;
                f.mul_by_034(&c0, &c1, &c2);
            }
        }
    }

    fn exp_by_x(mut f: Fp6<P::Fp6Params>) -> Fp6<P::Fp6Params> {
        f = f.cyclotomic_exp(&P::X);
        if P::X_IS_NEGATIVE {
            f.conjugate();
        }
        f
    }

    pub fn final_exponentiation(value: &Fp6<P::Fp6Params>) -> Fp6<P::Fp6Params> {
        let value_inv = value.inverse().unwrap();
        let value_to_first_chunk = Self::final_exponentiation_first_chunk(value, &value_inv);
        Self::final_exponentiation_last_chunk(&value_to_first_chunk)
    }

    fn final_exponentiation_first_chunk(elt: &Fp6<P::Fp6Params>, elt_inv: &Fp6<P::Fp6Params>) -> Fp6<P::Fp6Params> {
        // (q^3-1)*(q+1)

        // elt_q3 = elt^(q^3)
        let mut elt_q3 = *elt;
        elt_q3.conjugate();
        // elt_q3_over_elt = elt^(q^3-1)
        let elt_q3_over_elt = elt_q3 * elt_inv;
        // alpha = elt^((q^3-1) * q)
        let mut alpha = elt_q3_over_elt;
        alpha.frobenius_map(1);
        // beta = elt^((q^3-1)*(q+1)
        alpha * &elt_q3_over_elt
    }

    fn final_exponentiation_last_chunk(f: &Fp6<P::Fp6Params>) -> Fp6<P::Fp6Params> {
        // hard_part
        // From https://eprint.iacr.org/2020/351.pdf, Alg.6
        // R0(x) := (-103*x^7 + 70*x^6 + 269*x^5 - 197*x^4 - 314*x^3 - 73*x^2 - 263*x - 220)
        // R1(x) := (103*x^9 - 276*x^8 + 77*x^7 + 492*x^6 - 445*x^5 - 65*x^4 + 452*x^3 - 181*x^2 + 34*x + 229)
        // f ^ R0(u) * (f ^ q) ^ R1(u) in a 2-NAF multi-exp fashion.

        // steps 1,2,3
        let f0 = *f;
        let mut f0p = f0;
        f0p.frobenius_map(1);
        let f1 = Self::exp_by_x(f0);
        let mut f1p = f1;
        f1p.frobenius_map(1);
        let f2 = Self::exp_by_x(f1);
        let mut f2p = f2;
        f2p.frobenius_map(1);
        let f3 = Self::exp_by_x(f2);
        let mut f3p = f3;
        f3p.frobenius_map(1);
        let f4 = Self::exp_by_x(f3);
        let mut f4p = f4;
        f4p.frobenius_map(1);
        let f5 = Self::exp_by_x(f4);
        let mut f5p = f5;
        f5p.frobenius_map(1);
        let f6 = Self::exp_by_x(f5);
        let mut f6p = f6;
        f6p.frobenius_map(1);
        let f7 = Self::exp_by_x(f6);
        let mut f7p = f7;
        f7p.frobenius_map(1);

        // step 4
        let f8p = Self::exp_by_x(f7p);
        let f9p = Self::exp_by_x(f8p);

        // step 5
        let mut f5p_p3 = f5p;
        f5p_p3.conjugate();
        let result1 = f3p * &f6p * &f5p_p3;

        // step 6
        let result2 = result1.square();
        let f4_2p = f4 * &f2p;
        let mut tmp1_p3 = f0 * &f1 * &f3 * &f4_2p * &f8p;
        tmp1_p3.conjugate();
        let result3 = result2 * &f5 * &f0p * &tmp1_p3;

        // step 7
        let result4 = result3.square();
        let mut f7_p3 = f7;
        f7_p3.conjugate();
        let result5 = result4 * &f9p * &f7_p3;

        // step 8
        let result6 = result5.square();
        let f2_4p = f2 * &f4p;
        let f4_2p_5p = f4_2p * &f5p;
        let mut tmp2_p3 = f2_4p * &f3 * &f3p;
        tmp2_p3.conjugate();
        let result7 = result6 * &f4_2p_5p * &f6 * &f7p * &tmp2_p3;

        // step 9
        let result8 = result7.square();
        let mut tmp3_p3 = f0p * &f9p;
        tmp3_p3.conjugate();
        let result9 = result8 * &f0 * &f7 * &f1p * &tmp3_p3;

        // step 10
        let result10 = result9.square();
        let f6p_8p = f6p * &f8p;
        let f5_7p = f5 * &f7p;
        let mut tmp4_p3 = f6p_8p;
        tmp4_p3.conjugate();
        let result11 = result10 * &f5_7p * &f2p * &tmp4_p3;

        // step 11
        let result12 = result11.square();
        let f3_6 = f3 * &f6;
        let f1_7 = f1 * &f7;
        let mut tmp5_p3 = f1_7 * &f2;
        tmp5_p3.conjugate();
        let result13 = result12 * &f3_6 * &f9p * &tmp5_p3;

        // step 12
        let result14 = result13.square();
        let mut tmp6_p3 = f4_2p * &f5_7p * &f6p_8p;
        tmp6_p3.conjugate();
        let result15 = result14 * &f0 * &f0p * &f3p * &f5p * &tmp6_p3;

        // step 13
        let result16 = result15.square();
        let mut tmp7_p3 = f3_6;
        tmp7_p3.conjugate();
        let result17 = result16 * &f1p * &tmp7_p3;

        // step 14
        let result18 = result17.square();
        let mut tmp8_p3 = f2_4p * &f4_2p_5p * &f9p;
        tmp8_p3.conjugate();
        let result19 = result18 * &f1_7 * &f5_7p * &f0p * &tmp8_p3;

        result19
    }
}

impl<P: BW6Parameters> PairingEngine for BW6<P>
where
    G1Affine<P>: PairingCurve<
        BaseField = <P::G1Parameters as ModelParameters>::BaseField,
        ScalarField = <P::G1Parameters as ModelParameters>::ScalarField,
        Projective = G1Projective<P>,
        PairWith = G2Affine<P>,
        Prepared = G1Prepared<P>,
        PairingResult = Fp6<P::Fp6Params>,
    >,
    G2Affine<P>: PairingCurve<
        BaseField = <P::G2Parameters as ModelParameters>::BaseField,
        ScalarField = <P::G1Parameters as ModelParameters>::ScalarField,
        Projective = G2Projective<P>,
        PairWith = G1Affine<P>,
        Prepared = G2Prepared<P>,
        PairingResult = Fp6<P::Fp6Params>,
    >,
{
    type Fq = P::Fp;
    type Fqe = P::Fp;
    type Fqk = Fp6<P::Fp6Params>;
    type Fr = <P::G1Parameters as ModelParameters>::ScalarField;
    type G1Affine = G1Affine<P>;
    type G1Projective = G1Projective<P>;
    type G2Affine = G2Affine<P>;
    type G2Projective = G2Projective<P>;

    fn miller_loop<'a, I>(i: I) -> Self::Fqk
    where
        I: IntoIterator<
            Item = &'a (
                &'a <Self::G1Affine as PairingCurve>::Prepared,
                &'a <Self::G2Affine as PairingCurve>::Prepared,
            ),
        >,
    {
        // Alg.5 in https://eprint.iacr.org/2020/351.pdf

        let mut pairs_1 = vec![];
        let mut pairs_2 = vec![];
        for (p, q) in i {
            if !p.is_zero() && !q.is_zero() {
                pairs_1.push((p, q.ell_coeffs_1.iter()));
                pairs_2.push((p, q.ell_coeffs_2.iter()));
            }
        }

        // f_{u+1,Q}(P)
        let mut f_1 = Self::Fqk::one();

        for i in BitIterator::new(P::ATE_LOOP_COUNT_1).skip(1) {
            f_1.square_in_place();

            for (p, ref mut coeffs) in &mut pairs_1 {
                Self::ell(&mut f_1, coeffs.next().unwrap(), &p.0);
            }
            if i {
                for &mut (p, ref mut coeffs) in &mut pairs_1 {
                    Self::ell(&mut f_1, coeffs.next().unwrap(), &p.0);
                }
            }
        }

        if P::ATE_LOOP_COUNT_1_IS_NEGATIVE {
            f_1.conjugate();
        }

        // f_{u^2-u^2-u,Q}(P)
        let mut f_2 = Self::Fqk::one();

        for i in (1..P::ATE_LOOP_COUNT_2.len()).rev() {
            if i != P::ATE_LOOP_COUNT_2.len() - 1 {
                f_2.square_in_place();
            }

            for (p, ref mut coeffs) in &mut pairs_2 {
                Self::ell(&mut f_2, coeffs.next().unwrap(), &p.0);
            }

            let bit = P::ATE_LOOP_COUNT_2[i - 1];
            match bit {
                1 => {
                    for &mut (p, ref mut coeffs) in &mut pairs_2 {
                        Self::ell(&mut f_2, coeffs.next().unwrap(), &p.0);
                    }
                }
                -1 => {
                    for &mut (p, ref mut coeffs) in &mut pairs_2 {
                        Self::ell(&mut f_2, coeffs.next().unwrap(), &p.0);
                    }
                }
                _ => continue,
            }
        }

        if P::ATE_LOOP_COUNT_2_IS_NEGATIVE {
            f_2.conjugate();
        }

        f_2.frobenius_map(1);

        f_1 * &f_2
    }

    fn final_exponentiation(f: &Self::Fqk) -> Option<Self::Fqk> {
        Some(Self::final_exponentiation(f))
    }
}
