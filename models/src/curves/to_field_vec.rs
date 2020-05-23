use crate::curves::{Field, Fp2, Fp2Parameters, FpParameters, PrimeField};
use snarkvm_errors::curves::ConstraintFieldError;

/// Types that can be converted to a vector of `F` elements. Useful for specifying
/// how public inputs to a constraint system should be represented inside
/// that constraint system.
pub trait ToConstraintField<F: Field> {
    fn to_field_elements(&self) -> Result<Vec<F>, ConstraintFieldError>;
}

impl<F: PrimeField> ToConstraintField<F> for F {
    fn to_field_elements(&self) -> Result<Vec<F>, ConstraintFieldError> {
        Ok(vec![*self])
    }
}

// Impl for base field
impl<F: Field> ToConstraintField<F> for [F] {
    #[inline]
    fn to_field_elements(&self) -> Result<Vec<F>, ConstraintFieldError> {
        Ok(self.to_vec())
    }
}

impl<F: Field> ToConstraintField<F> for () {
    #[inline]
    fn to_field_elements(&self) -> Result<Vec<F>, ConstraintFieldError> {
        Ok(Vec::new())
    }
}

// Impl for constraint Fp2<F>
impl<P: Fp2Parameters> ToConstraintField<P::Fp> for Fp2<P> {
    #[inline]
    fn to_field_elements(&self) -> Result<Vec<P::Fp>, ConstraintFieldError> {
        let mut c0 = self.c0.to_field_elements()?;
        let c1 = self.c1.to_field_elements()?;
        c0.extend_from_slice(&c1);
        Ok(c0)
    }
}

impl<F: PrimeField> ToConstraintField<F> for [u8] {
    #[inline]
    fn to_field_elements(&self) -> Result<Vec<F>, ConstraintFieldError> {
        let max_size = <F as PrimeField>::Params::CAPACITY / 8;
        let max_size = max_size as usize;
        let fes = self
            .chunks(max_size)
            .map(|chunk| {
                let mut chunk = chunk.to_vec();
                let len = chunk.len();
                for _ in len..(max_size + 1) {
                    chunk.push(0u8);
                }
                F::read(chunk.as_slice())
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(fes)
    }
}

impl<F: PrimeField> ToConstraintField<F> for [u8; 32] {
    #[inline]
    fn to_field_elements(&self) -> Result<Vec<F>, ConstraintFieldError> {
        self.as_ref().to_field_elements()
    }
}
