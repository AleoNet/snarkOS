use crate::{
    curves::{Field, FpParameters, PrimeField},
    gadgets::{
        r1cs::{Assignment, ConstraintSystem, ConstraintVar, LinearCombination, Variable},
        utilities::{
            alloc::AllocGadget,
            eq::{ConditionalEqGadget, EqGadget, EvaluateEqGadget},
            select::CondSelectGadget,
            uint::UInt8,
            ToBytesGadget,
        },
    },
};
use snarkos_errors::gadgets::SynthesisError;
use snarkos_utilities::bititerator::BitIterator;

use std::borrow::Borrow;

/// Represents a variable in the constraint system which is guaranteed
/// to be either zero or one.
#[derive(Copy, Clone, Debug)]
pub struct AllocatedBit {
    variable: Variable,
    value: Option<bool>,
}

impl AllocatedBit {
    pub fn get_value(&self) -> Option<bool> {
        self.value
    }

    pub fn get_variable(&self) -> Variable {
        self.variable
    }

    /// Performs an XOR operation over the two operands, returning
    /// an `AllocatedBit`.
    pub fn xor<F, CS>(mut cs: CS, a: &Self, b: &Self) -> Result<Self, SynthesisError>
    where
        F: Field,
        CS: ConstraintSystem<F>,
    {
        let mut result_value = None;

        let result_var = cs.alloc(
            || "xor result",
            || {
                if a.value.get()? ^ b.value.get()? {
                    result_value = Some(true);

                    Ok(F::one())
                } else {
                    result_value = Some(false);

                    Ok(F::zero())
                }
            },
        )?;

        // Constrain (a + a) * (b) = (a + b - c)
        // Given that a and b are boolean constrained, if they
        // are equal, the only solution for c is 0, and if they
        // are different, the only solution for c is 1.
        //
        // ¬(a ∧ b) ∧ ¬(¬a ∧ ¬b) = c
        // (1 - (a * b)) * (1 - ((1 - a) * (1 - b))) = c
        // (1 - ab) * (1 - (1 - a - b + ab)) = c
        // (1 - ab) * (a + b - ab) = c
        // a + b - ab - (a^2)b - (b^2)a + (a^2)(b^2) = c
        // a + b - ab - ab - ab + ab = c
        // a + b - 2ab = c
        // -2a * b = c - a - b
        // 2a * b = a + b - c
        // (a + a) * b = a + b - c
        cs.enforce(
            || "xor constraint",
            |lc| lc + a.variable + a.variable,
            |lc| lc + b.variable,
            |lc| lc + a.variable + b.variable - result_var,
        );

        Ok(AllocatedBit {
            variable: result_var,
            value: result_value,
        })
    }

    /// Performs an AND operation over the two operands, returning
    /// an `AllocatedBit`.
    pub fn and<F, CS>(mut cs: CS, a: &Self, b: &Self) -> Result<Self, SynthesisError>
    where
        F: Field,
        CS: ConstraintSystem<F>,
    {
        let mut result_value = None;

        let result_var = cs.alloc(
            || "and result",
            || {
                if a.value.get()? & b.value.get()? {
                    result_value = Some(true);

                    Ok(F::one())
                } else {
                    result_value = Some(false);

                    Ok(F::zero())
                }
            },
        )?;

        // Constrain (a) * (b) = (c), ensuring c is 1 iff
        // a AND b are both 1.
        cs.enforce(
            || "and constraint",
            |lc| lc + a.variable,
            |lc| lc + b.variable,
            |lc| lc + result_var,
        );

        Ok(AllocatedBit {
            variable: result_var,
            value: result_value,
        })
    }

    /// Performs an OR operation over the two operands, returning
    /// an `AllocatedBit`.
    pub fn or<F, CS>(mut cs: CS, a: &Self, b: &Self) -> Result<Self, SynthesisError>
    where
        F: Field,
        CS: ConstraintSystem<F>,
    {
        let mut result_value = None;

        let result_var = cs.alloc(
            || "or result",
            || {
                if a.value.get()? | b.value.get()? {
                    result_value = Some(true);
                    Ok(F::one())
                } else {
                    result_value = Some(false);
                    Ok(F::zero())
                }
            },
        )?;

        // Constrain (1 - a) * (1 - b) = (c), ensuring c is 1 iff
        // a and b are both false, and otherwise c is 0.
        cs.enforce(
            || "nor constraint",
            |lc| lc + CS::one() - a.variable,
            |lc| lc + CS::one() - b.variable,
            |lc| lc + CS::one() - result_var,
        );

        Ok(AllocatedBit {
            variable: result_var,
            value: result_value,
        })
    }

    /// Calculates `a AND (NOT b)`.
    pub fn and_not<F, CS>(mut cs: CS, a: &Self, b: &Self) -> Result<Self, SynthesisError>
    where
        F: Field,
        CS: ConstraintSystem<F>,
    {
        let mut result_value = None;

        let result_var = cs.alloc(
            || "and not result",
            || {
                if a.value.get()? & !b.value.get()? {
                    result_value = Some(true);

                    Ok(F::one())
                } else {
                    result_value = Some(false);

                    Ok(F::zero())
                }
            },
        )?;

        // Constrain (a) * (1 - b) = (c), ensuring c is 1 iff
        // a is true and b is false, and otherwise c is 0.
        cs.enforce(
            || "and not constraint",
            |lc| lc + a.variable,
            |lc| lc + CS::one() - b.variable,
            |lc| lc + result_var,
        );

        Ok(AllocatedBit {
            variable: result_var,
            value: result_value,
        })
    }

    /// Calculates `(NOT a) AND (NOT b)`.
    pub fn nor<F, CS>(mut cs: CS, a: &Self, b: &Self) -> Result<Self, SynthesisError>
    where
        F: Field,
        CS: ConstraintSystem<F>,
    {
        let mut result_value = None;

        let result_var = cs.alloc(
            || "nor result",
            || {
                if !a.value.get()? & !b.value.get()? {
                    result_value = Some(true);

                    Ok(F::one())
                } else {
                    result_value = Some(false);

                    Ok(F::zero())
                }
            },
        )?;

        // Constrain (1 - a) * (1 - b) = (c), ensuring c is 1 iff
        // a and b are both false, and otherwise c is 0.
        cs.enforce(
            || "nor constraint",
            |lc| lc + CS::one() - a.variable,
            |lc| lc + CS::one() - b.variable,
            |lc| lc + result_var,
        );

        Ok(AllocatedBit {
            variable: result_var,
            value: result_value,
        })
    }
}

impl PartialEq for AllocatedBit {
    fn eq(&self, other: &Self) -> bool {
        self.value.is_some() && other.value.is_some() && self.value == other.value
    }
}

impl Eq for AllocatedBit {}

impl<F: Field> AllocGadget<bool, F> for AllocatedBit {
    fn alloc<Fn, T, CS: ConstraintSystem<F>>(mut cs: CS, value_gen: Fn) -> Result<Self, SynthesisError>
    where
        Fn: FnOnce() -> Result<T, SynthesisError>,
        T: Borrow<bool>,
    {
        let mut value = None;

        let var = cs.alloc(
            || "boolean",
            || {
                value = Some(*value_gen()?.borrow());
                if value.get()? { Ok(F::one()) } else { Ok(F::zero()) }
            },
        )?;

        // Constrain: (1 - a) * a = 0
        // This constrains a to be either 0 or 1.
        cs.enforce(
            || "boolean constraint",
            |lc| lc + CS::one() - var,
            |lc| lc + var,
            |lc| lc,
        );

        Ok(AllocatedBit { variable: var, value })
    }

    fn alloc_input<Fn, T, CS: ConstraintSystem<F>>(mut cs: CS, value_gen: Fn) -> Result<Self, SynthesisError>
    where
        Fn: FnOnce() -> Result<T, SynthesisError>,
        T: Borrow<bool>,
    {
        let mut value = None;

        let var = cs.alloc_input(
            || "boolean",
            || {
                value = Some(*value_gen()?.borrow());
                if value.get()? { Ok(F::one()) } else { Ok(F::zero()) }
            },
        )?;

        // Constrain: (1 - a) * a = 0
        // This constrains a to be either 0 or 1.
        cs.enforce(
            || "boolean constraint",
            |lc| lc + CS::one() - var,
            |lc| lc + var,
            |lc| lc,
        );

        Ok(AllocatedBit { variable: var, value })
    }
}

