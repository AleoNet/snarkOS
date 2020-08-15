macro_rules! uint_impl {
    ($name: ident, $_type: ty, $size: expr) => {
        #[derive(Clone, Debug)]
        pub struct $name {
            pub bits: Vec<Boolean>,
            pub negated: bool,
            pub value: Option<$_type>,
        }

        impl $name {
            pub fn constant(value: $_type) -> Self {
                let mut bits = Vec::with_capacity($size);

                let mut tmp = value;

                for _ in 0..$size {
                    // If last bit is one, push one.
                    if tmp & 1 == 1 {
                        bits.push(Boolean::constant(true))
                    } else {
                        bits.push(Boolean::constant(false))
                    }

                    tmp >>= 1;
                }

                Self {
                    bits,
                    negated: false,
                    value: Some(value),
                }
            }
        }

        impl UInt for $name {
            fn negate(&self) -> Self {
                Self {
                    bits: self.bits.clone(),
                    negated: true,
                    value: self.value.clone(),
                }
            }

            fn is_constant(&self) -> bool {
                let mut constant = true;

                // If any bits of self are allocated bits, return false
                for bit in &self.bits {
                    match *bit {
                        Boolean::Is(ref _bit) => constant = false,
                        Boolean::Not(ref _bit) => constant = false,
                        Boolean::Constant(_bit) => {}
                    }
                }

                constant
            }

            fn to_bits_le(&self) -> Vec<Boolean> {
                self.bits.iter().cloned().collect()
            }

            fn from_bits_le(bits: &[Boolean]) -> Self {
                assert_eq!(bits.len(), $size);

                let bits = bits.to_vec();

                let mut value = Some(0 as $_type);
                for b in bits.iter().rev() {
                    value.as_mut().map(|v| *v <<= 1);

                    match *b {
                        Boolean::Constant(b) => {
                            if b {
                                value.as_mut().map(|v| *v |= 1);
                            }
                        }
                        Boolean::Is(ref b) => match b.get_value() {
                            Some(true) => {
                                value.as_mut().map(|v| *v |= 1);
                            }
                            Some(false) => {}
                            None => value = None,
                        },
                        Boolean::Not(ref b) => match b.get_value() {
                            Some(false) => {
                                value.as_mut().map(|v| *v |= 1);
                            }
                            Some(true) => {}
                            None => value = None,
                        },
                    }
                }

                Self {
                    value,
                    negated: false,
                    bits,
                }
            }

            fn rotr(&self, by: usize) -> Self {
                let by = by % $size;

                let new_bits = self
                    .bits
                    .iter()
                    .skip(by)
                    .chain(self.bits.iter())
                    .take($size)
                    .cloned()
                    .collect();

                Self {
                    bits: new_bits,
                    negated: false,
                    value: self.value.map(|v| v.rotate_right(by as u32) as $_type),
                }
            }

            fn xor<F: Field, CS: ConstraintSystem<F>>(&self, mut cs: CS, other: &Self) -> Result<Self, SynthesisError> {
                let new_value = match (self.value, other.value) {
                    (Some(a), Some(b)) => Some(a ^ b),
                    _ => None,
                };

                let bits = self
                    .bits
                    .iter()
                    .zip(other.bits.iter())
                    .enumerate()
                    .map(|(i, (a, b))| Boolean::xor(cs.ns(|| format!("xor of bit_gadget {}", i)), a, b))
                    .collect::<Result<_, _>>()?;

                Ok(Self {
                    bits,
                    negated: false,
                    value: new_value,
                })
            }

            fn addmany<F: PrimeField, CS: ConstraintSystem<F>>(
                mut cs: CS,
                operands: &[Self],
            ) -> Result<Self, SynthesisError> {
                // Make some arbitrary bounds for ourselves to avoid overflows
                // in the scalar field
                assert!(F::Parameters::MODULUS_BITS >= 128);
                assert!(operands.len() >= 2); // Weird trivial cases that should never happen

                // Compute the maximum value of the sum we allocate enough bits for the result
                let mut max_value = (operands.len() as u128) * u128::from(<$_type>::max_value());

                // Keep track of the resulting value
                let mut result_value = Some(0u128);

                // This is a linear combination that we will enforce to be "zero"
                let mut lc = LinearCombination::zero();

                let mut all_constants = true;

                // Iterate over the operands
                for op in operands {
                    // Accumulate the value
                    match op.value {
                        Some(val) => {
                            // Subtract or add operand
                            if op.negated {
                                // Perform subtraction
                                result_value.as_mut().map(|v| *v -= u128::from(val));
                            } else {
                                // Perform addition
                                result_value.as_mut().map(|v| *v += u128::from(val));
                            }
                        }
                        None => {
                            // If any of our operands have unknown value, we won't
                            // know the value of the result
                            result_value = None;
                        }
                    }

                    // Iterate over each bit_gadget of the operand and add the operand to
                    // the linear combination
                    let mut coeff = F::one();
                    for bit in &op.bits {
                        match *bit {
                            Boolean::Is(ref bit) => {
                                all_constants = false;

                                if op.negated {
                                    // Subtract coeff * bit gadget
                                    lc = lc - (coeff, bit.get_variable());
                                } else {
                                    // Add coeff * bit_gadget
                                    lc = lc + (coeff, bit.get_variable());
                                }
                            }
                            Boolean::Not(ref bit) => {
                                all_constants = false;

                                if op.negated {
                                    // subtract coeff * (1 - bit_gadget) = coeff * ONE - coeff * bit_gadget
                                    lc = lc - (coeff, CS::one()) + (coeff, bit.get_variable());
                                } else {
                                    // Add coeff * (1 - bit_gadget) = coeff * ONE - coeff * bit_gadget
                                    lc = lc + (coeff, CS::one()) - (coeff, bit.get_variable());
                                }
                            }
                            Boolean::Constant(bit) => {
                                if bit {
                                    if op.negated {
                                        lc = lc - (coeff, CS::one());
                                    } else {
                                        lc = lc + (coeff, CS::one());
                                    }
                                }
                            }
                        }

                        coeff.double_in_place();
                    }
                }

                // The value of the actual result is modulo 2 ^ $size
                let modular_value = result_value.map(|v| v as $_type);

                if all_constants && modular_value.is_some() {
                    // We can just return a constant, rather than
                    // unpacking the result into allocated bits.

                    return Ok(Self::constant(modular_value.unwrap()));
                }

                // Storage area for the resulting bits
                let mut result_bits = vec![];

                // Allocate each bit_gadget of the result
                let mut coeff = F::one();
                let mut i = 0;
                while max_value != 0 {
                    // Allocate the bit_gadget
                    let b = AllocatedBit::alloc(cs.ns(|| format!("result bit_gadget {}", i)), || {
                        result_value.map(|v| (v >> i) & 1 == 1).get()
                    })?;

                    // Subtract this bit_gadget from the linear combination to ensure the sums
                    // balance out
                    lc = lc - (coeff, b.get_variable());

                    result_bits.push(b.into());

                    max_value >>= 1;
                    i += 1;
                    coeff.double_in_place();
                }

                // Enforce that the linear combination equals zero
                cs.enforce(|| "modular addition", |lc| lc, |lc| lc, |_| lc);

                // Discard carry bits that we don't care about
                result_bits.truncate($size);

                Ok(Self {
                    bits: result_bits,
                    negated: false,
                    value: modular_value,
                })
            }

            fn sub<F: PrimeField, CS: ConstraintSystem<F>>(
                &self,
                mut cs: CS,
                other: &Self,
            ) -> Result<Self, SynthesisError> {
                // pseudocode:
                //
                // a - b
                // a + (-b)

                Self::addmany(&mut cs.ns(|| "add_not"), &[self.clone(), other.negate()])
            }

            fn sub_unsafe<F: PrimeField, CS: ConstraintSystem<F>>(
                &self,
                mut cs: CS,
                other: &Self,
            ) -> Result<Self, SynthesisError> {
                match (self.value, other.value) {
                    (Some(val1), Some(val2)) => {
                        // Check for overflow
                        if val1 < val2 {
                            // Instead of erroring, return 0

                            if Self::result_is_constant(&self, &other) {
                                // Return constant 0
                                Ok(Self::constant(0 as $_type))
                            } else {
                                // Return allocated 0
                                let result_value = Some(0u128);
                                let modular_value = result_value.map(|v| v as $_type);

                                // Storage area for the resulting bits
                                let mut result_bits = vec![];

                                // This is a linear combination that we will enforce to be "zero"
                                let mut lc = LinearCombination::zero();

                                // Allocate each bit_gadget of the result
                                let mut coeff = F::one();
                                for i in 0..$size {
                                    // Allocate the bit_gadget
                                    let b = AllocatedBit::alloc(cs.ns(|| format!("result bit_gadget {}", i)), || {
                                        result_value.map(|v| (v >> i) & 1 == 1).get()
                                    })?;

                                    // Subtract this bit_gadget from the linear combination to ensure the sums
                                    // balance out
                                    lc = lc - (coeff, b.get_variable());

                                    result_bits.push(b.into());

                                    coeff.double_in_place();
                                }

                                // Enforce that the linear combination equals zero
                                cs.enforce(|| "unsafe subtraction", |lc| lc, |lc| lc, |_| lc);

                                // Discard carry bits that we don't care about
                                result_bits.truncate($size);

                                Ok(Self {
                                    bits: result_bits,
                                    negated: false,
                                    value: modular_value,
                                })
                            }
                        } else {
                            // Perform subtraction
                            self.sub(&mut cs.ns(|| ""), &other)
                        }
                    }
                    (_, _) => {
                        // If either of our operands have unknown value, we won't
                        // know the value of the result
                        return Err(SynthesisError::AssignmentMissing);
                    }
                }
            }

            fn mul<F: PrimeField, CS: ConstraintSystem<F>>(
                &self,
                mut cs: CS,
                other: &Self,
            ) -> Result<Self, SynthesisError> {
                // pseudocode:
                //
                // res = 0;
                // shifted_self = self;
                // for bit in other.bits {
                //   if bit {
                //     res += shifted_self;
                //   }
                //   shifted_self = shifted_self << 1;
                // }
                // return res

                let is_constant = Boolean::constant(Self::result_is_constant(&self, &other));
                let constant_result = Self::constant(0 as $_type);
                let allocated_result = Self::alloc(&mut cs.ns(|| "allocated_1u32"), || Ok(0 as $_type))?;
                let zero_result = Self::conditionally_select(
                    &mut cs.ns(|| "constant_or_allocated"),
                    &is_constant,
                    &constant_result,
                    &allocated_result,
                )?;

                let mut left_shift = self.clone();

                let partial_products = other
                    .bits
                    .iter()
                    .enumerate()
                    .map(|(i, bit)| {
                        let current_left_shift = left_shift.clone();
                        left_shift = Self::addmany(&mut cs.ns(|| format!("shift_left_{}", i)), &[
                            left_shift.clone(),
                            left_shift.clone(),
                        ])
                        .unwrap();

                        Self::conditionally_select(
                            &mut cs.ns(|| format!("calculate_product_{}", i)),
                            &bit,
                            &current_left_shift,
                            &zero_result,
                        )
                        .unwrap()
                    })
                    .collect::<Vec<Self>>();

                Self::addmany(&mut cs.ns(|| format!("partial_products")), &partial_products)
            }

            fn div<F: PrimeField, CS: ConstraintSystem<F>>(
                &self,
                mut cs: CS,
                other: &Self,
            ) -> Result<Self, SynthesisError> {
                // pseudocode:
                //
                // if D = 0 then error(DivisionByZeroException) end
                // Q := 0                  -- Initialize quotient and remainder to zero
                // R := 0
                // for i := n − 1 .. 0 do  -- Where n is number of bits in N
                //   R := R << 1           -- Left-shift R by 1 bit
                //   R(0) := N(i)          -- Set the least-significant bit of R equal to bit i of the numerator
                //   if R ≥ D then
                //     R := R − D
                //     Q(i) := 1
                //   end
                // end

                if other.eq(&Self::constant(0 as $_type)) {
                    return Err(SynthesisError::DivisionByZero);
                }

                let is_constant = Boolean::constant(Self::result_is_constant(&self, &other));

                let allocated_true = Boolean::from(AllocatedBit::alloc(&mut cs.ns(|| "true"), || Ok(true)).unwrap());
                let true_bit = Boolean::conditionally_select(
                    &mut cs.ns(|| "constant_or_allocated_true"),
                    &is_constant,
                    &Boolean::constant(true),
                    &allocated_true,
                )?;

                let allocated_one = Self::alloc(&mut cs.ns(|| "one"), || Ok(1 as $_type))?;
                let one = Self::conditionally_select(
                    &mut cs.ns(|| format!("constant_or_allocated_1u{}", $size)),
                    &is_constant,
                    &Self::constant(1 as $_type),
                    &allocated_one,
                )?;

                let allocated_zero = Self::alloc(&mut cs.ns(|| "zero"), || Ok(0 as $_type))?;
                let zero = Self::conditionally_select(
                    &mut cs.ns(|| format!("constant_or_allocated_0u{}", $size)),
                    &is_constant,
                    &Self::constant(0 as $_type),
                    &allocated_zero,
                )?;

                let self_is_zero = Boolean::Constant(self.eq(&Self::constant(0 as $_type)));
                let mut quotient = zero.clone();
                let mut remainder = zero.clone();

                for (i, bit) in self.bits.iter().rev().enumerate() {
                    // Left shift remainder by 1
                    remainder = Self::addmany(&mut cs.ns(|| format!("shift_left_{}", i)), &[
                        remainder.clone(),
                        remainder.clone(),
                    ])?;

                    // Set the least-significant bit of remainder to bit i of the numerator
                    let bit_is_true = Boolean::constant(bit.eq(&Boolean::constant(true)));
                    let new_remainder = Self::addmany(&mut cs.ns(|| format!("set_remainder_bit_{}", i)), &[
                        remainder.clone(),
                        one.clone(),
                    ])?;

                    remainder = Self::conditionally_select(
                        &mut cs.ns(|| format!("increment_or_remainder_{}", i)),
                        &bit_is_true,
                        &new_remainder,
                        &remainder,
                    )?;

                    // Greater than or equal to:
                    //   R >= D
                    //   (R == D) || (R > D)
                    //   (R == D) || ((R !=D) && ((R - D) != 0))
                    //
                    //  (R > D)                     checks subtraction overflow before evaluation
                    //  (R != D) && ((R - D) != 0)  instead evaluate subtraction and check for overflow after

                    let no_remainder = Boolean::constant(remainder.eq(&other));
                    let subtraction = remainder.sub_unsafe(&mut cs.ns(|| format!("subtract_divisor_{}", i)), &other)?;
                    let sub_is_zero = Boolean::constant(subtraction.eq(&Self::constant(0 as $_type)));
                    let cond1 = Boolean::and(
                        &mut cs.ns(|| format!("cond_1_{}", i)),
                        &no_remainder.not(),
                        &sub_is_zero.not(),
                    )?;
                    let cond2 = Boolean::or(&mut cs.ns(|| format!("cond_2_{}", i)), &no_remainder, &cond1)?;

                    remainder = Self::conditionally_select(
                        &mut cs.ns(|| format!("subtract_or_same_{}", i)),
                        &cond2,
                        &subtraction,
                        &remainder,
                    )?;

                    let index = $size - 1 - i as usize;
                    let bit_value = (1 as $_type) << (index as $_type);
                    let mut new_quotient = quotient.clone();
                    new_quotient.bits[index] = true_bit.clone();
                    new_quotient.value = Some(new_quotient.value.unwrap() + bit_value);

                    quotient = Self::conditionally_select(
                        &mut cs.ns(|| format!("set_bit_or_same_{}", i)),
                        &cond2,
                        &new_quotient,
                        &quotient,
                    )?;
                }
                Self::conditionally_select(&mut cs.ns(|| "self_or_quotient"), &self_is_zero, self, &quotient)
            }

            fn pow<F: Field + PrimeField, CS: ConstraintSystem<F>>(
                &self,
                mut cs: CS,
                other: &Self,
            ) -> Result<Self, SynthesisError> {
                // let mut res = Self::one();
                //
                // let mut found_one = false;
                //
                // for i in BitIterator::new(exp) {
                //     if !found_one {
                //         if i {
                //             found_one = true;
                //         } else {
                //             continue;
                //         }
                //     }
                //
                //     res.square_in_place();
                //
                //     if i {
                //         res *= self;
                //     }
                // }
                // res

                let is_constant = Boolean::constant(Self::result_is_constant(&self, &other));
                let constant_result = Self::constant(1 as $_type);
                let allocated_result = Self::alloc(&mut cs.ns(|| "allocated_1u64"), || Ok(1 as $_type))?;
                let mut result = Self::conditionally_select(
                    &mut cs.ns(|| "constant_or_allocated"),
                    &is_constant,
                    &constant_result,
                    &allocated_result,
                )?;

                for (i, bit) in other.bits.iter().rev().enumerate() {
                    let found_one = Boolean::Constant(result.eq(&Self::constant(1 as $_type)));
                    let cond1 = Boolean::and(cs.ns(|| format!("found_one_{}", i)), &bit.not(), &found_one)?;
                    let square = result.mul(cs.ns(|| format!("square_{}", i)), &result).unwrap();

                    result = Self::conditionally_select(
                        &mut cs.ns(|| format!("result_or_sqaure_{}", i)),
                        &cond1,
                        &result,
                        &square,
                    )?;

                    let mul_by_self = result
                        .mul(cs.ns(|| format!("multiply_by_self_{}", i)), &self)
                        .unwrap();

                    result = Self::conditionally_select(
                        &mut cs.ns(|| format!("mul_by_self_or_result_{}", i)),
                        &bit,
                        &mul_by_self,
                        &result,
                    )?;
                }

                Ok(result)
            }
        }

        impl PartialEq for $name {
            fn eq(&self, other: &Self) -> bool {
                !self.value.is_none() && !other.value.is_none() && self.value == other.value
            }
        }

        impl Eq for $name {}

        impl PartialOrd for $name {
            fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                Option::from(self.value.cmp(&other.value))
            }
        }

        impl<F: Field> EqGadget<F> for $name {}

        impl<F: Field> ConditionalEqGadget<F> for $name {
            fn conditional_enforce_equal<CS: ConstraintSystem<F>>(
                &self,
                mut cs: CS,
                other: &Self,
                condition: &Boolean,
            ) -> Result<(), SynthesisError> {
                for (i, (a, b)) in self.bits.iter().zip(&other.bits).enumerate() {
                    a.conditional_enforce_equal(
                        &mut cs.ns(|| format!("{} equality check for {}-th bit", $size, i)),
                        b,
                        condition,
                    )?;
                }
                Ok(())
            }

            fn cost() -> usize {
                8 * <Boolean as ConditionalEqGadget<F>>::cost()
            }
        }

        impl<F: Field> AllocGadget<$_type, F> for $name {
            fn alloc<Fn: FnOnce() -> Result<T, SynthesisError>, T: Borrow<$_type>, CS: ConstraintSystem<F>>(
                mut cs: CS,
                value_gen: Fn,
            ) -> Result<Self, SynthesisError> {
                let value = value_gen().map(|val| *val.borrow());
                let values = match value {
                    Ok(mut val) => {
                        let mut v = Vec::with_capacity($size);

                        for _ in 0..$size {
                            v.push(Some(val & 1 == 1));
                            val >>= 1;
                        }

                        v
                    }
                    _ => vec![None; $size],
                };

                let bits = values
                    .into_iter()
                    .enumerate()
                    .map(|(i, v)| {
                        Ok(Boolean::from(AllocatedBit::alloc(
                            &mut cs.ns(|| format!("allocated bit_gadget {}", i)),
                            || v.ok_or(SynthesisError::AssignmentMissing),
                        )?))
                    })
                    .collect::<Result<Vec<_>, SynthesisError>>()?;

                Ok(Self {
                    bits,
                    negated: false,
                    value: value.ok(),
                })
            }

            fn alloc_input<Fn: FnOnce() -> Result<T, SynthesisError>, T: Borrow<$_type>, CS: ConstraintSystem<F>>(
                mut cs: CS,
                value_gen: Fn,
            ) -> Result<Self, SynthesisError> {
                let value = value_gen().map(|val| *val.borrow());
                let values = match value {
                    Ok(mut val) => {
                        let mut v = Vec::with_capacity($size);
                        for _ in 0..$size {
                            v.push(Some(val & 1 == 1));
                            val >>= 1;
                        }

                        v
                    }
                    _ => vec![None; $size],
                };

                let bits = values
                    .into_iter()
                    .enumerate()
                    .map(|(i, v)| {
                        Ok(Boolean::from(AllocatedBit::alloc_input(
                            &mut cs.ns(|| format!("allocated bit_gadget {}", i)),
                            || v.ok_or(SynthesisError::AssignmentMissing),
                        )?))
                    })
                    .collect::<Result<Vec<_>, SynthesisError>>()?;

                Ok(Self {
                    bits,
                    negated: false,
                    value: value.ok(),
                })
            }
        }

        impl<F: PrimeField> CondSelectGadget<F> for $name {
            fn conditionally_select<CS: ConstraintSystem<F>>(
                mut cs: CS,
                cond: &Boolean,
                first: &Self,
                second: &Self,
            ) -> Result<Self, SynthesisError> {
                if let Boolean::Constant(cond) = *cond {
                    if cond {
                        Ok(first.clone())
                    } else {
                        Ok(second.clone())
                    }
                } else {
                    let mut is_negated = false;

                    let result_val = cond.get_value().and_then(|c| {
                        if c {
                            is_negated = first.negated;
                            first.value
                        } else {
                            is_negated = second.negated;
                            second.value
                        }
                    });

                    let mut result = Self::alloc(cs.ns(|| "cond_select_result"), || result_val.get().map(|v| v))?;

                    result.negated = is_negated;

                    let expected_bits = first
                        .bits
                        .iter()
                        .zip(&second.bits)
                        .enumerate()
                        .map(|(i, (a, b))| {
                            Boolean::conditionally_select(
                                &mut cs.ns(|| format!("{}_cond_select_{}", $size, i)),
                                cond,
                                a,
                                b,
                            )
                            .unwrap()
                        })
                        .collect::<Vec<Boolean>>();

                    for (i, (actual, expected)) in result.to_bits_le().iter().zip(expected_bits.iter()).enumerate() {
                        actual.enforce_equal(&mut cs.ns(|| format!("selected_result_bit_{}", i)), expected)?;
                    }

                    Ok(result)
                }
            }

            fn cost() -> usize {
                $size * (<Boolean as ConditionalEqGadget<F>>::cost() + <Boolean as CondSelectGadget<F>>::cost())
            }
        }

        impl<F: Field> ToBytesGadget<F> for $name {
            #[inline]
            fn to_bytes<CS: ConstraintSystem<F>>(&self, _cs: CS) -> Result<Vec<UInt8>, SynthesisError> {
                let value_chunks = match self.value.map(|val| {
                    let mut bytes = [0u8; 8];
                    val.write(bytes.as_mut()).unwrap();
                    bytes
                }) {
                    Some(chunks) => [Some(chunks[0]), Some(chunks[1]), Some(chunks[2]), Some(chunks[3])],
                    None => [None, None, None, None],
                };
                let mut bytes = Vec::new();
                for (i, chunk8) in self.to_bits_le().chunks(8).into_iter().enumerate() {
                    let byte = UInt8 {
                        bits: chunk8.to_vec(),
                        negated: false,
                        value: value_chunks[i],
                    };
                    bytes.push(byte);
                }

                Ok(bytes)
            }

            fn to_bytes_strict<CS: ConstraintSystem<F>>(&self, cs: CS) -> Result<Vec<UInt8>, SynthesisError> {
                self.to_bytes(cs)
            }
        }
    };
}
