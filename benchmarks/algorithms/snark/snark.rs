// Copyright (C) 2019-2020 Aleo Systems Inc.
// This file is part of the snarkOS library.

// The snarkOS library is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// The snarkOS library is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with the snarkOS library. If not, see <https://www.gnu.org/licenses/>.

#[macro_use]
extern crate criterion;

use snarkos_algorithms::snark::GM17;
use snarkos_curves::bls12_377::{Bls12_377, Fr};
use snarkos_errors::gadgets::SynthesisError;
use snarkos_models::{
    algorithms::SNARK,
    curves::Field,
    gadgets::r1cs::{ConstraintSynthesizer, ConstraintSystem},
};

use criterion::Criterion;
use rand::{self, thread_rng, Rng};

type GM17SNARK = GM17<Bls12_377, Benchmark<Fr>, Fr>;

struct Benchmark<F: Field> {
    inputs: Vec<Option<F>>,
    num_constraints: usize,
}

impl<F: Field> ConstraintSynthesizer<F> for Benchmark<F> {
    fn generate_constraints<CS: ConstraintSystem<F>>(self, cs: &mut CS) -> Result<(), SynthesisError> {
        assert!(self.inputs.len() >= 2);
        assert!(self.num_constraints >= self.inputs.len());

        let mut variables: Vec<_> = Vec::with_capacity(self.inputs.len());
        for (i, input) in self.inputs.into_iter().enumerate() {
            let input_var = cs.alloc_input(
                || format!("input_{}", i),
                || input.ok_or(SynthesisError::AssignmentMissing),
            )?;
            variables.push((input, input_var));
        }

        for i in 0..self.num_constraints {
            let new_entry = {
                let (input_1_val, input_1_var) = variables[i];
                let (input_2_val, input_2_var) = variables[i + 1];
                let result_val = input_1_val.and_then(|input_1| input_2_val.map(|input_2| input_1 * &input_2));
                let result_var = cs.alloc(
                    || format!("result_{}", i),
                    || result_val.ok_or(SynthesisError::AssignmentMissing),
                )?;
                cs.enforce(
                    || format!("enforce_constraint_{}", i),
                    |lc| lc + input_1_var,
                    |lc| lc + input_2_var,
                    |lc| lc + result_var,
                );
                (result_val, result_var)
            };
            variables.push(new_entry);
        }
        Ok(())
    }
}

fn snark_setup(c: &mut Criterion) {
    let num_inputs = 100;
    let num_constraints = num_inputs;
    let rng = &mut thread_rng();
    let mut inputs: Vec<Option<Fr>> = Vec::with_capacity(num_inputs);
    for _ in 0..num_inputs {
        inputs.push(Some(rng.gen()));
    }

    c.bench_function("snark_setup", move |b| {
        b.iter(|| {
            GM17SNARK::setup(
                Benchmark::<Fr> {
                    inputs: vec![None; num_inputs],
                    num_constraints,
                },
                rng,
            )
            .unwrap()
        })
    });
}

fn snark_prove(c: &mut Criterion) {
    let num_inputs = 100;
    let num_constraints = num_inputs;
    let rng = &mut thread_rng();
    let mut inputs: Vec<Option<Fr>> = Vec::with_capacity(num_inputs);
    for _ in 0..num_inputs {
        inputs.push(Some(rng.gen()));
    }

    let params = GM17SNARK::setup(
        Benchmark::<Fr> {
            inputs: vec![None; num_inputs],
            num_constraints,
        },
        rng,
    )
    .unwrap();

    c.bench_function("snark_prove", move |b| {
        b.iter(|| {
            GM17SNARK::prove(
                &params.0,
                Benchmark {
                    inputs: inputs.clone(),
                    num_constraints,
                },
                rng,
            )
            .unwrap()
        })
    });
}

criterion_group! {
    name = gm17_snark;
    config = Criterion::default().sample_size(10);
    targets = snark_setup, snark_prove
}

criterion_main!(gm17_snark);
