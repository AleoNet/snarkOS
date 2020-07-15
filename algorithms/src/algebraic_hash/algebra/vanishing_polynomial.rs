use snarkos_models::curves::Field;

#[derive(Clone)]
/// Struct describing vanishing polynomials for a multiplicative coset H,
/// with |H| a power of 2.
/// As H is a coset, every element can be described as h*g^i,
/// and therefore has vanishing polynomial Z_H(x) = x^|H| - h^|H|
pub struct VanishingPolynomial<F: Field> {
    /// h^|H|
    pub constant_term: F,
    /// log_2(|H|)
    pub dim_h: u64,
    // |H|
    pub order_h: u64,
}

impl<F: Field> VanishingPolynomial<F> {
    pub fn new(offset: F, dim_h: u64) -> Self {
        let order_h = 1 << dim_h;
        let vp = VanishingPolynomial {
            constant_term: offset.pow([order_h]),
            dim_h,
            order_h,
        };
        vp
    }

    pub fn evaluate(&self, x: &F) -> F {
        let mut result = x.pow([self.order_h]);
        result -= &self.constant_term;
        result
    }
}