impl<F: PrimeField> CondSelectGadget<F> for AllocatedBit {
    fn conditionally_select<CS: ConstraintSystem<F>>(
        cs: CS,
        cond: &Boolean,
        first: &Self,
        second: &Self,
    ) -> Result<Self, SynthesisError> {
        cond_select_helper(cs, cond, (first.value, first.variable), (second.value, second.variable))
    }

    fn cost() -> usize {
        1
    }
}

fn cond_select_helper<F: PrimeField, CS: ConstraintSystem<F>>(
    mut cs: CS,
    cond: &Boolean,
    first: (Option<bool>, impl Into<ConstraintVar<F>>),
    second: (Option<bool>, impl Into<ConstraintVar<F>>),
) -> Result<AllocatedBit, SynthesisError> {
    let mut result_val = None;
    let result_var = cs.alloc(
        || "cond_select_result",
        || {
            result_val = cond.get_value().and_then(|c| if c { first.0 } else { second.0 });
            result_val.get().map(|v| F::from(v as u8))
        },
    )?;

    let first_var = first.1.into();
    let second_var = second.1.into();

    // a = self; b = other; c = cond;
    //
    // r = c * a + (1  - c) * b
    // r = b + c * (a - b)
    // c * (a - b) = r - b
    let one = CS::one();
    cs.enforce(
        || "conditionally_select",
        |_| cond.lc(one, F::one()),
        |lc| (&first_var - &second_var) + lc,
        |lc| ConstraintVar::from(result_var) - &second_var + lc,
    );

    Ok(AllocatedBit {
        value: result_val,
        variable: result_var,
    })
}

/// This is a boolean value which may be either a constant or
/// an interpretation of an `AllocatedBit`.
#[derive(Copy, Clone, Debug)]
pub enum Boolean {
    /// Existential view of the boolean variable
    Is(AllocatedBit),
    /// Negated view of the boolean variable
    Not(AllocatedBit),
    /// Constant (not an allocated variable)
    Constant(bool),
}

impl Boolean {
    pub fn get_value(&self) -> Option<bool> {
        match *self {
            Boolean::Constant(c) => Some(c),
            Boolean::Is(ref v) => v.get_value(),
            Boolean::Not(ref v) => v.get_value().map(|b| !b),
        }
    }

    pub fn lc<F: Field>(&self, one: Variable, coeff: F) -> LinearCombination<F> {
        match *self {
            Boolean::Constant(c) => {
                if c {
                    (coeff, one).into()
                } else {
                    LinearCombination::<F>::zero()
                }
            }
            Boolean::Is(ref v) => (coeff, v.get_variable()).into(),
            Boolean::Not(ref v) => LinearCombination::<F>::zero() + (coeff, one) - (coeff, v.get_variable()),
        }
    }

    /// Construct a boolean vector from a vector of u8
    pub fn constant_u8_vec<F: Field, CS: ConstraintSystem<F>>(cs: &mut CS, values: &[u8]) -> Vec<Self> {
        let mut input_bits = vec![];
        for (byte_i, input_byte) in values.iter().enumerate() {
            for bit_i in (0..8).rev() {
                let cs = cs.ns(|| format!("input_bit_gadget {} {}", byte_i, bit_i));
                input_bits.push(
                    AllocatedBit::alloc(cs, || Ok((input_byte >> bit_i) & 1u8 == 1u8))
                        .unwrap()
                        .into(),
                );
            }
        }
        input_bits
    }

    /// Construct a boolean from a known constant.
    pub fn constant(b: bool) -> Self {
        Boolean::Constant(b)
    }

    /// Return a negated interpretation of this boolean.
    pub fn not(&self) -> Self {
        match *self {
            Boolean::Constant(c) => Boolean::Constant(!c),
            Boolean::Is(ref v) => Boolean::Not(*v),
            Boolean::Not(ref v) => Boolean::Is(*v),
        }
    }

    /// Perform XOR over two boolean operands.
    pub fn xor<'a, F, CS>(cs: CS, a: &'a Self, b: &'a Self) -> Result<Self, SynthesisError>
    where
        F: Field,
        CS: ConstraintSystem<F>,
    {
        match (a, b) {
            (&Boolean::Constant(false), x) | (x, &Boolean::Constant(false)) => Ok(*x),
            (&Boolean::Constant(true), x) | (x, &Boolean::Constant(true)) => Ok(x.not()),
            // a XOR (NOT b) = NOT(a XOR b)
            (is @ &Boolean::Is(_), not @ &Boolean::Not(_)) | (not @ &Boolean::Not(_), is @ &Boolean::Is(_)) => {
                Ok(Boolean::xor(cs, is, &not.not())?.not())
            }
            // a XOR b = (NOT a) XOR (NOT b)
            (&Boolean::Is(ref a), &Boolean::Is(ref b)) | (&Boolean::Not(ref a), &Boolean::Not(ref b)) => {
                Ok(Boolean::Is(AllocatedBit::xor(cs, a, b)?))
            }
        }
    }

    /// Perform OR over two boolean operands.
    pub fn or<'a, F, CS>(cs: CS, a: &'a Self, b: &'a Self) -> Result<Self, SynthesisError>
    where
        F: Field,
        CS: ConstraintSystem<F>,
    {
        match (a, b) {
            (&Boolean::Constant(false), x) | (x, &Boolean::Constant(false)) => Ok(*x),
            (&Boolean::Constant(true), _) | (_, &Boolean::Constant(true)) => Ok(Boolean::Constant(true)),
            // a OR b = NOT ((NOT a) AND b)
            (a @ &Boolean::Is(_), b @ &Boolean::Not(_))
            | (b @ &Boolean::Not(_), a @ &Boolean::Is(_))
            | (b @ &Boolean::Not(_), a @ &Boolean::Not(_)) => Ok(Boolean::and(cs, &a.not(), &b.not())?.not()),
            (&Boolean::Is(ref a), &Boolean::Is(ref b)) => AllocatedBit::or(cs, a, b).map(Boolean::from),
        }
    }

