use snarkos_curves::templates::short_weierstrass::short_weierstrass_jacobian::{
    GroupAffine as SWAffine,
    GroupProjective as SWProjective,
};
use snarkos_errors::gadgets::SynthesisError;
use snarkos_models::{
    curves::{AffineCurve, Field, PrimeField, ProjectiveCurve, SWModelParameters},
    gadgets::{
        curves::{FieldGadget, GroupGadget},
        r1cs::{Assignment, ConstraintSystem},
        utilities::{
            alloc::AllocGadget,
            boolean::Boolean,
            eq::{ConditionalEqGadget, EqGadget, NEqGadget},
            select::CondSelectGadget,
            uint::UInt8,
            ToBitsGadget,
            ToBytesGadget,
        },
    },
};
use snarkos_utilities::bititerator::BitIterator;

use std::{borrow::Borrow, marker::PhantomData, ops::Neg};

#[derive(Derivative)]
#[derivative(Debug, Clone)]
#[must_use]
pub struct AffineGadget<P: SWModelParameters, F: Field, FG: FieldGadget<P::BaseField, F>> {
    pub x: FG,
    pub y: FG,
    pub infinity: Boolean,
    _parameters: PhantomData<P>,
    _engine: PhantomData<F>,
}

impl<P: SWModelParameters, F: Field, FG: FieldGadget<P::BaseField, F>> AffineGadget<P, F, FG> {
    pub fn new(x: FG, y: FG, infinity: Boolean) -> Self {
        Self {
            x,
            y,
            infinity,
            _parameters: PhantomData,
            _engine: PhantomData,
        }
    }

    pub fn alloc_without_check<Fn: FnOnce() -> Result<SWProjective<P>, SynthesisError>, CS: ConstraintSystem<F>>(
        mut cs: CS,
        value_gen: Fn,
    ) -> Result<Self, SynthesisError> {
        let (x, y, infinity) = match value_gen() {
            Ok(ge) => {
                let ge = ge.into_affine();
                (Ok(ge.x), Ok(ge.y), Ok(ge.infinity))
            }
            _ => (
                Err(SynthesisError::AssignmentMissing),
                Err(SynthesisError::AssignmentMissing),
                Err(SynthesisError::AssignmentMissing),
            ),
        };

        let x = FG::alloc(&mut cs.ns(|| "x"), || x)?;
        let y = FG::alloc(&mut cs.ns(|| "y"), || y)?;
        let infinity = Boolean::alloc(&mut cs.ns(|| "infinity"), || infinity)?;

        Ok(Self::new(x, y, infinity))
    }
}

impl<P, F, FG> PartialEq for AffineGadget<P, F, FG>
where
    P: SWModelParameters,
    F: Field,
    FG: FieldGadget<P::BaseField, F>,
{
    fn eq(&self, other: &Self) -> bool {
        self.x == other.x && self.y == other.y
    }
}

impl<P, F, FG> Eq for AffineGadget<P, F, FG>
where
    P: SWModelParameters,
    F: Field,
    FG: FieldGadget<P::BaseField, F>,
{
}

