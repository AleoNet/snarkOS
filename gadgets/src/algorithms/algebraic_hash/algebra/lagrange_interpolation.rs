use crate::algorithms::algebraic_hash::algebra::vanishing_polynomial::VanishingPolynomialGadget;
use snarkos_algorithms::algebraic_hash::algebra::lagrange_interpolation::LagrangeInterpolator;
use snarkos_models::{
    curves::PrimeField,
    gadgets::{
        curves::{FieldGadget, FpGadget},
        r1cs::ConstraintSystem,
        utilities::alloc::AllocGadget,
    },
};

pub struct LagrangeInterpolationGadget<F: PrimeField> {
    pub lagrange_interpolator: LagrangeInterpolator<F>,
    // A hack for optimizations outside of the lagrange interpolation
    pub vp_t: Option<FpGadget<F>>,
    poly_evaluations: Vec<FpGadget<F>>,
}

impl<F: PrimeField> LagrangeInterpolationGadget<F> {
    pub fn new(domain_offset: F, domain_generator: F, domain_dim: u64, poly_evaluations: Vec<FpGadget<F>>) -> Self {
        let mut poly_evaluations_F: Vec<F> = Vec::new();
        for i in 0..(1 << domain_dim) {
            poly_evaluations_F.push(poly_evaluations[i].get_value().unwrap());
        }

        let lagrange_interpolator: LagrangeInterpolator<F> =
            LagrangeInterpolator::new(domain_offset, domain_generator, domain_dim, poly_evaluations_F);

        let lagrange_interpolation_gadget = LagrangeInterpolationGadget {
            lagrange_interpolator,
            vp_t: None,
            poly_evaluations,
        };
        lagrange_interpolation_gadget
    }

    fn compute_lagrange_coefficients_constraints<CS: ConstraintSystem<F>>(
        &mut self,
        mut cs: CS,
        interpolation_point: &FpGadget<F>,
    ) -> Vec<FpGadget<F>> {
        let t = interpolation_point;
        let lagrange_coeffs = self
            .lagrange_interpolator
            .compute_lagrange_coefficients(t.get_value().unwrap());
        let mut lagrange_coeffs_FG: Vec<FpGadget<F>> = Vec::new();
        // Now we convert these lagrange coefficients to gadgets, and then constrain them.
        // The i-th lagrange coefficients constraint is:
        // (v_inv[i] * t - v_inv[i] * domain_elem[i]) * (coeff) = 1/Z_I(t)
        //
        let domain_vp_gadget = VanishingPolynomialGadget::<F>::new(self.lagrange_interpolator.domain_vp.clone());
        let vp_t = domain_vp_gadget.evaluate_constraints(&mut cs, t);
        let inv_vp_t = vp_t.inverse(cs.ns(|| "Take inverse of Z_I(t)")).unwrap();
        self.vp_t = Some(vp_t);
        for i in 0..(self.lagrange_interpolator.domain_order) {
            let constant =
                (-self.lagrange_interpolator.all_domain_elems[i]) * &self.lagrange_interpolator.v_inv_elems[i];
            let mut A_element = t
                .mul_by_constant(&mut cs, &self.lagrange_interpolator.v_inv_elems[i])
                .unwrap();
            A_element.add_constant_in_place(&mut cs, &constant).unwrap();

            let lag_coeff =
                FpGadget::<F>::alloc(&mut cs.ns(|| format!("generate lagrange coefficient {:?}", i)), || {
                    Ok(lagrange_coeffs[i])
                })
                .unwrap();
            lagrange_coeffs_FG.push(lag_coeff);
            // Enforce the actual constraint (A_element) * (lagrange_coeff) = 1/Z_I(t)
            A_element
                .mul_equals(
                    cs.ns(|| format!("Check the {:?}th lagrange coefficient", i)),
                    &lagrange_coeffs_FG[i],
                    &inv_vp_t,
                )
                .unwrap();
        }
        return lagrange_coeffs_FG;
    }

    pub fn interpolate_constraints<CS: ConstraintSystem<F>>(
        &mut self,
        mut cs: CS,
        interpolation_point: &FpGadget<F>,
    ) -> FpGadget<F> {
        let lagrange_coeffs = self.compute_lagrange_coefficients_constraints(&mut cs, interpolation_point);
        let mut interpolation = FpGadget::<F>::from(&mut cs, &F::zero());
        // Set interpolation to be: sum_{i in domain} lagrange_coeff(i)*f(i)
        for i in 0..self.lagrange_interpolator.domain_order {
            let intermediate = lagrange_coeffs[i]
                .mul(
                    cs.ns(|| {
                        format!(
                            "Compute the product of {:?}th lagrange coefficient and polynomial interpolation",
                            i
                        )
                    }),
                    &self.poly_evaluations[i],
                )
                .unwrap();
            interpolation = interpolation.add(&mut cs, &intermediate).unwrap();
        }
        interpolation
    }
}