    /// Perform AND over two boolean operands.
    pub fn and<'a, F: Field, CS: ConstraintSystem<F>>(
        cs: CS,
        a: &'a Self,
        b: &'a Self,
    ) -> Result<Self, SynthesisError> {
        match (a, b) {
            // false AND x is always false
            (&Boolean::Constant(false), _) | (_, &Boolean::Constant(false)) => Ok(Boolean::Constant(false)),
            // true AND x is always x
            (&Boolean::Constant(true), x) | (x, &Boolean::Constant(true)) => Ok(*x),
            // a AND (NOT b)
            (&Boolean::Is(ref is), &Boolean::Not(ref not)) | (&Boolean::Not(ref not), &Boolean::Is(ref is)) => {
                Ok(Boolean::Is(AllocatedBit::and_not(cs, is, not)?))
            }
            // (NOT a) AND (NOT b) = a NOR b
            (&Boolean::Not(ref a), &Boolean::Not(ref b)) => Ok(Boolean::Is(AllocatedBit::nor(cs, a, b)?)),
            // a AND b
            (&Boolean::Is(ref a), &Boolean::Is(ref b)) => Ok(Boolean::Is(AllocatedBit::and(cs, a, b)?)),
        }
    }

    /// Perform AND over all the given boolean operands.
    pub fn kary_and<F, CS>(mut cs: CS, bits: &[Self]) -> Result<Self, SynthesisError>
    where
        F: Field,
        CS: ConstraintSystem<F>,
    {
        assert!(!bits.is_empty());
        let mut bits = bits.iter();

        let mut cur: Self = *bits.next().unwrap();
        for (i, next) in bits.enumerate() {
            cur = Boolean::and(cs.ns(|| format!("AND {}", i)), &cur, next)?;
        }

        Ok(cur)
    }

    /// Asserts that at least one operand is false.
    pub fn enforce_nand<F, CS>(mut cs: CS, bits: &[Self]) -> Result<(), SynthesisError>
    where
        F: Field,
        CS: ConstraintSystem<F>,
    {
        let res = Self::kary_and(&mut cs, bits)?;

        match res {
            Boolean::Constant(false) => Ok(()),
            Boolean::Constant(true) => Err(SynthesisError::AssignmentMissing),
            Boolean::Is(ref res) => {
                cs.enforce(|| "enforce nand", |lc| lc, |lc| lc, |lc| lc + res.get_variable());

                Ok(())
            }
            Boolean::Not(ref res) => {
                cs.enforce(
                    || "enforce nand",
                    |lc| lc,
                    |lc| lc,
                    |lc| lc + CS::one() - res.get_variable(),
                );

                Ok(())
            }
        }
    }

    /// Asserts that this bit_gadget representation is "in
    /// the field" when interpreted in big endian.
    pub fn enforce_in_field<F, CS, NativeF: PrimeField>(mut cs: CS, bits: &[Self]) -> Result<(), SynthesisError>
    where
        F: Field,
        CS: ConstraintSystem<F>,
    {
        let mut bits_iter = bits.iter();

        // b = char() - 1
        let mut b = NativeF::characteristic().to_vec();
        assert_eq!(b[0] % 2, 1);
        b[0] -= 1;

        // Runs of ones in r
        let mut last_run = Boolean::constant(true);
        let mut current_run = vec![];

        let mut found_one = false;
        let mut run_i = 0;
        let mut nand_i = 0;

        let char_num_bits = <NativeF as PrimeField>::Parameters::MODULUS_BITS as usize;
        if bits.len() > char_num_bits {
            let num_extra_bits = bits.len() - char_num_bits;
            let mut or_result = Boolean::constant(false);
            for (i, should_be_zero) in bits[0..num_extra_bits].iter().enumerate() {
                or_result = Boolean::or(&mut cs.ns(|| format!("Check {}-th or", i)), &or_result, should_be_zero)?;
                let _ = bits_iter.next().unwrap();
            }
            or_result.enforce_equal(
                &mut cs.ns(|| "Check that or of extra bits is zero"),
                &Boolean::constant(false),
            )?;
        }

        for b in BitIterator::new(b) {
            // Skip over unset bits at the beginning
            found_one |= b;
            if !found_one {
                continue;
            }

            let a = bits_iter.next().unwrap();

            if b {
                // This is part of a run of ones.
                current_run.push(a.clone());
            } else {
                if !current_run.is_empty() {
                    // This is the start of a run of zeros, but we need
                    // to k-ary AND against `last_run` first.

                    current_run.push(last_run);
                    last_run = Self::kary_and(cs.ns(|| format!("run {}", run_i)), &current_run)?;
                    run_i += 1;
                    current_run.truncate(0);
                }

                // If `last_run` is true, `a` must be false, or it would
                // not be in the field.
                //
                // If `last_run` is false, `a` can be true or false.
                //
                // Ergo, at least one of `last_run` and `a` must be false.
                Self::enforce_nand(cs.ns(|| format!("nand {}", nand_i)), &[last_run, *a])?;
                nand_i += 1;
            }
        }
        assert!(bits_iter.next().is_none());

        // We should always end in a "run" of zeros, because
        // the characteristic is an odd prime. So, this should
        // be empty.
        assert!(current_run.is_empty());

        Ok(())
    }
}

impl PartialEq for Boolean {
    fn eq(&self, other: &Self) -> bool {
        use self::Boolean::*;

        match (*self, *other) {
            (Is(a), Is(b)) | (Not(a), Not(b)) => a == b,
            (Is(a), Not(b)) | (Not(a), Is(b)) => a != b,
            (Is(a), Constant(b)) | (Constant(b), Is(a)) => a.value.unwrap() == b,
            (Not(a), Constant(b)) | (Constant(b), Not(a)) => a.value.unwrap() != b,
            (Constant(a), Constant(b)) => a == b,
        }
    }
}

impl Eq for Boolean {}

impl From<AllocatedBit> for Boolean {
    fn from(b: AllocatedBit) -> Boolean {
        Boolean::Is(b)
    }
}

/// a == b = !(a XOR b)
impl<F: PrimeField> EvaluateEqGadget<F> for Boolean {
    fn evaluate_equal<CS: ConstraintSystem<F>>(&self, cs: CS, other: &Self) -> Result<Boolean, SynthesisError> {
        let xor = Boolean::xor(cs, self, other)?;
        Ok(xor.not())
    }
}

impl<F: Field> AllocGadget<bool, F> for Boolean {
    fn alloc<Fn, T, CS: ConstraintSystem<F>>(cs: CS, value_gen: Fn) -> Result<Self, SynthesisError>
    where
        Fn: FnOnce() -> Result<T, SynthesisError>,
        T: Borrow<bool>,
    {
        AllocatedBit::alloc(cs, value_gen).map(Boolean::from)
    }

    fn alloc_input<Fn, T, CS: ConstraintSystem<F>>(cs: CS, value_gen: Fn) -> Result<Self, SynthesisError>
    where
        Fn: FnOnce() -> Result<T, SynthesisError>,
        T: Borrow<bool>,
    {
        AllocatedBit::alloc_input(cs, value_gen).map(Boolean::from)
    }
}

impl<F: Field> EqGadget<F> for Boolean {}

impl<F: Field> ConditionalEqGadget<F> for Boolean {
    fn conditional_enforce_equal<CS: ConstraintSystem<F>>(
        &self,
        mut cs: CS,
        other: &Self,
        condition: &Boolean,
    ) -> Result<(), SynthesisError> {
        use self::Boolean::*;
        let one = CS::one();
        let difference: LinearCombination<F> = match (self, other) {
            // 1 - 1 = 0 - 0 = 0
            (Constant(true), Constant(true)) | (Constant(false), Constant(false)) => return Ok(()),
            // false != true
            (Constant(_), Constant(_)) => return Err(SynthesisError::AssignmentMissing),
            // 1 - a
            (Constant(true), Is(a)) | (Is(a), Constant(true)) => LinearCombination::zero() + one - a.get_variable(),
            // a - 0 = a
            (Constant(false), Is(a)) | (Is(a), Constant(false)) => LinearCombination::zero() + a.get_variable(),
            // 1 - !a = 1 - (1 - a) = a
            (Constant(true), Not(a)) | (Not(a), Constant(true)) => LinearCombination::zero() + a.get_variable(),
            // !a - 0 = !a = 1 - a
            (Constant(false), Not(a)) | (Not(a), Constant(false)) => LinearCombination::zero() + one - a.get_variable(),
            // b - a,
            (Is(a), Is(b)) => LinearCombination::zero() + b.get_variable() - a.get_variable(),
            // !b - a = (1 - b) - a
            (Is(a), Not(b)) | (Not(b), Is(a)) => LinearCombination::zero() + one - b.get_variable() - a.get_variable(),
            // !b - !a = (1 - b) - (1 - a) = a - b,
            (Not(a), Not(b)) => LinearCombination::zero() + a.get_variable() - b.get_variable(),
        };

        if let Constant(false) = condition {
            Ok(())
        } else {
            cs.enforce(
                || "conditional_equals",
                |lc| difference + &lc,
                |lc| condition.lc(one, F::one()) + &lc,
                |lc| lc,
            );
            Ok(())
        }
    }

    fn cost() -> usize {
        1
    }
}

impl<F: Field> ToBytesGadget<F> for Boolean {
    fn to_bytes<CS: ConstraintSystem<F>>(&self, _cs: CS) -> Result<Vec<UInt8>, SynthesisError> {
        let mut bits = vec![Boolean::constant(false); 7];
        bits.push(*self);
        bits.reverse();
        let value = self.get_value().map(|val| val as u8);
        let byte = UInt8 {
            bits,
            negated: false,
            value,
        };
        Ok(vec![byte])
    }

    /// Additionally checks if the produced list of booleans is 'valid'.
    fn to_bytes_strict<CS: ConstraintSystem<F>>(&self, cs: CS) -> Result<Vec<UInt8>, SynthesisError> {
        self.to_bytes(cs)
    }
}