impl<P, F, FG> GroupGadget<SWProjective<P>, F> for AffineGadget<P, F, FG>
where
    P: SWModelParameters,
    F: PrimeField,
    FG: FieldGadget<P::BaseField, F>,
{
    type Value = SWProjective<P>;
    type Variable = (FG::Variable, FG::Variable);

    #[inline]
    fn get_value(&self) -> Option<Self::Value> {
        match (self.x.get_value(), self.y.get_value(), self.infinity.get_value()) {
            (Some(x), Some(y), Some(infinity)) => Some(SWAffine::new(x, y, infinity).into_projective()),
            (None, None, None) => None,
            _ => unreachable!(),
        }
    }

    #[inline]
    fn get_variable(&self) -> Self::Variable {
        (self.x.get_variable(), self.y.get_variable())
    }

    #[inline]
    fn zero<CS: ConstraintSystem<F>>(mut cs: CS) -> Result<Self, SynthesisError> {
        Ok(Self::new(
            FG::zero(cs.ns(|| "zero"))?,
            FG::one(cs.ns(|| "one"))?,
            Boolean::Constant(true),
        ))
    }

    #[inline]
    /// Incomplete addition: neither `self` nor `other` can be the neutral
    /// element.
    fn add<CS: ConstraintSystem<F>>(&self, mut cs: CS, other: &Self) -> Result<Self, SynthesisError> {
        // lambda = (B.y - A.y)/(B.x - A.x)
        // C.x = lambda^2 - A.x - B.x
        // C.y = lambda(A.x - C.x) - A.y
        //
        // Special cases:
        //
        // doubling: if B.y = A.y and B.x = A.x then lambda is unbound and
        // C = (lambda^2, lambda^3)
        //
        // addition of negative point: if B.y = -A.y and B.x = A.x then no
        // lambda can satisfy the first equation unless B.y - A.y = 0. But
        // then this reduces to doubling.
        //
        // So we need to check that A.x - B.x != 0, which can be done by
        // enforcing I * (B.x - A.x) = 1
        // This is done below when we calculate inv (by F::inverse)

        let x2_minus_x1 = other.x.sub(cs.ns(|| "x2 - x1"), &self.x)?;
        let y2_minus_y1 = other.y.sub(cs.ns(|| "y2 - y1"), &self.y)?;

        let inv = x2_minus_x1.inverse(cs.ns(|| "compute inv"))?;

        let lambda = FG::alloc(cs.ns(|| "lambda"), || {
            Ok(y2_minus_y1.get_value().get()? * &inv.get_value().get()?)
        })?;

        let x_3 = FG::alloc(&mut cs.ns(|| "x_3"), || {
            let lambda_val = lambda.get_value().get()?;
            let x1 = self.x.get_value().get()?;
            let x2 = other.x.get_value().get()?;
            Ok((lambda_val.square() - &x1) - &x2)
        })?;

        let y_3 = FG::alloc(&mut cs.ns(|| "y_3"), || {
            let lambda_val = lambda.get_value().get()?;
            let x_1 = self.x.get_value().get()?;
            let y_1 = self.y.get_value().get()?;
            let x_3 = x_3.get_value().get()?;
            Ok(lambda_val * &(x_1 - &x_3) - &y_1)
        })?;

        // Check lambda
        lambda.mul_equals(cs.ns(|| "check lambda"), &x2_minus_x1, &y2_minus_y1)?;

        // Check x3
        let x3_plus_x1_plus_x2 = x_3
            .add(cs.ns(|| "x3 + x1"), &self.x)?
            .add(cs.ns(|| "x3 + x1 + x2"), &other.x)?;
        lambda.mul_equals(cs.ns(|| "check x3"), &lambda, &x3_plus_x1_plus_x2)?;

        // Check y3
        let y3_plus_y1 = y_3.add(cs.ns(|| "y3 + y1"), &self.y)?;
        let x1_minus_x3 = self.x.sub(cs.ns(|| "x1 - x3"), &x_3)?;

        lambda.mul_equals(cs.ns(|| ""), &x1_minus_x3, &y3_plus_y1)?;

        Ok(Self::new(x_3, y_3, Boolean::Constant(false)))
    }

    /// Incomplete addition: neither `self` nor `other` can be the neutral
    /// element.
    fn add_constant<CS: ConstraintSystem<F>>(
        &self,
        mut cs: CS,
        other: &SWProjective<P>,
    ) -> Result<Self, SynthesisError> {
        // lambda = (B.y - A.y)/(B.x - A.x)
        // C.x = lambda^2 - A.x - B.x
        // C.y = lambda(A.x - C.x) - A.y
        //
        // Special cases:
        //
        // doubling: if B.y = A.y and B.x = A.x then lambda is unbound and
        // C = (lambda^2, lambda^3)
        //
        // addition of negative point: if B.y = -A.y and B.x = A.x then no
        // lambda can satisfy the first equation unless B.y - A.y = 0. But
        // then this reduces to doubling.
        //
        // So we need to check that A.x - B.x != 0, which can be done by
        // enforcing I * (B.x - A.x) = 1
        if other.is_zero() {
            return Err(SynthesisError::AssignmentMissing);
        }
        let other = other.into_affine();
        let other_x = other.x;
        let other_y = other.y;

        let x2_minus_x1 = self
            .x
            .sub_constant(cs.ns(|| "x2 - x1"), &other_x)?
            .negate(cs.ns(|| "neg1"))?;
        let y2_minus_y1 = self
            .y
            .sub_constant(cs.ns(|| "y2 - y1"), &other_y)?
            .negate(cs.ns(|| "neg2"))?;

        let inv = x2_minus_x1.inverse(cs.ns(|| "compute inv"))?;

        let lambda = FG::alloc(cs.ns(|| "lambda"), || {
            Ok(y2_minus_y1.get_value().get()? * &inv.get_value().get()?)
        })?;

        let x_3 = FG::alloc(&mut cs.ns(|| "x_3"), || {
            let lambda_val = lambda.get_value().get()?;
            let x1 = self.x.get_value().get()?;
            let x2 = other_x;
            Ok((lambda_val.square() - &x1) - &x2)
        })?;

        let y_3 = FG::alloc(&mut cs.ns(|| "y_3"), || {
            let lambda_val = lambda.get_value().get()?;
            let x_1 = self.x.get_value().get()?;
            let y_1 = self.y.get_value().get()?;
            let x_3 = x_3.get_value().get()?;
            Ok(lambda_val * &(x_1 - &x_3) - &y_1)
        })?;

        // Check lambda
        lambda.mul_equals(cs.ns(|| "check lambda"), &x2_minus_x1, &y2_minus_y1)?;

        // Check x3
        let x3_plus_x1_plus_x2 = x_3
            .add(cs.ns(|| "x3 + x1"), &self.x)?
            .add_constant(cs.ns(|| "x3 + x1 + x2"), &other_x)?;
        lambda.mul_equals(cs.ns(|| "check x3"), &lambda, &x3_plus_x1_plus_x2)?;

        // Check y3
        let y3_plus_y1 = y_3.add(cs.ns(|| "y3 + y1"), &self.y)?;
        let x1_minus_x3 = self.x.sub(cs.ns(|| "x1 - x3"), &x_3)?;

        lambda.mul_equals(cs.ns(|| ""), &x1_minus_x3, &y3_plus_y1)?;

        Ok(Self::new(x_3, y_3, Boolean::Constant(false)))
    }

    #[inline]
    fn double_in_place<CS: ConstraintSystem<F>>(&mut self, mut cs: CS) -> Result<(), SynthesisError> {
        let a = P::COEFF_A;
        let x_squared = self.x.square(cs.ns(|| "x^2"))?;

        let one = P::BaseField::one();
        let two = one.double();
        let three = two + &one;

        let three_x_squared = x_squared.mul_by_constant(cs.ns(|| "3 * x^2"), &three)?;
        let three_x_squared_plus_a = three_x_squared.add_constant(cs.ns(|| "3 * x^2 + a"), &a)?;

        let two_y = self.y.double(cs.ns(|| "2y"))?;

        let lambda = FG::alloc(cs.ns(|| "lambda"), || {
            let y_doubled_inv = two_y.get_value().get()?.inverse().get()?;
            Ok(three_x_squared_plus_a.get_value().get()? * &y_doubled_inv)
        })?;

        // Check lambda
        lambda.mul_equals(cs.ns(|| "check lambda"), &two_y, &three_x_squared_plus_a)?;

        let x = lambda
            .square(cs.ns(|| "lambda^2"))?
            .sub(cs.ns(|| "lambda^2 - x"), &self.x)?
            .sub(cs.ns(|| "lambda^2 - 2x"), &self.x)?;

        let y = self
            .x
            .sub(cs.ns(|| "x - self.x"), &x)?
            .mul(cs.ns(|| "times lambda"), &lambda)?
            .sub(cs.ns(|| "plus self.y"), &self.y)?;

        *self = Self::new(x, y, Boolean::Constant(false));
        Ok(())
    }

    fn negate<CS: ConstraintSystem<F>>(&self, mut cs: CS) -> Result<Self, SynthesisError> {
        Ok(Self::new(
            self.x.clone(),
            self.y.negate(cs.ns(|| "negate y"))?,
            self.infinity,
        ))
    }

    fn cost_of_add() -> usize {
        3 * FG::cost_of_mul() + FG::cost_of_inv()
    }

    fn cost_of_double() -> usize {
        4 * FG::cost_of_mul()
    }
}

