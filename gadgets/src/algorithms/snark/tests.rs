use crate::{algorithms::snark::*, curves::bls12_377::PairingGadget as Bls12_377PairingGadget};
use snarkos_algorithms::snark::{create_random_proof, generate_random_parameters, GM17};
use snarkos_curves::bls12_377::{Bls12_377, Fq, Fr};
use snarkos_models::{
    curves::{Field, PrimeField},
    gadgets::{
        algorithms::snark::SNARKVerifierGadget,
        r1cs::{ConstraintSynthesizer, ConstraintSystem, TestConstraintSystem},
        utilities::{alloc::AllocGadget, boolean::Boolean},
    },
};
use snarkvm_errors::gadgets::SynthesisError;
use snarkvm_utilities::bititerator::BitIterator;

use rand::{thread_rng, Rng};

type TestProofSystem = GM17<Bls12_377, Bench<Fr>, Fr>;
type TestVerifierGadget = GM17VerifierGadget<Bls12_377, Fq, Bls12_377PairingGadget>;
type TestProofGadget = ProofGadget<Bls12_377, Fq, Bls12_377PairingGadget>;
type TestVkGadget = VerifyingKeyGadget<Bls12_377, Fq, Bls12_377PairingGadget>;

struct Bench<F: Field> {
    inputs: Vec<Option<F>>,
    num_constraints: usize,
}

impl<F: Field> ConstraintSynthesizer<F> for Bench<F> {
    fn generate_constraints<CS: ConstraintSystem<F>>(self, cs: &mut CS) -> Result<(), SynthesisError> {
        assert!(self.inputs.len() >= 2);
        assert!(self.num_constraints >= self.inputs.len());

        let mut variables: Vec<_> = Vec::with_capacity(self.inputs.len());
        for (i, input) in self.inputs.into_iter().enumerate() {
            let input_var = cs.alloc_input(
                || format!("Input {}", i),
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
                    || format!("Result {}", i),
                    || result_val.ok_or(SynthesisError::AssignmentMissing),
                )?;
                cs.enforce(
                    || format!("Enforce constraint {}", i),
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

#[test]
fn gm17_verifier_test() {
    let num_inputs = 100;
    let num_constraints = num_inputs;
    let rng = &mut thread_rng();
    let mut inputs: Vec<Option<Fr>> = Vec::with_capacity(num_inputs);
    for _ in 0..num_inputs {
        inputs.push(Some(rng.gen()));
    }
    let params = generate_random_parameters(
        Bench::<Fr> {
            inputs: vec![None; num_inputs],
            num_constraints,
        },
        rng,
    )
    .unwrap();

    {
        let proof = {
            // Create an instance of our circuit (with the witness).
            // Create a gm17 proof with our parameters.
            create_random_proof(
                Bench {
                    inputs: inputs.clone(),
                    num_constraints,
                },
                &params,
                rng,
            )
            .unwrap()
        };

        let mut cs = TestConstraintSystem::<Fq>::new();

        let inputs: Vec<_> = inputs.into_iter().map(|input| input.unwrap()).collect();
        let mut input_gadgets = Vec::new();

        {
            let mut cs = cs.ns(|| "Allocate Input");
            for (i, input) in inputs.into_iter().enumerate() {
                let mut input_bits = BitIterator::new(input.into_repr()).collect::<Vec<_>>();
                // Input must be in little-endian, but BitIterator outputs in big-endian.
                input_bits.reverse();

                let input_bits =
                    Vec::<Boolean>::alloc_input(cs.ns(|| format!("Input {}", i)), || Ok(input_bits)).unwrap();
                input_gadgets.push(input_bits);
            }
        }

        let vk_gadget = TestVkGadget::alloc_input(cs.ns(|| "Vk"), || Ok(&params.vk)).unwrap();
        let proof_gadget = TestProofGadget::alloc(cs.ns(|| "Proof"), || Ok(proof.clone())).unwrap();
        println!("Time to verify!");
        <TestVerifierGadget as SNARKVerifierGadget<TestProofSystem, Fq>>::check_verify(
            cs.ns(|| "Verify"),
            &vk_gadget,
            input_gadgets.iter(),
            &proof_gadget,
        )
        .unwrap();
        if !cs.is_satisfied() {
            println!("=========================================================");
            println!("Unsatisfied constraints:");
            println!("{:?}", cs.which_is_unsatisfied().unwrap());
            println!("=========================================================");
        }

        // cs.print_named_objects();
        assert!(cs.is_satisfied());
    }
}