impl<F: PrimeField> CondSelectGadget<F> for Boolean {
    fn conditionally_select<CS>(mut cs: CS, cond: &Self, first: &Self, second: &Self) -> Result<Self, SynthesisError>
    where
        CS: ConstraintSystem<F>,
    {
        match cond {
            Boolean::Constant(true) => Ok(first.clone()),
            Boolean::Constant(false) => Ok(second.clone()),
            cond @ Boolean::Not(_) => Self::conditionally_select(cs, &cond.not(), second, first),
            cond @ Boolean::Is(_) => match (first, second) {
                (x, &Boolean::Constant(false)) => Boolean::and(cs.ns(|| "and"), cond, x).into(),
                (&Boolean::Constant(false), x) => Boolean::and(cs.ns(|| "and"), &cond.not(), x),
                (&Boolean::Constant(true), x) => Boolean::or(cs.ns(|| "or"), cond, x).into(),
                (x, &Boolean::Constant(true)) => Boolean::or(cs.ns(|| "or"), &cond.not(), x),
                (a @ Boolean::Is(_), b @ Boolean::Is(_))
                | (a @ Boolean::Not(_), b @ Boolean::Not(_))
                | (a @ Boolean::Is(_), b @ Boolean::Not(_))
                | (a @ Boolean::Not(_), b @ Boolean::Is(_)) => {
                    let a_lc = a.lc(CS::one(), F::one());
                    let b_lc = b.lc(CS::one(), F::one());
                    Ok(cond_select_helper(cs, cond, (a.get_value(), a_lc), (b.get_value(), b_lc))?.into())
                }
            },
        }
    }

    fn cost() -> usize {
        1
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        curves::{Field, One, PrimeField, Zero},
        gadgets::r1cs::{Fr, TestConstraintSystem},
    };
    use snarkos_utilities::{bititerator::BitIterator, rand::UniformRand};

    use rand::SeedableRng;
    use rand_xorshift::XorShiftRng;
    use std::str::FromStr;

    #[test]
    fn test_boolean_to_byte() {
        for val in [true, false].iter() {
            let mut cs = TestConstraintSystem::<Fr>::new();
            let a: Boolean = AllocatedBit::alloc(&mut cs, || Ok(*val)).unwrap().into();
            let bytes = a.to_bytes(&mut cs.ns(|| "ToBytes")).unwrap();
            assert_eq!(bytes.len(), 1);
            let byte = &bytes[0];
            assert_eq!(byte.value.unwrap(), *val as u8);

            for (i, bit_gadget) in byte.bits.iter().enumerate() {
                assert_eq!(bit_gadget.get_value().unwrap(), (byte.value.unwrap() >> i) & 1 == 1);
            }
        }
    }

    #[test]
    fn test_allocated_bit() {
        let mut cs = TestConstraintSystem::<Fr>::new();

        AllocatedBit::alloc(&mut cs, || Ok(true)).unwrap();
        assert!(cs.get("boolean") == Fr::one());
        assert!(cs.is_satisfied());
        cs.set("boolean", Fr::zero());
        assert!(cs.is_satisfied());
        cs.set("boolean", Fr::from_str("2").unwrap());
        assert!(!cs.is_satisfied());
        assert!(cs.which_is_unsatisfied() == Some("boolean constraint"));
    }

    #[test]
    fn test_xor() {
        for a_val in [false, true].iter() {
            for b_val in [false, true].iter() {
                let mut cs = TestConstraintSystem::<Fr>::new();
                let a = AllocatedBit::alloc(cs.ns(|| "a"), || Ok(*a_val)).unwrap();
                let b = AllocatedBit::alloc(cs.ns(|| "b"), || Ok(*b_val)).unwrap();
                let c = AllocatedBit::xor(&mut cs, &a, &b).unwrap();
                assert_eq!(c.value.unwrap(), *a_val ^ *b_val);

                assert!(cs.is_satisfied());
            }
        }
    }

    #[test]
    fn test_or() {
        for a_val in [false, true].iter() {
            for b_val in [false, true].iter() {
                let mut cs = TestConstraintSystem::<Fr>::new();
                let a = AllocatedBit::alloc(cs.ns(|| "a"), || Ok(*a_val)).unwrap();
                let b = AllocatedBit::alloc(cs.ns(|| "b"), || Ok(*b_val)).unwrap();
                let c = AllocatedBit::or(&mut cs, &a, &b).unwrap();
                assert_eq!(c.value.unwrap(), *a_val | *b_val);

                assert!(cs.is_satisfied());
                assert!(cs.get("a/boolean") == if *a_val { Fr::one() } else { Fr::zero() });
                assert!(cs.get("b/boolean") == if *b_val { Fr::one() } else { Fr::zero() });
            }
        }
    }

    #[test]
    fn test_and() {
        for a_val in [false, true].iter() {
            for b_val in [false, true].iter() {
                let mut cs = TestConstraintSystem::<Fr>::new();
                let a = AllocatedBit::alloc(cs.ns(|| "a"), || Ok(*a_val)).unwrap();
                let b = AllocatedBit::alloc(cs.ns(|| "b"), || Ok(*b_val)).unwrap();
                let c = AllocatedBit::and(&mut cs, &a, &b).unwrap();
                assert_eq!(c.value.unwrap(), *a_val & *b_val);

                assert!(cs.is_satisfied());
                assert!(cs.get("a/boolean") == if *a_val { Fr::one() } else { Fr::zero() });
                assert!(cs.get("b/boolean") == if *b_val { Fr::one() } else { Fr::zero() });
                assert!(cs.get("and result") == if *a_val & *b_val { Fr::one() } else { Fr::zero() });

                // Invert the result and check if the constraint system is still satisfied
                cs.set("and result", if *a_val & *b_val { Fr::zero() } else { Fr::one() });
                assert!(!cs.is_satisfied());
            }
        }
    }

    #[test]
    fn test_and_not() {
        for a_val in [false, true].iter() {
            for b_val in [false, true].iter() {
                let mut cs = TestConstraintSystem::<Fr>::new();
                let a = AllocatedBit::alloc(cs.ns(|| "a"), || Ok(*a_val)).unwrap();
                let b = AllocatedBit::alloc(cs.ns(|| "b"), || Ok(*b_val)).unwrap();
                let c = AllocatedBit::and_not(&mut cs, &a, &b).unwrap();
                assert_eq!(c.value.unwrap(), *a_val & !*b_val);

                assert!(cs.is_satisfied());
                assert!(cs.get("a/boolean") == if *a_val { Fr::one() } else { Fr::zero() });
                assert!(cs.get("b/boolean") == if *b_val { Fr::one() } else { Fr::zero() });
                assert!(cs.get("and not result") == if *a_val & !*b_val { Fr::one() } else { Fr::zero() });

                // Invert the result and check if the constraint system is still satisfied
                cs.set("and not result", if *a_val & !*b_val { Fr::zero() } else { Fr::one() });
                assert!(!cs.is_satisfied());
            }
        }
    }

    #[test]
    fn test_nor() {
        for a_val in [false, true].iter() {
            for b_val in [false, true].iter() {
                let mut cs = TestConstraintSystem::<Fr>::new();
                let a = AllocatedBit::alloc(cs.ns(|| "a"), || Ok(*a_val)).unwrap();
                let b = AllocatedBit::alloc(cs.ns(|| "b"), || Ok(*b_val)).unwrap();
                let c = AllocatedBit::nor(&mut cs, &a, &b).unwrap();
                assert_eq!(c.value.unwrap(), !*a_val & !*b_val);

                assert!(cs.is_satisfied());
                assert!(cs.get("a/boolean") == if *a_val { Fr::one() } else { Fr::zero() });
                assert!(cs.get("b/boolean") == if *b_val { Fr::one() } else { Fr::zero() });
                assert!(cs.get("nor result") == if !*a_val & !*b_val { Fr::one() } else { Fr::zero() });

                // Invert the result and check if the constraint system is still satisfied
                cs.set("nor result", if !*a_val & !*b_val { Fr::zero() } else { Fr::one() });
                assert!(!cs.is_satisfied());
            }
        }
    }