impl<P, F, FG> CondSelectGadget<F> for AffineGadget<P, F, FG>
where
    P: SWModelParameters,
    F: PrimeField,
    FG: FieldGadget<P::BaseField, F>,
{
    #[inline]
    fn conditionally_select<CS: ConstraintSystem<F>>(
        mut cs: CS,
        cond: &Boolean,
        first: &Self,
        second: &Self,
    ) -> Result<Self, SynthesisError> {
        let x = FG::conditionally_select(&mut cs.ns(|| "x"), cond, &first.x, &second.x)?;
        let y = FG::conditionally_select(&mut cs.ns(|| "y"), cond, &first.y, &second.y)?;
        let infinity =
            Boolean::conditionally_select(&mut cs.ns(|| "infinity"), cond, &first.infinity, &second.infinity)?;

        Ok(Self::new(x, y, infinity))
    }

    fn cost() -> usize {
        2 * <FG as CondSelectGadget<F>>::cost() + <Boolean as CondSelectGadget<F>>::cost()
    }
}

impl<P, F, FG> EqGadget<F> for AffineGadget<P, F, FG>
where
    P: SWModelParameters,
    F: Field,
    FG: FieldGadget<P::BaseField, F>,
{
}

impl<P, F, FG> ConditionalEqGadget<F> for AffineGadget<P, F, FG>
where
    P: SWModelParameters,
    F: Field,
    FG: FieldGadget<P::BaseField, F>,
{
    #[inline]
    fn conditional_enforce_equal<CS: ConstraintSystem<F>>(
        &self,
        mut cs: CS,
        other: &Self,
        condition: &Boolean,
    ) -> Result<(), SynthesisError> {
        self.x
            .conditional_enforce_equal(&mut cs.ns(|| "X Coordinate Conditional Equality"), &other.x, condition)?;
        self.y
            .conditional_enforce_equal(&mut cs.ns(|| "Y Coordinate Conditional Equality"), &other.y, condition)?;
        self.infinity.conditional_enforce_equal(
            &mut cs.ns(|| "Infinity Conditional Equality"),
            &other.infinity,
            condition,
        )?;
        Ok(())
    }

    fn cost() -> usize {
        2 * <FG as ConditionalEqGadget<F>>::cost()
    }
}

