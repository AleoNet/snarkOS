use snarkos_algorithms::snark::{Proof, VerifyingKey, GM17};
use snarkos_models::{
    curves::{to_field_vec::ToConstraintField, AffineCurve, Field, PairingEngine},
    gadgets::{
        algorithms::SNARKVerifierGadget,
        curves::{FieldGadget, GroupGadget, PairingGadget},
        r1cs::{ConstraintSynthesizer, ConstraintSystem, SynthesisError},
        utilities::{alloc::AllocGadget, eq::EqGadget, uint8::UInt8, ToBitsGadget, ToBytesGadget},
    },
};

use std::{borrow::Borrow, marker::PhantomData};

#[derive(Derivative)]
#[derivative(Clone(bound = "P::G1Gadget: Clone, P::G2Gadget: Clone"))]
pub struct ProofGadget<PairingE: PairingEngine, F: Field, P: PairingGadget<PairingE, F>> {
    pub a: P::G1Gadget,
    pub b: P::G2Gadget,
    pub c: P::G1Gadget,
}

#[derive(Derivative)]
#[derivative(Clone(bound = "P::G1Gadget: Clone, P::GTGadget: Clone, P::G1PreparedGadget: Clone, \
                            P::G2PreparedGadget: Clone, "))]
pub struct VerifyingKeyGadget<Pairing: PairingEngine, F: Field, P: PairingGadget<Pairing, F>> {
    pub h_g2: P::G2Gadget,
    pub g_alpha_g1: P::G1Gadget,
    pub h_beta_g2: P::G2Gadget,
    pub g_gamma_g1: P::G1Gadget,
    pub h_gamma_g2: P::G2Gadget,
    pub query: Vec<P::G1Gadget>,
}

impl<Pairing: PairingEngine, F: Field, P: PairingGadget<Pairing, F>> VerifyingKeyGadget<Pairing, F, P> {
    pub fn prepare<CS: ConstraintSystem<F>>(
        &self,
        mut cs: CS,
    ) -> Result<PreparedVerifyingKeyGadget<Pairing, F, P>, SynthesisError> {
        let mut cs = cs.ns(|| "Preparing verifying key");
        let g_alpha_pc = P::prepare_g1(&mut cs.ns(|| "Prepare g_alpha_g1"), &self.g_alpha_g1)?;
        let h_beta_pc = P::prepare_g2(&mut cs.ns(|| "Prepare h_beta_g2"), &self.h_beta_g2)?;
        let g_gamma_pc = P::prepare_g1(&mut cs.ns(|| "Prepare g_gamma_pc"), &self.g_gamma_g1)?;
        let h_gamma_pc = P::prepare_g2(&mut cs.ns(|| "Prepare h_gamma_pc"), &self.h_gamma_g2)?;
        let h_pc = P::prepare_g2(&mut cs.ns(|| "Prepare h_pc"), &self.h_g2)?;
        Ok(PreparedVerifyingKeyGadget {
            g_alpha: self.g_alpha_g1.clone(),
            h_beta: self.h_beta_g2.clone(),
            g_alpha_pc,
            h_beta_pc,
            g_gamma_pc,
            h_gamma_pc,
            h_pc,
            query: self.query.clone(),
        })
    }
}

#[derive(Derivative)]
#[derivative(Clone(bound = "P::G1Gadget: Clone, P::GTGadget: Clone, P::G1PreparedGadget: Clone, \
                            P::G2PreparedGadget: Clone, "))]
pub struct PreparedVerifyingKeyGadget<Pairing: PairingEngine, F: Field, P: PairingGadget<Pairing, F>> {
    pub g_alpha: P::G1Gadget,
    pub h_beta: P::G2Gadget,
    pub g_alpha_pc: P::G1PreparedGadget,
    pub h_beta_pc: P::G2PreparedGadget,
    pub g_gamma_pc: P::G1PreparedGadget,
    pub h_gamma_pc: P::G2PreparedGadget,
    pub h_pc: P::G2PreparedGadget,
    pub query: Vec<P::G1Gadget>,
}

pub struct Gm17VerifierGadget<Pairing: PairingEngine, F: Field, P: PairingGadget<Pairing, F>> {
    _pairing_engine: PhantomData<Pairing>,
    _engine: PhantomData<F>,
    _pairing_gadget: PhantomData<P>,
}