    #[test]
    fn test_enforce_equal() {
        for a_bool in [false, true].iter().cloned() {
            for b_bool in [false, true].iter().cloned() {
                for a_neg in [false, true].iter().cloned() {
                    for b_neg in [false, true].iter().cloned() {
                        let mut cs = TestConstraintSystem::<Fr>::new();

                        let mut a: Boolean = AllocatedBit::alloc(cs.ns(|| "a"), || Ok(a_bool)).unwrap().into();
                        let mut b: Boolean = AllocatedBit::alloc(cs.ns(|| "b"), || Ok(b_bool)).unwrap().into();

                        if a_neg {
                            a = a.not();
                        }
                        if b_neg {
                            b = b.not();
                        }

                        a.enforce_equal(&mut cs, &b).unwrap();

                        assert_eq!(cs.is_satisfied(), (a_bool ^ a_neg) == (b_bool ^ b_neg));
                    }
                }
            }
        }
    }

    #[test]
    fn test_conditional_enforce_equal() {
        for a_bool in [false, true].iter().cloned() {
            for b_bool in [false, true].iter().cloned() {
                for a_neg in [false, true].iter().cloned() {
                    for b_neg in [false, true].iter().cloned() {
                        let mut cs = TestConstraintSystem::<Fr>::new();

                        // First test if constraint system is satisfied
                        // when we do want to enforce the condition.
                        let mut a: Boolean = AllocatedBit::alloc(cs.ns(|| "a"), || Ok(a_bool)).unwrap().into();
                        let mut b: Boolean = AllocatedBit::alloc(cs.ns(|| "b"), || Ok(b_bool)).unwrap().into();

                        if a_neg {
                            a = a.not();
                        }
                        if b_neg {
                            b = b.not();
                        }

                        a.conditional_enforce_equal(&mut cs, &b, &Boolean::constant(true))
                            .unwrap();

                        assert_eq!(cs.is_satisfied(), (a_bool ^ a_neg) == (b_bool ^ b_neg));

                        // Now test if constraint system is satisfied even
                        // when we don't want to enforce the condition.
                        let mut cs = TestConstraintSystem::<Fr>::new();

                        let mut a: Boolean = AllocatedBit::alloc(cs.ns(|| "a"), || Ok(a_bool)).unwrap().into();
                        let mut b: Boolean = AllocatedBit::alloc(cs.ns(|| "b"), || Ok(b_bool)).unwrap().into();

                        if a_neg {
                            a = a.not();
                        }
                        if b_neg {
                            b = b.not();
                        }

                        let false_cond = AllocatedBit::alloc(cs.ns(|| "cond"), || Ok(false)).unwrap().into();
                        a.conditional_enforce_equal(&mut cs, &b, &false_cond).unwrap();

                        assert!(cs.is_satisfied());
                    }
                }
            }
        }
    }

    #[test]
    fn test_boolean_negation() {
        let mut cs = TestConstraintSystem::<Fr>::new();

        let mut b = Boolean::from(AllocatedBit::alloc(&mut cs, || Ok(true)).unwrap());

        match b {
            Boolean::Is(_) => {}
            _ => panic!("unexpected value"),
        }

        b = b.not();

        match b {
            Boolean::Not(_) => {}
            _ => panic!("unexpected value"),
        }

        b = b.not();

        match b {
            Boolean::Is(_) => {}
            _ => panic!("unexpected value"),
        }

        b = Boolean::constant(true);

        match b {
            Boolean::Constant(true) => {}
            _ => panic!("unexpected value"),
        }

        b = b.not();

        match b {
            Boolean::Constant(false) => {}
            _ => panic!("unexpected value"),
        }

        b = b.not();

        match b {
            Boolean::Constant(true) => {}
            _ => panic!("unexpected value"),
        }
    }

    #[derive(Copy, Clone, Debug)]
    enum OperandType {
        True,
        False,
        AllocatedTrue,
        AllocatedFalse,
        NegatedAllocatedTrue,
        NegatedAllocatedFalse,
    }

    #[test]
    fn test_boolean_xor() {
        let variants = [
            OperandType::True,
            OperandType::False,
            OperandType::AllocatedTrue,
            OperandType::AllocatedFalse,
            OperandType::NegatedAllocatedTrue,
            OperandType::NegatedAllocatedFalse,
        ];

        for first_operand in variants.iter().cloned() {
            for second_operand in variants.iter().cloned() {
                let mut cs = TestConstraintSystem::<Fr>::new();

                let a;
                let b;

                {
                    let mut dyn_construct = |operand, name| {
                        let cs = cs.ns(|| name);

                        match operand {
                            OperandType::True => Boolean::constant(true),
                            OperandType::False => Boolean::constant(false),
                            OperandType::AllocatedTrue => Boolean::from(AllocatedBit::alloc(cs, || Ok(true)).unwrap()),
                            OperandType::AllocatedFalse => {
                                Boolean::from(AllocatedBit::alloc(cs, || Ok(false)).unwrap())
                            }
                            OperandType::NegatedAllocatedTrue => {
                                Boolean::from(AllocatedBit::alloc(cs, || Ok(true)).unwrap()).not()
                            }
                            OperandType::NegatedAllocatedFalse => {
                                Boolean::from(AllocatedBit::alloc(cs, || Ok(false)).unwrap()).not()
                            }
                        }
                    };

                    a = dyn_construct(first_operand, "a");
                    b = dyn_construct(second_operand, "b");
                }

                let c = Boolean::xor(&mut cs, &a, &b).unwrap();

                assert!(cs.is_satisfied());

                match (first_operand, second_operand, c) {
                    (OperandType::True, OperandType::True, Boolean::Constant(false)) => {}
                    (OperandType::True, OperandType::False, Boolean::Constant(true)) => {}
                    (OperandType::True, OperandType::AllocatedTrue, Boolean::Not(_)) => {}
                    (OperandType::True, OperandType::AllocatedFalse, Boolean::Not(_)) => {}
                    (OperandType::True, OperandType::NegatedAllocatedTrue, Boolean::Is(_)) => {}
                    (OperandType::True, OperandType::NegatedAllocatedFalse, Boolean::Is(_)) => {}

                    (OperandType::False, OperandType::True, Boolean::Constant(true)) => {}
                    (OperandType::False, OperandType::False, Boolean::Constant(false)) => {}
                    (OperandType::False, OperandType::AllocatedTrue, Boolean::Is(_)) => {}
                    (OperandType::False, OperandType::AllocatedFalse, Boolean::Is(_)) => {}
                    (OperandType::False, OperandType::NegatedAllocatedTrue, Boolean::Not(_)) => {}
                    (OperandType::False, OperandType::NegatedAllocatedFalse, Boolean::Not(_)) => {}

                    (OperandType::AllocatedTrue, OperandType::True, Boolean::Not(_)) => {}
                    (OperandType::AllocatedTrue, OperandType::False, Boolean::Is(_)) => {}
                    (OperandType::AllocatedTrue, OperandType::AllocatedTrue, Boolean::Is(ref v)) => {
                        assert!(cs.get("xor result") == Fr::zero());
                        assert_eq!(v.value, Some(false));
                    }
                    (OperandType::AllocatedTrue, OperandType::AllocatedFalse, Boolean::Is(ref v)) => {
                        assert!(cs.get("xor result") == Fr::one());
                        assert_eq!(v.value, Some(true));
                    }
                    (OperandType::AllocatedTrue, OperandType::NegatedAllocatedTrue, Boolean::Not(ref v)) => {
                        assert!(cs.get("xor result") == Fr::zero());
                        assert_eq!(v.value, Some(false));
                    }
                    (OperandType::AllocatedTrue, OperandType::NegatedAllocatedFalse, Boolean::Not(ref v)) => {
                        assert!(cs.get("xor result") == Fr::one());
                        assert_eq!(v.value, Some(true));
                    }

                    (OperandType::AllocatedFalse, OperandType::True, Boolean::Not(_)) => {}
                    (OperandType::AllocatedFalse, OperandType::False, Boolean::Is(_)) => {}
                    (OperandType::AllocatedFalse, OperandType::AllocatedTrue, Boolean::Is(ref v)) => {
                        assert!(cs.get("xor result") == Fr::one());
                        assert_eq!(v.value, Some(true));
                    }
                    (OperandType::AllocatedFalse, OperandType::AllocatedFalse, Boolean::Is(ref v)) => {
                        assert!(cs.get("xor result") == Fr::zero());
                        assert_eq!(v.value, Some(false));
                    }
                    (OperandType::AllocatedFalse, OperandType::NegatedAllocatedTrue, Boolean::Not(ref v)) => {
                        assert!(cs.get("xor result") == Fr::one());
                        assert_eq!(v.value, Some(true));
                    }
                    (OperandType::AllocatedFalse, OperandType::NegatedAllocatedFalse, Boolean::Not(ref v)) => {
                        assert!(cs.get("xor result") == Fr::zero());
                        assert_eq!(v.value, Some(false));
                    }

                    (OperandType::NegatedAllocatedTrue, OperandType::True, Boolean::Is(_)) => {}
                    (OperandType::NegatedAllocatedTrue, OperandType::False, Boolean::Not(_)) => {}
                    (OperandType::NegatedAllocatedTrue, OperandType::AllocatedTrue, Boolean::Not(ref v)) => {
                        assert!(cs.get("xor result") == Fr::zero());
                        assert_eq!(v.value, Some(false));
                    }
                    (OperandType::NegatedAllocatedTrue, OperandType::AllocatedFalse, Boolean::Not(ref v)) => {
                        assert!(cs.get("xor result") == Fr::one());
                        assert_eq!(v.value, Some(true));
                    }
                    (OperandType::NegatedAllocatedTrue, OperandType::NegatedAllocatedTrue, Boolean::Is(ref v)) => {
                        assert!(cs.get("xor result") == Fr::zero());
                        assert_eq!(v.value, Some(false));
                    }
                    (OperandType::NegatedAllocatedTrue, OperandType::NegatedAllocatedFalse, Boolean::Is(ref v)) => {
                        assert!(cs.get("xor result") == Fr::one());
                        assert_eq!(v.value, Some(true));
                    }

                    (OperandType::NegatedAllocatedFalse, OperandType::True, Boolean::Is(_)) => {}
                    (OperandType::NegatedAllocatedFalse, OperandType::False, Boolean::Not(_)) => {}
                    (OperandType::NegatedAllocatedFalse, OperandType::AllocatedTrue, Boolean::Not(ref v)) => {
                        assert!(cs.get("xor result") == Fr::one());
                        assert_eq!(v.value, Some(true));
                    }
                    (OperandType::NegatedAllocatedFalse, OperandType::AllocatedFalse, Boolean::Not(ref v)) => {
                        assert!(cs.get("xor result") == Fr::zero());
                        assert_eq!(v.value, Some(false));
                    }
                    (OperandType::NegatedAllocatedFalse, OperandType::NegatedAllocatedTrue, Boolean::Is(ref v)) => {
                        assert!(cs.get("xor result") == Fr::one());
                        assert_eq!(v.value, Some(true));
                    }
                    (OperandType::NegatedAllocatedFalse, OperandType::NegatedAllocatedFalse, Boolean::Is(ref v)) => {
                        assert!(cs.get("xor result") == Fr::zero());
                        assert_eq!(v.value, Some(false));
                    }

                    _ => panic!("this should never be encountered"),
                }
            }
        }
    }

