use crate::{
    curves::Field,
    gadgets::r1cs::{ConstraintVar, ConstraintVar::*, LinearCombination, Variable},
};

use std::ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub};

impl<F: Field> From<Variable> for ConstraintVar<F> {
    #[inline]
    fn from(var: Variable) -> Self {
        Var(var)
    }
}

impl<F: Field> From<(F, Variable)> for ConstraintVar<F> {
    #[inline]
    fn from(coeff_var: (F, Variable)) -> Self {
        LC(coeff_var.into())
    }
}

impl<F: Field> From<LinearCombination<F>> for ConstraintVar<F> {
    #[inline]
    fn from(lc: LinearCombination<F>) -> Self {
        LC(lc)
    }
}

impl<F: Field> From<(F, LinearCombination<F>)> for ConstraintVar<F> {
    #[inline]
    fn from((coeff, mut lc): (F, LinearCombination<F>)) -> Self {
        lc *= coeff;
        LC(lc)
    }
}

impl<F: Field> From<(F, ConstraintVar<F>)> for ConstraintVar<F> {
    #[inline]
    fn from((coeff, var): (F, ConstraintVar<F>)) -> Self {
        match var {
            LC(lc) => (coeff, lc).into(),
            Var(var) => (coeff, var).into(),
        }
    }
}

impl<F: Field> ConstraintVar<F> {
    /// Returns an empty linear combination.
    #[inline]
    pub fn zero() -> Self {
        LC(LinearCombination::zero())
    }

    /// Negate the coefficients of all variables in `self`.
    pub fn negate_in_place(&mut self) {
        match self {
            LC(ref mut lc) => lc.negate_in_place(),
            Var(var) => *self = (-F::one(), *var).into(),
        }
    }

    /// Double the coefficients of all variables in `self`.
    pub fn double_in_place(&mut self) {
        match self {
            LC(lc) => lc.double_in_place(),
            Var(var) => *self = (F::one().double(), *var).into(),
        }
    }
}

impl<F: Field> Add<LinearCombination<F>> for ConstraintVar<F> {
    type Output = LinearCombination<F>;

    #[inline]
    fn add(self, other_lc: LinearCombination<F>) -> LinearCombination<F> {
        match self {
            LC(lc) => other_lc + lc,
            Var(var) => other_lc + var,
        }
    }
}

impl<F: Field> Sub<LinearCombination<F>> for ConstraintVar<F> {
    type Output = LinearCombination<F>;

    #[inline]
    fn sub(self, other_lc: LinearCombination<F>) -> LinearCombination<F> {
        let result = match self {
            LC(lc) => other_lc - lc,
            Var(var) => other_lc - var,
        };
        -result
    }
}

impl<F: Field> Add<LinearCombination<F>> for &ConstraintVar<F> {
    type Output = LinearCombination<F>;

    #[inline]
    fn add(self, other_lc: LinearCombination<F>) -> LinearCombination<F> {
        match self {
            LC(lc) => other_lc + lc,
            Var(var) => other_lc + *var,
        }
    }
}

impl<F: Field> Sub<LinearCombination<F>> for &ConstraintVar<F> {
    type Output = LinearCombination<F>;

    #[inline]
    fn sub(self, other_lc: LinearCombination<F>) -> LinearCombination<F> {
        let result = match self {
            LC(lc) => other_lc - lc,
            Var(var) => other_lc - *var,
        };
        -result
    }
}

impl<F: Field> Add<(F, Variable)> for ConstraintVar<F> {
    type Output = Self;

    #[inline]
    fn add(self, var: (F, Variable)) -> Self {
        let lc = match self {
            LC(lc) => lc + var,
            Var(var2) => LinearCombination::from(var2) + var,
        };
        LC(lc)
    }
}

impl<F: Field> AddAssign<(F, Variable)> for ConstraintVar<F> {
    #[inline]
    fn add_assign(&mut self, var: (F, Variable)) {
        match self {
            LC(ref mut lc) => *lc += var,
            Var(var2) => *self = LC(LinearCombination::from(*var2) + var),
        };
    }
}