impl<
        Pairing: PairingEngine,
        F: Field,
        P: PairingGadget<Pairing, F>,
        C: ConstraintSynthesizer<Pairing::Fr>,
        V: ToConstraintField<Pairing::Fr>,
    > SNARKVerifierGadget<GM17<Pairing, C, V>, F> for Gm17VerifierGadget<Pairing, F, P>
{
    type ProofGadget = ProofGadget<Pairing, F, P>;
    type VerificationKeyGadget = VerifyingKeyGadget<Pairing, F, P>;

    fn check_verify<'a, CS: ConstraintSystem<F>, I: Iterator<Item = &'a T>, T: 'a + ToBitsGadget<F> + ?Sized>(
        mut cs: CS,
        vk: &Self::VerificationKeyGadget,
        mut public_inputs: I,
        proof: &Self::ProofGadget,
    ) -> Result<(), SynthesisError> {
        let pvk = vk.prepare(&mut cs.ns(|| "Prepare vk"))?;
        // e(A*G^{alpha}, B*H^{beta}) = e(G^{alpha}, H^{beta}) * e(G^{psi}, H^{gamma}) *
        // e(C, H) where psi = \sum_{i=0}^l input_i pvk.query[i]

        let g_psi = {
            let mut cs = cs.ns(|| "Process input");
            let mut g_psi = pvk.query[0].clone();
            let mut input_len = 1;
            for (i, (input, b)) in public_inputs.by_ref().zip(pvk.query.iter().skip(1)).enumerate() {
                let input_bits = input.to_bits(cs.ns(|| format!("Input {}", i)))?;
                g_psi = b.mul_bits(cs.ns(|| format!("Mul {}", i)), &g_psi, input_bits.iter())?;
                input_len += 1;
            }
            // Check that the input and the query in the verification are of the
            // same length.
            assert!(input_len == pvk.query.len() && public_inputs.next().is_none());
            g_psi
        };

        let mut test1_a_g_alpha = proof.a.add(cs.ns(|| "A * G^{alpha}"), &pvk.g_alpha)?;
        let test1_b_h_beta = proof.b.add(cs.ns(|| "B * H^{beta}"), &pvk.h_beta)?;

        let test1_exp = {
            test1_a_g_alpha = test1_a_g_alpha.negate(cs.ns(|| "neg 1"))?;
            let test1_a_g_alpha_prep = P::prepare_g1(cs.ns(|| "First prep"), &test1_a_g_alpha)?;
            let test1_b_h_beta_prep = P::prepare_g2(cs.ns(|| "Second prep"), &test1_b_h_beta)?;

            let g_psi_prep = P::prepare_g1(cs.ns(|| "Third prep"), &g_psi)?;

            let c_prep = P::prepare_g1(cs.ns(|| "Fourth prep"), &proof.c)?;

            P::miller_loop(
                cs.ns(|| "Miller loop 1"),
                &[test1_a_g_alpha_prep, g_psi_prep, c_prep, pvk.g_alpha_pc.clone()],
                &[
                    test1_b_h_beta_prep,
                    pvk.h_gamma_pc.clone(),
                    pvk.h_pc.clone(),
                    pvk.h_beta_pc.clone(),
                ],
            )?
        };

        let test1 = P::final_exponentiation(cs.ns(|| "Final Exp 1"), &test1_exp).unwrap();

        // e(A, H^{gamma}) = e(G^{gamma}, B)
        let test2_exp = {
            let a_prep = P::prepare_g1(cs.ns(|| "Fifth prep"), &proof.a)?;
            // pvk.h_gamma_pc
            //&pvk.g_gamma_pc
            let proof_b = proof.b.negate(cs.ns(|| "Negate b"))?;
            let b_prep = P::prepare_g2(cs.ns(|| "Sixth prep"), &proof_b)?;
            P::miller_loop(cs.ns(|| "Miller loop 4"), &[a_prep, pvk.g_gamma_pc.clone()], &[
                pvk.h_gamma_pc,
                b_prep,
            ])?
        };
        let test2 = P::final_exponentiation(cs.ns(|| "Final Exp 2"), &test2_exp)?;

        let one = P::GTGadget::one(cs.ns(|| "GT One"))?;
        test1.enforce_equal(cs.ns(|| "Test 1"), &one)?;
        test2.enforce_equal(cs.ns(|| "Test 2"), &one)?;
        Ok(())
    }
}