impl<P, F, FG> NEqGadget<F> for AffineGadget<P, F, FG>
where
    P: SWModelParameters,
    F: Field,
    FG: FieldGadget<P::BaseField, F>,
{
    #[inline]
    fn enforce_not_equal<CS: ConstraintSystem<F>>(&self, mut cs: CS, other: &Self) -> Result<(), SynthesisError> {
        self.x
            .enforce_not_equal(&mut cs.ns(|| "X Coordinate Inequality"), &other.x)?;
        self.y
            .enforce_not_equal(&mut cs.ns(|| "Y Coordinate Inequality"), &other.y)?;
        Ok(())
    }

    fn cost() -> usize {
        2 * <FG as NEqGadget<F>>::cost()
    }
}

impl<P: SWModelParameters, F: PrimeField, FG: FieldGadget<P::BaseField, F>> AllocGadget<SWProjective<P>, F>
    for AffineGadget<P, F, FG>
{
    #[inline]
    fn alloc<Fn, T, CS: ConstraintSystem<F>>(mut cs: CS, value_gen: Fn) -> Result<Self, SynthesisError>
    where
        Fn: FnOnce() -> Result<T, SynthesisError>,
        T: Borrow<SWProjective<P>>,
    {
        let (x, y, infinity) = match value_gen() {
            Ok(ge) => {
                let ge = ge.borrow().into_affine();
                (Ok(ge.x), Ok(ge.y), Ok(ge.infinity))
            }
            _ => (
                Err(SynthesisError::AssignmentMissing),
                Err(SynthesisError::AssignmentMissing),
                Err(SynthesisError::AssignmentMissing),
            ),
        };

        // Perform on-curve check.
        let b = P::COEFF_B;
        let a = P::COEFF_A;

        let x = FG::alloc(&mut cs.ns(|| "x"), || x)?;
        let y = FG::alloc(&mut cs.ns(|| "y"), || y)?;
        let infinity = Boolean::alloc(&mut cs.ns(|| "infinity"), || infinity)?;

        // Check that y^2 = x^3 + ax +b
        // We do this by checking that y^2 - b = x * (x^2 +a)
        let x2 = x.square(&mut cs.ns(|| "x^2"))?;
        let y2 = y.square(&mut cs.ns(|| "y^2"))?;

        let x2_plus_a = x2.add_constant(cs.ns(|| "x^2 + a"), &a)?;
        let y2_minus_b = y2.add_constant(cs.ns(|| "y^2 - b"), &b.neg())?;

        x2_plus_a.mul_equals(cs.ns(|| "on curve check"), &x, &y2_minus_b)?;

        Ok(Self::new(x, y, infinity))
    }

    #[inline]
    fn alloc_checked<Fn, T, CS: ConstraintSystem<F>>(mut cs: CS, value_gen: Fn) -> Result<Self, SynthesisError>
    where
        Fn: FnOnce() -> Result<T, SynthesisError>,
        T: Borrow<SWProjective<P>>,
    {
        let cofactor_weight = BitIterator::new(P::COFACTOR).filter(|b| *b).count();
        // If we multiply by r, we actually multiply by r - 2.
        let r_minus_1 = (-P::ScalarField::one()).into_repr();
        let r_weight = BitIterator::new(&r_minus_1).filter(|b| *b).count();

        // We pick the most efficient method of performing the prime order check:
        // If the cofactor has lower hamming weight than the scalar field's modulus,
        // we first multiply by the inverse of the cofactor, and then, after allocating,
        // multiply by the cofactor. This ensures the resulting point has no cofactors
        //
        // Else, we multiply by the scalar field's modulus and ensure that the result
        // is zero.
        if cofactor_weight < r_weight {
            let ge = Self::alloc(cs.ns(|| "Alloc checked"), || {
                value_gen().map(|ge| ge.borrow().into_affine().mul_by_cofactor_inv().into_projective())
            })?;
            let mut seen_one = false;
            let mut result = Self::zero(cs.ns(|| "result"))?;
            for (i, b) in BitIterator::new(P::COFACTOR).enumerate() {
                let mut cs = cs.ns(|| format!("Iteration {}", i));

                let old_seen_one = seen_one;
                if seen_one {
                    result.double_in_place(cs.ns(|| "Double"))?;
                } else {
                    seen_one = b;
                }

                if b {
                    result = if old_seen_one {
                        result.add(cs.ns(|| "Add"), &ge)?
                    } else {
                        ge.clone()
                    };
                }
            }
            Ok(result)
        } else {
            let ge = Self::alloc(cs.ns(|| "Alloc checked"), value_gen)?;
            let mut seen_one = false;
            let mut result = Self::zero(cs.ns(|| "result"))?;
            // Returns bits in big-endian order
            for (i, b) in BitIterator::new(r_minus_1).enumerate() {
                let mut cs = cs.ns(|| format!("Iteration {}", i));

                let old_seen_one = seen_one;
                if seen_one {
                    result.double_in_place(cs.ns(|| "Double"))?;
                } else {
                    seen_one = b;
                }

                if b {
                    result = if old_seen_one {
                        result.add(cs.ns(|| "Add"), &ge)?
                    } else {
                        ge.clone()
                    };
                }
            }
            let neg_ge = ge.negate(cs.ns(|| "Negate ge"))?;
            neg_ge.enforce_equal(cs.ns(|| "Check equals"), &result)?;
            Ok(ge)
        }
    }

    #[inline]
    fn alloc_input<Fn, T, CS: ConstraintSystem<F>>(mut cs: CS, value_gen: Fn) -> Result<Self, SynthesisError>
    where
        Fn: FnOnce() -> Result<T, SynthesisError>,
        T: Borrow<SWProjective<P>>,
    {
        // When allocating the input we assume that the verifier has performed
        // any on curve checks already.
        let (x, y, infinity) = match value_gen() {
            Ok(ge) => {
                let ge = ge.borrow().into_affine();
                (Ok(ge.x), Ok(ge.y), Ok(ge.infinity))
            }
            _ => (
                Err(SynthesisError::AssignmentMissing),
                Err(SynthesisError::AssignmentMissing),
                Err(SynthesisError::AssignmentMissing),
            ),
        };

        let x = FG::alloc_input(&mut cs.ns(|| "x"), || x)?;
        let y = FG::alloc_input(&mut cs.ns(|| "y"), || y)?;
        let infinity = Boolean::alloc_input(&mut cs.ns(|| "infinity"), || infinity)?;

        Ok(Self::new(x, y, infinity))
    }
}

