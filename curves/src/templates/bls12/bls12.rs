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

use crate::templates::bls12::{
    g1::{G1Affine, G1Prepared, G1Projective},
    g2::{G2Affine, G2Prepared, G2Projective},
};
use serde::{Deserialize, Serialize};
use snarkos_models::curves::{
    fp12_2over3over2::{Fp12, Fp12Parameters},
    fp2::Fp2Parameters,
    fp6_3over2::Fp6Parameters,
    Field,
    Fp2,
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

pub trait Bls12Parameters: 'static {
    const X: &'static [u64];
    const X_IS_NEGATIVE: bool;
    const TWIST_TYPE: TwistType;
    type Fp: PrimeField + SquareRootField + Into<<Self::Fp as PrimeField>::BigInteger>;
    type Fp2Params: Fp2Parameters<Fp = Self::Fp>;
    type Fp6Params: Fp6Parameters<Fp2Params = Self::Fp2Params>;
    type Fp12Params: Fp12Parameters<Fp6Params = Self::Fp6Params>;
    type G1Parameters: SWModelParameters<BaseField = Self::Fp>;
    type G2Parameters: SWModelParameters<
        BaseField = Fp2<Self::Fp2Params>,
        ScalarField = <Self::G1Parameters as ModelParameters>::ScalarField,
    >;
}

#[derive(Derivative, Serialize, Deserialize)]
#[derivative(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub struct Bls12<P: Bls12Parameters>(PhantomData<fn() -> P>);

type CoeffTriplet<T> = (Fp2<T>, Fp2<T>, Fp2<T>);

impl<P: Bls12Parameters> Bls12<P> {
    // Evaluate the line function at point p.
    fn ell(f: &mut Fp12<P::Fp12Params>, coeffs: &CoeffTriplet<P::Fp2Params>, p: &G1Affine<P>) {
        let mut c0 = coeffs.0;
        let mut c1 = coeffs.1;
        let mut c2 = coeffs.2;

        match P::TWIST_TYPE {
            TwistType::M => {
                c2.mul_by_fp(&p.y);
                c1.mul_by_fp(&p.x);
                f.mul_by_014(&c0, &c1, &c2);
            }
            TwistType::D => {
                c0.mul_by_fp(&p.y);
                c1.mul_by_fp(&p.x);
                f.mul_by_034(&c0, &c1, &c2);
            }
        }
    }

    fn exp_by_x(mut f: Fp12<P::Fp12Params>) -> Fp12<P::Fp12Params> {
        f = f.cyclotomic_exp(P::X);
        if P::X_IS_NEGATIVE {
            f.conjugate();
        }
        f
    }
}

impl<P: Bls12Parameters> PairingEngine for Bls12<P>
where
    G1Affine<P>: PairingCurve<
        BaseField = <P::G1Parameters as ModelParameters>::BaseField,
        ScalarField = <P::G1Parameters as ModelParameters>::ScalarField,
        Projective = G1Projective<P>,
        PairWith = G2Affine<P>,
        Prepared = G1Prepared<P>,
        PairingResult = Fp12<P::Fp12Params>,
    >,
    G2Affine<P>: PairingCurve<
        BaseField = <P::G2Parameters as ModelParameters>::BaseField,
        ScalarField = <P::G1Parameters as ModelParameters>::ScalarField,
        Projective = G2Projective<P>,
        PairWith = G1Affine<P>,
        Prepared = G2Prepared<P>,
        PairingResult = Fp12<P::Fp12Params>,
    >,
{
    type Fq = P::Fp;
    type Fqe = Fp2<P::Fp2Params>;
    type Fqk = Fp12<P::Fp12Params>;
    type Fr = <P::G1Parameters as ModelParameters>::ScalarField;
    type G1Affine = G1Affine<P>;
    type G1Projective = G1Projective<P>;
    type G2Affine = G2Affine<P>;
    type G2Projective = G2Projective<P>;

    fn miller_loop<'a, I>(i: I) -> Self::Fqk
    where
        I: Iterator<
            Item = (
                &'a <Self::G1Affine as PairingCurve>::Prepared,
                &'a <Self::G2Affine as PairingCurve>::Prepared,
            ),
        >,
    {
        let mut pairs = vec![];
        for (p, q) in i {
            if !p.is_zero() && !q.is_zero() {
                pairs.push((p, q.ell_coeffs.iter()));
            }
        }

        let mut f = Self::Fqk::one();

        for i in BitIterator::new(P::X).skip(1) {
            f.square_in_place();

            for &mut (p, ref mut coeffs) in &mut pairs {
                Self::ell(&mut f, coeffs.next().unwrap(), &p.0);
            }

            if i {
                for &mut (p, ref mut coeffs) in &mut pairs {
                    Self::ell(&mut f, coeffs.next().unwrap(), &p.0);
                }
            }
        }

        if P::X_IS_NEGATIVE {
            f.conjugate();
        }

        f
    }

    fn final_exponentiation(f: &Self::Fqk) -> Option<Self::Fqk> {
        // Computing the final exponentation following
        // https://eprint.iacr.org/2016/130.pdf.
        // We don't use their "faster" formula because it is difficult to make
        // it work for curves with odd `P::X`.
        // Hence we implement the algorithm from Table 1 below.

        // f1 = r.conjugate() = f^(p^6)
        let mut f1 = *f;
        f1.conjugate();

        match f.inverse() {
            Some(mut f2) => {
                // f2 = f^(-1);
                // r = f^(p^6 - 1)
                let mut r = f1 * &f2;

                // f2 = f^(p^6 - 1)
                f2 = r;
                // r = f^((p^6 - 1)(p^2))
                r.frobenius_map(2);

                // r = f^((p^6 - 1)(p^2) + (p^6 - 1))
                // r = f^((p^6 - 1)(p^2 + 1))
                r *= &f2;

                // Hard part of the final exponentation is below:
                // From https://eprint.iacr.org/2016/130.pdf, Table 1
                let mut y0 = r.cyclotomic_square();
                y0.conjugate();

                let mut y5 = Self::exp_by_x(r);

                let mut y1 = y5.cyclotomic_square();
                let mut y3 = y0 * &y5;
                y0 = Self::exp_by_x(y3);
                let y2 = Self::exp_by_x(y0);
                let mut y4 = Self::exp_by_x(y2);
                y4 *= &y1;
                y1 = Self::exp_by_x(y4);
                y3.conjugate();
                y1 *= &y3;
                y1 *= &r;
                y3 = r;
                y3.conjugate();
                y0 *= &r;
                y0.frobenius_map(3);
                y4 *= &y3;
                y4.frobenius_map(1);
                y5 *= &y2;
                y5.frobenius_map(2);
                y5 *= &y0;
                y5 *= &y4;
                y5 *= &y1;
                Some(y5)
            }
            None => None,
        }
    }
}