impl<F: Field> Neg for ConstraintVar<F> {
    type Output = Self;

    #[inline]
    fn neg(mut self) -> Self {
        self.negate_in_place();
        self
    }
}

impl<F: Field> Mul<F> for ConstraintVar<F> {
    type Output = Self;

    #[inline]
    fn mul(self, scalar: F) -> Self {
        match self {
            LC(lc) => LC(lc * scalar),
            Var(var) => (scalar, var).into(),
        }
    }
}

impl<F: Field> MulAssign<F> for ConstraintVar<F> {
    #[inline]
    fn mul_assign(&mut self, scalar: F) {
        match self {
            LC(lc) => *lc *= scalar,
            Var(var) => *self = (scalar, *var).into(),
        }
    }
}

impl<F: Field> Sub<(F, Variable)> for ConstraintVar<F> {
    type Output = Self;

    #[inline]
    fn sub(self, (coeff, var): (F, Variable)) -> Self {
        self + (-coeff, var)
    }
}

impl<F: Field> Add<Variable> for ConstraintVar<F> {
    type Output = Self;

    fn add(self, other: Variable) -> Self {
        self + (F::one(), other)
    }
}

impl<F: Field> Sub<Variable> for ConstraintVar<F> {
    type Output = Self;

    #[inline]
    fn sub(self, other: Variable) -> Self {
        self - (F::one(), other)
    }
}

impl<'a, F: Field> Add<&'a Self> for ConstraintVar<F> {
    type Output = Self;

    #[inline]
    fn add(self, other: &'a Self) -> Self {
        let lc = match self {
            LC(lc2) => lc2,
            Var(var) => var.into(),
        };
        let lc2 = match other {
            LC(lc2) => lc + lc2,
            Var(var) => lc + *var,
        };
        LC(lc2)
    }
}

impl<'a, F: Field> Sub<&'a Self> for ConstraintVar<F> {
    type Output = Self;

    #[inline]
    fn sub(self, other: &'a Self) -> Self {
        let lc = match self {
            LC(lc2) => lc2,
            Var(var) => var.into(),
        };
        let lc2 = match other {
            LC(lc2) => lc - lc2,
            Var(var) => lc - *var,
        };
        LC(lc2)
    }
}

impl<F: Field> Add<&ConstraintVar<F>> for &ConstraintVar<F> {
    type Output = ConstraintVar<F>;

    #[inline]
    fn add(self, other: &ConstraintVar<F>) -> Self::Output {
        (ConstraintVar::zero() + self) + other
    }
}

impl<F: Field> Sub<&ConstraintVar<F>> for &ConstraintVar<F> {
    type Output = ConstraintVar<F>;

    #[inline]
    fn sub(self, other: &ConstraintVar<F>) -> Self::Output {
        (ConstraintVar::zero() + self) - other
    }
}

impl<'a, F: Field> Add<(F, &'a Self)> for ConstraintVar<F> {
    type Output = Self;

    #[inline]
    fn add(self, (coeff, other): (F, &'a Self)) -> Self {
        let mut lc = match self {
            LC(lc2) => lc2,
            Var(var) => LinearCombination::zero() + var,
        };

        lc = match other {
            LC(lc2) => lc + (coeff, lc2),
            Var(var) => lc + (coeff, *var),
        };
        LC(lc)
    }
}

impl<'a, F: Field> Sub<(F, &'a Self)> for ConstraintVar<F> {
    type Output = Self;

    #[inline]
    fn sub(self, (coeff, other): (F, &'a Self)) -> Self {
        let mut lc = match self {
            LC(lc2) => lc2,
            Var(var) => LinearCombination::zero() + var,
        };
        lc = match other {
            LC(lc2) => lc - (coeff, lc2),
            Var(var) => lc - (coeff, *var),
        };
        LC(lc)
    }
}