    #[test]
    fn test_boolean_cond_select() {
        let variants = [
            OperandType::True,
            OperandType::False,
            OperandType::AllocatedTrue,
            OperandType::AllocatedFalse,
            OperandType::NegatedAllocatedTrue,
            OperandType::NegatedAllocatedFalse,
        ];

        for condition in variants.iter().cloned() {
            for first_operand in variants.iter().cloned() {
                for second_operand in variants.iter().cloned() {
                    let mut cs = TestConstraintSystem::<Fr>::new();

                    let cond;
                    let a;
                    let b;

                    {
                        let mut dyn_construct = |operand, name| {
                            let cs = cs.ns(|| name);

                            match operand {
                                OperandType::True => Boolean::constant(true),
                                OperandType::False => Boolean::constant(false),
                                OperandType::AllocatedTrue => {
                                    Boolean::from(AllocatedBit::alloc(cs, || Ok(true)).unwrap())
                                }
                                OperandType::AllocatedFalse => {
                                    Boolean::from(AllocatedBit::alloc(cs, || Ok(false)).unwrap())
                                }
                                OperandType::NegatedAllocatedTrue => {
                                    Boolean::from(AllocatedBit::alloc(cs, || Ok(true)).unwrap()).not()
                                }
                                OperandType::NegatedAllocatedFalse => {
                                    Boolean::from(AllocatedBit::alloc(cs, || Ok(false)).unwrap()).not()
                                }
                            }
                        };

                        cond = dyn_construct(condition, "cond");
                        a = dyn_construct(first_operand, "a");
                        b = dyn_construct(second_operand, "b");
                    }

                    let before = cs.num_constraints();
                    let c = Boolean::conditionally_select(&mut cs, &cond, &a, &b).unwrap();
                    let after = cs.num_constraints();

                    assert!(
                        cs.is_satisfied(),
                        "failed with operands: cond: {:?}, a: {:?}, b: {:?}",
                        condition,
                        first_operand,
                        second_operand,
                    );
                    assert_eq!(
                        c.get_value(),
                        if cond.get_value().unwrap() {
                            a.get_value()
                        } else {
                            b.get_value()
                        }
                    );
                    assert!(<Boolean as CondSelectGadget<Fr>>::cost() >= after - before);
                }
            }
        }
    }