impl<P, F, FG> ToBitsGadget<F> for AffineGadget<P, F, FG>
where
    P: SWModelParameters,
    F: Field,
    FG: FieldGadget<P::BaseField, F>,
{
    fn to_bits<CS: ConstraintSystem<F>>(&self, mut cs: CS) -> Result<Vec<Boolean>, SynthesisError> {
        let mut x_bits = self.x.to_bits(&mut cs.ns(|| "X Coordinate To Bits"))?;
        let y_bits = self.y.to_bits(&mut cs.ns(|| "Y Coordinate To Bits"))?;
        x_bits.extend_from_slice(&y_bits);
        x_bits.push(self.infinity);
        Ok(x_bits)
    }

    fn to_bits_strict<CS: ConstraintSystem<F>>(&self, mut cs: CS) -> Result<Vec<Boolean>, SynthesisError> {
        let mut x_bits = self.x.to_bits_strict(&mut cs.ns(|| "X Coordinate To Bits"))?;
        let y_bits = self.y.to_bits_strict(&mut cs.ns(|| "Y Coordinate To Bits"))?;
        x_bits.extend_from_slice(&y_bits);
        x_bits.push(self.infinity);

        Ok(x_bits)
    }
}

impl<P, F, FG> ToBytesGadget<F> for AffineGadget<P, F, FG>
where
    P: SWModelParameters,
    F: Field,
    FG: FieldGadget<P::BaseField, F>,
{
    fn to_bytes<CS: ConstraintSystem<F>>(&self, mut cs: CS) -> Result<Vec<UInt8>, SynthesisError> {
        let mut x_bytes = self.x.to_bytes(&mut cs.ns(|| "X Coordinate To Bytes"))?;
        let y_bytes = self.y.to_bytes(&mut cs.ns(|| "Y Coordinate To Bytes"))?;
        let inf_bytes = self.infinity.to_bytes(&mut cs.ns(|| "Infinity to Bytes"))?;
        x_bytes.extend_from_slice(&y_bytes);
        x_bytes.extend_from_slice(&inf_bytes);
        Ok(x_bytes)
    }

    fn to_bytes_strict<CS: ConstraintSystem<F>>(&self, mut cs: CS) -> Result<Vec<UInt8>, SynthesisError> {
        let mut x_bytes = self.x.to_bytes_strict(&mut cs.ns(|| "X Coordinate To Bytes"))?;
        let y_bytes = self.y.to_bytes_strict(&mut cs.ns(|| "Y Coordinate To Bytes"))?;
        let inf_bytes = self.infinity.to_bytes(&mut cs.ns(|| "Infinity to Bytes"))?;
        x_bytes.extend_from_slice(&y_bytes);
        x_bytes.extend_from_slice(&inf_bytes);

        Ok(x_bytes)
    }
}
