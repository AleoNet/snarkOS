use crate::algebraic_hash::algebra::vanishing_polynomial::VanishingPolynomial;
use snarkos_models::curves::{batch_inversion, PrimeField};

#[derive(Clone)]
/// Struct describing Lagrange interpolation for a multiplicative coset I,
/// with |I| a power of 2.
/// TODO: Pull in lagrange poly explanation from libiop
pub struct LagrangeInterpolator<F: PrimeField> {
    pub domain_order: usize,
    pub all_domain_elems: Vec<F>,
    pub v_inv_elems: Vec<F>,
    pub domain_vp: VanishingPolynomial<F>,
    pub poly_evaluations: Vec<F>,
}

impl<F: PrimeField> LagrangeInterpolator<F> {
    pub fn new(domain_offset: F, domain_generator: F, domain_dim: u64, poly_evaluations: Vec<F>) -> Self {
        let domain_order = 1 << domain_dim;
        assert_eq!(poly_evaluations.len(), domain_order);
        let mut cur_elem = domain_offset;
        let mut all_domain_elems = vec![domain_offset];
        let mut v_inv_elems: Vec<F> = Vec::new();
        // Cache all elements in the domain
        for _ in 1..domain_order {
            cur_elem *= &domain_generator;
            all_domain_elems.push(cur_elem);
        }
        // By computing the following elements as constants,
        // we can further reduce the interpolation costs.
        //
        // m = order of the interpolation domain
        // v_inv[i] = prod_{j != i} h(g^i - g^j)
        // We use the following facts to compute this:
        //   v_inv[0] = m*h^{m-1}
        //   v_inv[i] = g^{-1} * v_inv[i-1]
        // TODO: Include proof of the above two points
        let g_inv = domain_generator.inverse().unwrap();
        let m = F::from((1 << domain_dim) as u64);
        let mut v_inv_i = m * &domain_offset.pow([(domain_order - 1) as u64]);
        for _ in 0..domain_order {
            v_inv_elems.push(v_inv_i);
            v_inv_i *= &g_inv;
        }

        // TODO: Ideally we'd cache the intermediate terms with Z_H(x) evaluations, since most of the exponents are precomputed.
        let vp = VanishingPolynomial::new(domain_offset, domain_dim);

        let lagrange_interpolation: LagrangeInterpolator<F> = LagrangeInterpolator {
            domain_order,
            all_domain_elems,
            v_inv_elems,
            domain_vp: vp,
            poly_evaluations,
        };
        lagrange_interpolation
    }

    pub fn compute_lagrange_coefficients(&self, interpolation_point: F) -> Vec<F> {
        /*
        * Let t be the interpolation point, H be the multiplicative coset, with elements of the form h*g^i.
        Compute each L_{i,H}(t) as Z_{H}(t) * v_i / (t- h g^i)
        where:
        - Z_{H}(t) = \prod_{j} (t-h*g^j) = (t^m-h^m), and
        - v_{i} = 1 / \prod_{j \neq i} h(g^i-g^j).
        Below we use the fact that v_{0} = 1/(m * h^(m-1)) and v_{i+1} = g * v_{i}.
        We compute the inverse of each coefficient, and then batch invert the entire result.
        TODO: explain deriviation more step by step
        */
        // TODO: Implement batch_inverse & mul like libiop for better efficiency
        let vp_t_inv = self.domain_vp.evaluate(&interpolation_point).inverse().unwrap();
        let mut inverted_lagrange_coeffs: Vec<F> = Vec::with_capacity(self.all_domain_elems.len());
        for i in 0..self.domain_order {
            let l = vp_t_inv * &self.v_inv_elems[i];
            let r = self.all_domain_elems[i];
            inverted_lagrange_coeffs.push(l * &(interpolation_point - &r));
        }
        let lagrange_coeffs = inverted_lagrange_coeffs.as_mut_slice();
        batch_inversion::<F>(lagrange_coeffs);
        lagrange_coeffs.iter().cloned().collect()
    }

    pub fn interpolate(&self, interpolation_point: F) -> F {
        let lagrange_coeffs = self.compute_lagrange_coefficients(interpolation_point);
        let mut interpolation = F::zero();
        for i in 0..self.domain_order {
            interpolation += &(lagrange_coeffs[i] * &self.poly_evaluations[i]);
        }
        interpolation
    }
}
