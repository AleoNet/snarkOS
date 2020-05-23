use snarkos_models::{
    curves::Field,
    gadgets::r1cs::{ConstraintSynthesizer, ConstraintSystem, LinearCombination},
};
use snarkvm_errors::gadgets::SynthesisError;

use std::marker::PhantomData;

pub struct Benchmark<F: Field> {
    num_constraints: usize,
    _engine: PhantomData<F>,
}

impl<F: Field> Benchmark<F> {
    pub fn new(num_constraints: usize) -> Self {
        Self {
            num_constraints,
            _engine: PhantomData,
        }
    }
}

impl<F: Field> ConstraintSynthesizer<F> for Benchmark<F> {
    fn generate_constraints<CS: ConstraintSystem<F>>(self, cs: &mut CS) -> Result<(), SynthesisError> {
        let mut assignments = Vec::new();

        let mut a_val = F::one();
        let mut a_var = cs.alloc_input(|| "a", || Ok(a_val))?;
        assignments.push((a_val, a_var));

        let mut b_val = F::one();
        let mut b_var = cs.alloc_input(|| "b", || Ok(b_val))?;
        assignments.push((a_val, a_var));

        for i in 0..self.num_constraints - 1 {
            if i % 2 != 0 {
                let c_val = a_val * &b_val;
                let c_var = cs.alloc(|| format!("{}", i), || Ok(c_val))?;

                cs.enforce(
                    || format!("{}: a * b = c", i),
                    |lc| lc + a_var,
                    |lc| lc + b_var,
                    |lc| lc + c_var,
                );

                assignments.push((c_val, c_var));
                a_val = b_val;
                a_var = b_var;
                b_val = c_val;
                b_var = c_var;
            } else {
                let c_val = a_val + &b_val;
                let c_var = cs.alloc(|| format!("{}", i), || Ok(c_val))?;

                cs.enforce(
                    || format!("{}: a + b = c", i),
                    |lc| lc + a_var + b_var,
                    |lc| lc + CS::one(),
                    |lc| lc + c_var,
                );

                assignments.push((c_val, c_var));
                a_val = b_val;
                a_var = b_var;
                b_val = c_val;
                b_var = c_var;
            }
        }

        let mut a_lc = LinearCombination::zero();
        let mut b_lc = LinearCombination::zero();
        let mut c_val = F::zero();

        for (val, var) in assignments {
            a_lc = a_lc + var;
            b_lc = b_lc + var;
            c_val = c_val + &val;
        }
        c_val = c_val.square();

        let c_var = cs.alloc(|| "c_val", || Ok(c_val))?;

        cs.enforce(|| "assignments.sum().square()", |_| a_lc, |_| b_lc, |lc| lc + c_var);

        Ok(())
    }
}
