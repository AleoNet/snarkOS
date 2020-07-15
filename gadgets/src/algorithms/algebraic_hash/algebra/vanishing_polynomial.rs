use snarkos_algorithms::algebraic_hash::algebra::vanishing_polynomial::VanishingPolynomial;
use snarkos_models::{
    curves::Field,
    gadgets::{curves::FieldGadget, r1cs::ConstraintSystem},
};

/// Struct describing vanishing polynomials for a multiplicative coset H,
/// with |H| a power of 2.
/// As H is a coset, every element can be described as h*g^i,
/// and therefore has vanishing polynomial Z_H(x) = x^|H| - h^|H|
pub struct VanishingPolynomialGadget<F: Field> {
    pub vp: VanishingPolynomial<F>,
}

impl<F: Field> VanishingPolynomialGadget<F> {
    pub fn new(vp: VanishingPolynomial<F>) -> Self {
        Self { vp }
    }

    /// Evaluates the constraints and just gives you the gadget for the result.
    /// Caution for use in holographic lincheck: The output has 2 entries in one matrix
    pub fn evaluate_constraints<CS: ConstraintSystem<F>, FG>(&self, mut cs: CS, x: &FG) -> FG
    where
        FG: FieldGadget<F, F>,
    {
        let vp_cs = &mut cs.ns(|| "vanishing polynomial");
        if self.vp.dim_h == 1 {
            let result = x.sub(&mut vp_cs.ns(|| "compute result"), x).unwrap();
            return result;
        }
        let mut cur = x.square(vp_cs.ns(|| format!("compute x^(2^{:?})", 1))).unwrap();
        for i in 1..self.vp.dim_h {
            cur.square_in_place(vp_cs.ns(|| format!("compute x^(2^{:?})", i + 1)))
                .unwrap();
        }
        cur.sub_constant_in_place(vp_cs.ns(|| "compute result"), &self.vp.constant_term)
            .unwrap();
        cur
    }
}