impl<Pairing: PairingEngine, F: Field, P: PairingGadget<Pairing, F>> AllocGadget<VerifyingKey<Pairing>, F>
    for VerifyingKeyGadget<Pairing, F, P>
{
    #[inline]
    fn alloc<Fn: FnOnce() -> Result<T, SynthesisError>, T: Borrow<VerifyingKey<Pairing>>, CS: ConstraintSystem<F>>(
        mut cs: CS,
        value_gen: Fn,
    ) -> Result<Self, SynthesisError> {
        value_gen().and_then(|vk| {
            let VerifyingKey {
                h_g2,
                g_alpha_g1,
                h_beta_g2,
                g_gamma_g1,
                h_gamma_g2,
                query,
            } = vk.borrow().clone();
            let h_g2 = P::G2Gadget::alloc(cs.ns(|| "h_g2"), || Ok(h_g2.into_projective()))?;
            let g_alpha_g1 = P::G1Gadget::alloc(cs.ns(|| "g_alpha"), || Ok(g_alpha_g1.into_projective()))?;
            let h_beta_g2 = P::G2Gadget::alloc(cs.ns(|| "h_beta"), || Ok(h_beta_g2.into_projective()))?;
            let g_gamma_g1 = P::G1Gadget::alloc(cs.ns(|| "g_gamma_g1"), || Ok(g_gamma_g1.into_projective()))?;
            let h_gamma_g2 = P::G2Gadget::alloc(cs.ns(|| "h_gamma_g2"), || Ok(h_gamma_g2.into_projective()))?;

            let query = query
                .into_iter()
                .enumerate()
                .map(|(i, query_i)| {
                    P::G1Gadget::alloc(cs.ns(|| format!("query_{}", i)), || Ok(query_i.into_projective()))
                })
                .collect::<Vec<_>>()
                .into_iter()
                .collect::<Result<_, _>>()?;
            Ok(Self {
                h_g2,
                g_alpha_g1,
                h_beta_g2,
                g_gamma_g1,
                h_gamma_g2,
                query,
            })
        })
    }

    #[inline]
    fn alloc_input<
        Fn: FnOnce() -> Result<T, SynthesisError>,
        T: Borrow<VerifyingKey<Pairing>>,
        CS: ConstraintSystem<F>,
    >(
        mut cs: CS,
        value_gen: Fn,
    ) -> Result<Self, SynthesisError> {
        value_gen().and_then(|vk| {
            let VerifyingKey {
                h_g2,
                g_alpha_g1,
                h_beta_g2,
                g_gamma_g1,
                h_gamma_g2,
                query,
            } = vk.borrow().clone();
            let h_g2 = P::G2Gadget::alloc_input(cs.ns(|| "h_g2"), || Ok(h_g2.into_projective()))?;
            let g_alpha_g1 = P::G1Gadget::alloc_input(cs.ns(|| "g_alpha"), || Ok(g_alpha_g1.into_projective()))?;
            let h_beta_g2 = P::G2Gadget::alloc_input(cs.ns(|| "h_beta"), || Ok(h_beta_g2.into_projective()))?;
            let g_gamma_g1 = P::G1Gadget::alloc_input(cs.ns(|| "g_gamma_g1"), || Ok(g_gamma_g1.into_projective()))?;
            let h_gamma_g2 = P::G2Gadget::alloc_input(cs.ns(|| "h_gamma_g2"), || Ok(h_gamma_g2.into_projective()))?;

            let query = query
                .into_iter()
                .enumerate()
                .map(|(i, query_i)| {
                    P::G1Gadget::alloc_input(cs.ns(|| format!("query_{}", i)), || Ok(query_i.into_projective()))
                })
                .collect::<Vec<_>>()
                .into_iter()
                .collect::<Result<_, _>>()?;
            Ok(Self {
                h_g2,
                g_alpha_g1,
                h_beta_g2,
                g_gamma_g1,
                h_gamma_g2,
                query,
            })
        })
    }
}