    #[test]
    fn test_boolean_or() {
        let variants = [
            OperandType::True,
            OperandType::False,
            OperandType::AllocatedTrue,
            OperandType::AllocatedFalse,
            OperandType::NegatedAllocatedTrue,
            OperandType::NegatedAllocatedFalse,
        ];

        for first_operand in variants.iter().cloned() {
            for second_operand in variants.iter().cloned() {
                let mut cs = TestConstraintSystem::<Fr>::new();

                let a;
                let b;

                {
                    let mut dyn_construct = |operand, name| {
                        let cs = cs.ns(|| name);

                        match operand {
                            OperandType::True => Boolean::constant(true),
                            OperandType::False => Boolean::constant(false),
                            OperandType::AllocatedTrue => Boolean::from(AllocatedBit::alloc(cs, || Ok(true)).unwrap()),
                            OperandType::AllocatedFalse => {
                                Boolean::from(AllocatedBit::alloc(cs, || Ok(false)).unwrap())
                            }
                            OperandType::NegatedAllocatedTrue => {
                                Boolean::from(AllocatedBit::alloc(cs, || Ok(true)).unwrap()).not()
                            }
                            OperandType::NegatedAllocatedFalse => {
                                Boolean::from(AllocatedBit::alloc(cs, || Ok(false)).unwrap()).not()
                            }
                        }
                    };

                    a = dyn_construct(first_operand, "a");
                    b = dyn_construct(second_operand, "b");
                }

                let c = Boolean::or(&mut cs, &a, &b).unwrap();

                assert!(cs.is_satisfied());

                match (first_operand, second_operand, c) {
                    (OperandType::True, OperandType::True, Boolean::Constant(true)) => {}
                    (OperandType::True, OperandType::False, Boolean::Constant(true)) => {}
                    (OperandType::True, OperandType::AllocatedTrue, Boolean::Constant(true)) => {}
                    (OperandType::True, OperandType::AllocatedFalse, Boolean::Constant(true)) => {}
                    (OperandType::True, OperandType::NegatedAllocatedTrue, Boolean::Constant(true)) => {}
                    (OperandType::True, OperandType::NegatedAllocatedFalse, Boolean::Constant(true)) => {}

                    (OperandType::False, OperandType::True, Boolean::Constant(true)) => {}
                    (OperandType::False, OperandType::False, Boolean::Constant(false)) => {}
                    (OperandType::False, OperandType::AllocatedTrue, Boolean::Is(_)) => {}
                    (OperandType::False, OperandType::AllocatedFalse, Boolean::Is(_)) => {}
                    (OperandType::False, OperandType::NegatedAllocatedTrue, Boolean::Not(_)) => {}
                    (OperandType::False, OperandType::NegatedAllocatedFalse, Boolean::Not(_)) => {}

                    (OperandType::AllocatedTrue, OperandType::True, Boolean::Constant(true)) => {}
                    (OperandType::AllocatedTrue, OperandType::False, Boolean::Is(_)) => {}
                    (OperandType::AllocatedTrue, OperandType::AllocatedTrue, Boolean::Is(ref v)) => {
                        assert_eq!(v.value, Some(true));
                    }
                    (OperandType::AllocatedTrue, OperandType::AllocatedFalse, Boolean::Is(ref v)) => {
                        assert_eq!(v.value, Some(true));
                    }
                    (OperandType::AllocatedTrue, OperandType::NegatedAllocatedTrue, Boolean::Not(ref v)) => {
                        assert_eq!(v.value, Some(false));
                    }
                    (OperandType::AllocatedTrue, OperandType::NegatedAllocatedFalse, Boolean::Not(ref v)) => {
                        assert_eq!(v.value, Some(false));
                    }

                    (OperandType::AllocatedFalse, OperandType::True, Boolean::Constant(true)) => {}
                    (OperandType::AllocatedFalse, OperandType::False, Boolean::Is(_)) => {}
                    (OperandType::AllocatedFalse, OperandType::AllocatedTrue, Boolean::Is(ref v)) => {
                        assert_eq!(v.value, Some(true));
                    }
                    (OperandType::AllocatedFalse, OperandType::AllocatedFalse, Boolean::Is(ref v)) => {
                        assert_eq!(v.value, Some(false));
                    }
                    (OperandType::AllocatedFalse, OperandType::NegatedAllocatedTrue, Boolean::Not(ref v)) => {
                        assert_eq!(v.value, Some(true));
                    }
                    (OperandType::AllocatedFalse, OperandType::NegatedAllocatedFalse, Boolean::Not(ref v)) => {
                        assert_eq!(v.value, Some(false));
                    }

                    (OperandType::NegatedAllocatedTrue, OperandType::True, Boolean::Constant(true)) => {}
                    (OperandType::NegatedAllocatedTrue, OperandType::False, Boolean::Not(_)) => {}
                    (OperandType::NegatedAllocatedTrue, OperandType::AllocatedTrue, Boolean::Not(ref v)) => {
                        assert_eq!(v.value, Some(false));
                    }
                    (OperandType::NegatedAllocatedTrue, OperandType::AllocatedFalse, Boolean::Not(ref v)) => {
                        assert_eq!(v.value, Some(true));
                    }
                    (OperandType::NegatedAllocatedTrue, OperandType::NegatedAllocatedTrue, Boolean::Not(ref v)) => {
                        assert_eq!(v.value, Some(true));
                    }
                    (OperandType::NegatedAllocatedTrue, OperandType::NegatedAllocatedFalse, Boolean::Not(ref v)) => {
                        assert_eq!(v.value, Some(false));
                    }

                    (OperandType::NegatedAllocatedFalse, OperandType::True, Boolean::Constant(true)) => {}
                    (OperandType::NegatedAllocatedFalse, OperandType::False, Boolean::Not(_)) => {}
                    (OperandType::NegatedAllocatedFalse, OperandType::AllocatedTrue, Boolean::Not(ref v)) => {
                        assert_eq!(v.value, Some(false));
                    }
                    (OperandType::NegatedAllocatedFalse, OperandType::AllocatedFalse, Boolean::Not(ref v)) => {
                        assert_eq!(v.value, Some(false));
                    }
                    (OperandType::NegatedAllocatedFalse, OperandType::NegatedAllocatedTrue, Boolean::Not(ref v)) => {
                        assert_eq!(v.value, Some(false));
                    }
                    (OperandType::NegatedAllocatedFalse, OperandType::NegatedAllocatedFalse, Boolean::Not(ref v)) => {
                        assert_eq!(v.value, Some(false));
                    }

                    _ => panic!(
                        "this should never be encountered, in case: (a = {:?}, b = {:?}, c = {:?})",
                        a, b, c
                    ),
                }
            }
        }
    }

    #[test]
    fn test_boolean_and() {
        let variants = [
            OperandType::True,
            OperandType::False,
            OperandType::AllocatedTrue,
            OperandType::AllocatedFalse,
            OperandType::NegatedAllocatedTrue,
            OperandType::NegatedAllocatedFalse,
        ];

        for first_operand in variants.iter().cloned() {
            for second_operand in variants.iter().cloned() {
                let mut cs = TestConstraintSystem::<Fr>::new();

                let a;
                let b;

                {
                    let mut dyn_construct = |operand, name| {
                        let cs = cs.ns(|| name);

                        match operand {
                            OperandType::True => Boolean::constant(true),
                            OperandType::False => Boolean::constant(false),
                            OperandType::AllocatedTrue => Boolean::from(AllocatedBit::alloc(cs, || Ok(true)).unwrap()),
                            OperandType::AllocatedFalse => {
                                Boolean::from(AllocatedBit::alloc(cs, || Ok(false)).unwrap())
                            }
                            OperandType::NegatedAllocatedTrue => {
                                Boolean::from(AllocatedBit::alloc(cs, || Ok(true)).unwrap()).not()
                            }
                            OperandType::NegatedAllocatedFalse => {
                                Boolean::from(AllocatedBit::alloc(cs, || Ok(false)).unwrap()).not()
                            }
                        }
                    };

                    a = dyn_construct(first_operand, "a");
                    b = dyn_construct(second_operand, "b");
                }

                let c = Boolean::and(&mut cs, &a, &b).unwrap();

                assert!(cs.is_satisfied());

                match (first_operand, second_operand, c) {
                    (OperandType::True, OperandType::True, Boolean::Constant(true)) => {}
                    (OperandType::True, OperandType::False, Boolean::Constant(false)) => {}
                    (OperandType::True, OperandType::AllocatedTrue, Boolean::Is(_)) => {}
                    (OperandType::True, OperandType::AllocatedFalse, Boolean::Is(_)) => {}
                    (OperandType::True, OperandType::NegatedAllocatedTrue, Boolean::Not(_)) => {}
                    (OperandType::True, OperandType::NegatedAllocatedFalse, Boolean::Not(_)) => {}

                    (OperandType::False, OperandType::True, Boolean::Constant(false)) => {}
                    (OperandType::False, OperandType::False, Boolean::Constant(false)) => {}
                    (OperandType::False, OperandType::AllocatedTrue, Boolean::Constant(false)) => {}
                    (OperandType::False, OperandType::AllocatedFalse, Boolean::Constant(false)) => {}
                    (OperandType::False, OperandType::NegatedAllocatedTrue, Boolean::Constant(false)) => {}
                    (OperandType::False, OperandType::NegatedAllocatedFalse, Boolean::Constant(false)) => {}

                    (OperandType::AllocatedTrue, OperandType::True, Boolean::Is(_)) => {}
                    (OperandType::AllocatedTrue, OperandType::False, Boolean::Constant(false)) => {}
                    (OperandType::AllocatedTrue, OperandType::AllocatedTrue, Boolean::Is(ref v)) => {
                        assert!(cs.get("and result") == Fr::one());
                        assert_eq!(v.value, Some(true));
                    }
                    (OperandType::AllocatedTrue, OperandType::AllocatedFalse, Boolean::Is(ref v)) => {
                        assert!(cs.get("and result") == Fr::zero());
                        assert_eq!(v.value, Some(false));
                    }
                    (OperandType::AllocatedTrue, OperandType::NegatedAllocatedTrue, Boolean::Is(ref v)) => {
                        assert!(cs.get("and not result") == Fr::zero());
                        assert_eq!(v.value, Some(false));
                    }
                    (OperandType::AllocatedTrue, OperandType::NegatedAllocatedFalse, Boolean::Is(ref v)) => {
                        assert!(cs.get("and not result") == Fr::one());
                        assert_eq!(v.value, Some(true));
                    }

                    (OperandType::AllocatedFalse, OperandType::True, Boolean::Is(_)) => {}
                    (OperandType::AllocatedFalse, OperandType::False, Boolean::Constant(false)) => {}
                    (OperandType::AllocatedFalse, OperandType::AllocatedTrue, Boolean::Is(ref v)) => {
                        assert!(cs.get("and result") == Fr::zero());
                        assert_eq!(v.value, Some(false));
                    }
                    (OperandType::AllocatedFalse, OperandType::AllocatedFalse, Boolean::Is(ref v)) => {
                        assert!(cs.get("and result") == Fr::zero());
                        assert_eq!(v.value, Some(false));
                    }
                    (OperandType::AllocatedFalse, OperandType::NegatedAllocatedTrue, Boolean::Is(ref v)) => {
                        assert!(cs.get("and not result") == Fr::zero());
                        assert_eq!(v.value, Some(false));
                    }
                    (OperandType::AllocatedFalse, OperandType::NegatedAllocatedFalse, Boolean::Is(ref v)) => {
                        assert!(cs.get("and not result") == Fr::zero());
                        assert_eq!(v.value, Some(false));
                    }

                    (OperandType::NegatedAllocatedTrue, OperandType::True, Boolean::Not(_)) => {}
                    (OperandType::NegatedAllocatedTrue, OperandType::False, Boolean::Constant(false)) => {}
                    (OperandType::NegatedAllocatedTrue, OperandType::AllocatedTrue, Boolean::Is(ref v)) => {
                        assert!(cs.get("and not result") == Fr::zero());
                        assert_eq!(v.value, Some(false));
                    }
                    (OperandType::NegatedAllocatedTrue, OperandType::AllocatedFalse, Boolean::Is(ref v)) => {
                        assert!(cs.get("and not result") == Fr::zero());
                        assert_eq!(v.value, Some(false));
                    }
                    (OperandType::NegatedAllocatedTrue, OperandType::NegatedAllocatedTrue, Boolean::Is(ref v)) => {
                        assert!(cs.get("nor result") == Fr::zero());
                        assert_eq!(v.value, Some(false));
                    }
                    (OperandType::NegatedAllocatedTrue, OperandType::NegatedAllocatedFalse, Boolean::Is(ref v)) => {
                        assert!(cs.get("nor result") == Fr::zero());
                        assert_eq!(v.value, Some(false));
                    }

                    (OperandType::NegatedAllocatedFalse, OperandType::True, Boolean::Not(_)) => {}
                    (OperandType::NegatedAllocatedFalse, OperandType::False, Boolean::Constant(false)) => {}
                    (OperandType::NegatedAllocatedFalse, OperandType::AllocatedTrue, Boolean::Is(ref v)) => {
                        assert!(cs.get("and not result") == Fr::one());
                        assert_eq!(v.value, Some(true));
                    }
                    (OperandType::NegatedAllocatedFalse, OperandType::AllocatedFalse, Boolean::Is(ref v)) => {
                        assert!(cs.get("and not result") == Fr::zero());
                        assert_eq!(v.value, Some(false));
                    }
                    (OperandType::NegatedAllocatedFalse, OperandType::NegatedAllocatedTrue, Boolean::Is(ref v)) => {
                        assert!(cs.get("nor result") == Fr::zero());
                        assert_eq!(v.value, Some(false));
                    }
                    (OperandType::NegatedAllocatedFalse, OperandType::NegatedAllocatedFalse, Boolean::Is(ref v)) => {
                        assert!(cs.get("nor result") == Fr::one());
                        assert_eq!(v.value, Some(true));
                    }

                    _ => {
                        panic!("unexpected behavior at {:?} AND {:?}", first_operand, second_operand);
                    }
                }
            }
        }
    }

