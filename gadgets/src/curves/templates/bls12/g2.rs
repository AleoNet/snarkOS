use crate::curves::templates::bls12::AffineGadget;
use snarkos_curves::templates::bls12::{Bls12Parameters, TwistType};
use snarkos_models::{
    curves::Field,
    gadgets::{
        curves::{FieldGadget, Fp2Gadget, GroupGadget},
        r1cs::{ConstraintSystem, SynthesisError},
        utilities::{eq::NEqGadget, uint8::UInt8, ToBytesGadget},
    },
};
use snarkos_utilities::bititerator::BitIterator;

use std::fmt::Debug;

pub type G2Gadget<P> = AffineGadget<<P as Bls12Parameters>::G2Parameters, <P as Bls12Parameters>::Fp, Fp2G<P>>;

type Fp2G<P> = Fp2Gadget<<P as Bls12Parameters>::Fp2Params, <P as Bls12Parameters>::Fp>;
type LCoeff<P> = (Fp2G<P>, Fp2G<P>);
#[derive(Derivative)]
#[derivative(
    Clone(bound = "Fp2Gadget<P::Fp2Params, P::Fp>: Clone"),
    Debug(bound = "Fp2Gadget<P::Fp2Params, P::Fp>: Debug")
)]
pub struct G2PreparedGadget<P: Bls12Parameters> {
    pub ell_coeffs: Vec<LCoeff<P>>,
}

impl<P: Bls12Parameters> ToBytesGadget<P::Fp> for G2PreparedGadget<P> {
    #[inline]
    fn to_bytes<CS: ConstraintSystem<P::Fp>>(&self, mut cs: CS) -> Result<Vec<UInt8>, SynthesisError> {
        let mut bytes = Vec::new();
        for (i, coeffs) in self.ell_coeffs.iter().enumerate() {
            let mut cs = cs.ns(|| format!("Iteration {}", i));
            bytes.extend_from_slice(&coeffs.0.to_bytes(&mut cs.ns(|| "c0"))?);
            bytes.extend_from_slice(&coeffs.1.to_bytes(&mut cs.ns(|| "c1"))?);
        }
        Ok(bytes)
    }

    fn to_bytes_strict<CS: ConstraintSystem<P::Fp>>(&self, cs: CS) -> Result<Vec<UInt8>, SynthesisError> {
        self.to_bytes(cs)
    }
}

impl<P: Bls12Parameters> G2PreparedGadget<P> {
    pub fn from_affine<CS: ConstraintSystem<P::Fp>>(mut cs: CS, q: &G2Gadget<P>) -> Result<Self, SynthesisError> {
        let two_inv = P::Fp::one().double().inverse().unwrap();
        let zero = G2Gadget::<P>::zero(cs.ns(|| "zero"))?;
        q.enforce_not_equal(cs.ns(|| "enforce not zero"), &zero)?;
        let mut ell_coeffs = vec![];
        let mut r = q.clone();

        for (j, i) in BitIterator::new(P::X).skip(1).enumerate() {
            let mut cs = cs.ns(|| format!("Iteration {}", j));
            ell_coeffs.push(Self::double(cs.ns(|| "double"), &mut r, &two_inv)?);

            if i {
                ell_coeffs.push(Self::add(cs.ns(|| "add"), &mut r, &q)?);
            }
        }

        Ok(Self { ell_coeffs })
    }

    fn double<CS: ConstraintSystem<P::Fp>>(
        mut cs: CS,
        r: &mut G2Gadget<P>,
        two_inv: &P::Fp,
    ) -> Result<LCoeff<P>, SynthesisError> {
        let a = r.y.inverse(cs.ns(|| "Inverse"))?;
        let mut b = r.x.square(cs.ns(|| "square x"))?;
        let b_tmp = b.clone();
        b.mul_by_fp_constant_in_place(cs.ns(|| "mul by two_inv"), two_inv)?;
        b.add_in_place(cs.ns(|| "compute b"), &b_tmp)?;

        let c = a.mul(cs.ns(|| "compute c"), &b)?;
        let d = r.x.double(cs.ns(|| "compute d"))?;
        let x3 = c.square(cs.ns(|| "c^2"))?.sub(cs.ns(|| "sub d"), &d)?;
        let e = c.mul(cs.ns(|| "c*r.x"), &r.x)?.sub(cs.ns(|| "sub r.y"), &r.y)?;
        let c_x3 = c.mul(cs.ns(|| "c*x_3"), &x3)?;
        let y3 = e.sub(cs.ns(|| "e = c * x3"), &c_x3)?;
        let mut f = c;
        f.negate_in_place(cs.ns(|| "c = -c"))?;
        r.x = x3;
        r.y = y3;
        match P::TWIST_TYPE {
            TwistType::M => Ok((e, f)),
            TwistType::D => Ok((f, e)),
        }
    }

    fn add<CS: ConstraintSystem<P::Fp>>(
        mut cs: CS,
        r: &mut G2Gadget<P>,
        q: &G2Gadget<P>,
    ) -> Result<LCoeff<P>, SynthesisError> {
        let a = q.x.sub(cs.ns(|| "q.x - r.x"), &r.x)?.inverse(cs.ns(|| "calc a"))?;
        let b = q.y.sub(cs.ns(|| "q.y - r.y"), &r.y)?;
        let c = a.mul(cs.ns(|| "compute c"), &b)?;
        let d = r.x.add(cs.ns(|| "r.x + q.x"), &q.x)?;
        let x3 = c.square(cs.ns(|| "c^2"))?.sub(cs.ns(|| "sub d"), &d)?;

        let e =
            r.x.sub(cs.ns(|| "r.x - x3"), &x3)?
                .mul(cs.ns(|| "c * (r.x - x3)"), &c)?;
        let y3 = e.sub(cs.ns(|| "calc y3"), &r.y)?;
        let g = c.mul(cs.ns(|| "c*r.x"), &r.x)?.sub(cs.ns(|| "calc g"), &r.y)?;
        let mut f = c;
        f.negate_in_place(cs.ns(|| "c = -c"))?;
        r.x = x3;
        r.y = y3;
        match P::TWIST_TYPE {
            TwistType::M => Ok((g, f)),
            TwistType::D => Ok((f, g)),
        }
    }
}