impl<Pairing: PairingEngine, F: Field, P: PairingGadget<Pairing, F>> AllocGadget<Proof<Pairing>, F>
    for ProofGadget<Pairing, F, P>
{
    #[inline]
    fn alloc<Fn: FnOnce() -> Result<T, SynthesisError>, T: Borrow<Proof<Pairing>>, CS: ConstraintSystem<F>>(
        mut cs: CS,
        value_gen: Fn,
    ) -> Result<Self, SynthesisError> {
        value_gen().and_then(|proof| {
            let Proof { a, b, c } = proof.borrow().clone();
            let a = P::G1Gadget::alloc_checked(cs.ns(|| "a"), || Ok(a.into_projective()))?;
            let b = P::G2Gadget::alloc_checked(cs.ns(|| "b"), || Ok(b.into_projective()))?;
            let c = P::G1Gadget::alloc_checked(cs.ns(|| "c"), || Ok(c.into_projective()))?;
            Ok(Self { a, b, c })
        })
    }

    #[inline]
    fn alloc_input<Fn: FnOnce() -> Result<T, SynthesisError>, T: Borrow<Proof<Pairing>>, CS: ConstraintSystem<F>>(
        mut cs: CS,
        value_gen: Fn,
    ) -> Result<Self, SynthesisError> {
        value_gen().and_then(|proof| {
            let Proof { a, b, c } = proof.borrow().clone();
            // We don't need to check here because the prime order check can be performed
            // in plain.
            let a = P::G1Gadget::alloc_input(cs.ns(|| "a"), || Ok(a.into_projective()))?;
            let b = P::G2Gadget::alloc_input(cs.ns(|| "b"), || Ok(b.into_projective()))?;
            let c = P::G1Gadget::alloc_input(cs.ns(|| "c"), || Ok(c.into_projective()))?;
            Ok(Self { a, b, c })
        })
    }
}

impl<Pairing: PairingEngine, F: Field, P: PairingGadget<Pairing, F>> ToBytesGadget<F>
    for VerifyingKeyGadget<Pairing, F, P>
{
    #[inline]
    fn to_bytes<CS: ConstraintSystem<F>>(&self, mut cs: CS) -> Result<Vec<UInt8>, SynthesisError> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.h_g2.to_bytes(&mut cs.ns(|| "h_g2 to bytes"))?);
        bytes.extend_from_slice(&self.g_alpha_g1.to_bytes(&mut cs.ns(|| "g_alpha_g1 to bytes"))?);
        bytes.extend_from_slice(&self.h_beta_g2.to_bytes(&mut cs.ns(|| "h_beta_g2 to bytes"))?);
        bytes.extend_from_slice(&self.g_gamma_g1.to_bytes(&mut cs.ns(|| "g_gamma_g1 to bytes"))?);
        bytes.extend_from_slice(&self.h_gamma_g2.to_bytes(&mut cs.ns(|| "h_gamma_g2 to bytes"))?);
        for (i, q) in self.query.iter().enumerate() {
            let mut cs = cs.ns(|| format!("Iteration {}", i));
            bytes.extend_from_slice(&q.to_bytes(&mut cs.ns(|| "q"))?);
        }
        Ok(bytes)
    }

    fn to_bytes_strict<CS: ConstraintSystem<F>>(&self, cs: CS) -> Result<Vec<UInt8>, SynthesisError> {
        self.to_bytes(cs)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::curves::bls12_377::PairingGadget as Bls12_377PairingGadget;
    use snarkos_algorithms::snark::{create_random_proof, generate_random_parameters, GM17};
    use snarkos_curves::bls12_377::{Bls12_377, Fq, Fr};
    use snarkos_models::{
        curves::PrimeField,
        gadgets::{
            r1cs::{ConstraintSynthesizer, ConstraintSystem, SynthesisError, TestConstraintSystem},
            utilities::boolean::Boolean,
        },
    };
    use snarkos_utilities::bititerator::BitIterator;

    use rand::{thread_rng, Rng};

    type TestProofSystem = GM17<Bls12_377, Bench<Fr>, Fr>;
    type TestVerifierGadget = Gm17VerifierGadget<Bls12_377, Fq, Bls12_377PairingGadget>;
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
}