    #[test]
    fn test_enforce_in_field() {
        {
            let mut cs = TestConstraintSystem::<Fr>::new();

            let mut bits = vec![];
            for (i, b) in BitIterator::new(Fr::characteristic()).skip(1).enumerate() {
                bits.push(Boolean::from(
                    AllocatedBit::alloc(cs.ns(|| format!("bit_gadget {}", i)), || Ok(b)).unwrap(),
                ));
            }

            Boolean::enforce_in_field::<_, _, Fr>(&mut cs, &bits).unwrap();

            assert!(!cs.is_satisfied());
        }

        let mut rng = XorShiftRng::seed_from_u64(1231275789u64);

        for _ in 0..1000 {
            let r = Fr::rand(&mut rng);
            let mut cs = TestConstraintSystem::<Fr>::new();

            let mut bits = vec![];
            for (i, b) in BitIterator::new(r.into_repr()).skip(1).enumerate() {
                bits.push(Boolean::from(
                    AllocatedBit::alloc(cs.ns(|| format!("bit_gadget {}", i)), || Ok(b)).unwrap(),
                ));
            }

            Boolean::enforce_in_field::<_, _, Fr>(&mut cs, &bits).unwrap();

            assert!(cs.is_satisfied());
        }

        // for _ in 0..1000 {
        //     // Sample a random element not in the field
        //     let r = loop {
        //         let mut a = Fr::rand(&mut rng).into_repr();
        //         let b = Fr::rand(&mut rng).into_repr();

        //         a.add_nocarry(&b);
        //         // we're shaving off the high bit_gadget later
        //         a.as_mut()[3] &= 0x7fffffffffffffff;
        //         if Fr::from_repr(a).is_err() {
        //             break a;
        //         }
        //     };

        //     let mut cs = TestConstraintSystem::<Fr>::new();

        //     let mut bits = vec![];
        //     for (i, b) in BitIterator::new(r).skip(1).enumerate() {
        //         bits.push(Boolean::from(
        //             AllocatedBit::alloc(cs.ns(|| format!("bit_gadget {}",
        // i)), Some(b))                 .unwrap(),
        //         ));
        //     }

        //     Boolean::enforce_in_field::<_, _, Fr>(&mut cs, &bits).unwrap();

        //     assert!(!cs.is_satisfied());
        // }
    }

    #[test]
    fn test_enforce_nand() {
        {
            let mut cs = TestConstraintSystem::<Fr>::new();

            assert!(Boolean::enforce_nand(&mut cs, &[Boolean::constant(false)]).is_ok());
            assert!(Boolean::enforce_nand(&mut cs, &[Boolean::constant(true)]).is_err());
        }

        for i in 1..5 {
            // with every possible assignment for them
            for mut b in 0..(1 << i) {
                // with every possible negation
                for mut n in 0..(1 << i) {
                    let mut cs = TestConstraintSystem::<Fr>::new();

                    let mut expected = true;

                    let mut bits = vec![];
                    for j in 0..i {
                        expected &= b & 1 == 1;

                        if n & 1 == 1 {
                            bits.push(Boolean::from(
                                AllocatedBit::alloc(cs.ns(|| format!("bit_gadget {}", j)), || Ok(b & 1 == 1)).unwrap(),
                            ));
                        } else {
                            bits.push(
                                Boolean::from(
                                    AllocatedBit::alloc(cs.ns(|| format!("bit_gadget {}", j)), || Ok(b & 1 == 0))
                                        .unwrap(),
                                )
                                .not(),
                            );
                        }

                        b >>= 1;
                        n >>= 1;
                    }

                    let expected = !expected;

                    Boolean::enforce_nand(&mut cs, &bits).unwrap();

                    if expected {
                        assert!(cs.is_satisfied());
                    } else {
                        assert!(!cs.is_satisfied());
                    }
                }
            }
        }
    }

    #[test]
    fn test_kary_and() {
        // test different numbers of operands
        for i in 1..15 {
            // with every possible assignment for them
            for mut b in 0..(1 << i) {
                let mut cs = TestConstraintSystem::<Fr>::new();

                let mut expected = true;

                let mut bits = vec![];
                for j in 0..i {
                    expected &= b & 1 == 1;

                    bits.push(Boolean::from(
                        AllocatedBit::alloc(cs.ns(|| format!("bit_gadget {}", j)), || Ok(b & 1 == 1)).unwrap(),
                    ));
                    b >>= 1;
                }

                let r = Boolean::kary_and(&mut cs, &bits).unwrap();

                assert!(cs.is_satisfied());

                match r {
                    Boolean::Is(ref r) => {
                        assert_eq!(r.value.unwrap(), expected);
                    }
                    _ => unreachable!(),
                }
            }
        }
    }
}
