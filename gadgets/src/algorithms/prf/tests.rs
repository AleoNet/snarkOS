use crate::algorithms::prf::*;
use snarkos_algorithms::prf::blake2s::Blake2s as B2SPRF;
use snarkos_curves::bls12_377::Fr;
use snarkos_models::{
    algorithms::PRF,
    gadgets::{algorithms::PRFGadget, r1cs::{ConstraintSystem, TestConstraintSystem}, utilities::{alloc::AllocGadget, boolean::{AllocatedBit, Boolean}, eq::EqGadget, uint8::UInt8}},
};

use blake2::Blake2s;
use digest::{FixedOutput, Input};
use rand::{Rng, SeedableRng};
use rand_xorshift::XorShiftRng;

#[test]
fn test_blake2s_constraints() {
    let mut cs = TestConstraintSystem::<Fr>::new();
    let input_bits: Vec<_> = (0..512)
        .map(|i| {
            AllocatedBit::alloc(cs.ns(|| format!("input bit_gadget {}", i)), || Ok(true))
                .unwrap()
                .into()
        })
        .collect();
    blake2s_gadget(&mut cs, &input_bits).unwrap();
    assert!(cs.is_satisfied());
    assert_eq!(cs.num_constraints(), 21792);
}

#[test]
fn test_blake2s_prf() {
    let mut rng = XorShiftRng::seed_from_u64(1231275789u64);
    let mut cs = TestConstraintSystem::<Fr>::new();

    let mut seed = [0u8; 32];
    rng.fill(&mut seed);

    let mut input = [0u8; 32];
    rng.fill(&mut input);

    let seed_gadget = Blake2sGadget::new_seed(&mut cs.ns(|| "declare_seed"), &seed);
    let input_gadget = UInt8::alloc_vec(&mut cs.ns(|| "declare_input"), &input).unwrap();
    let out = B2SPRF::evaluate(&seed, &input).unwrap();
    let actual_out_gadget =
        <Blake2sGadget as PRFGadget<_, Fr>>::OutputGadget::alloc(&mut cs.ns(|| "declare_output"), || Ok(out))
            .unwrap();

    let output_gadget =
        Blake2sGadget::check_evaluation_gadget(&mut cs.ns(|| "eval_blake2s"), &seed_gadget, &input_gadget).unwrap();
    output_gadget.enforce_equal(&mut cs, &actual_out_gadget).unwrap();

    if !cs.is_satisfied() {
        println!("which is unsatisfied: {:?}", cs.which_is_unsatisfied().unwrap());
    }
    assert!(cs.is_satisfied());
}

#[test]
fn test_blake2s_precomp_constraints() {
    // Test that 512 fixed leading bits (constants)
    // doesn't result in more constraints.

    let mut cs = TestConstraintSystem::<Fr>::new();
    let mut rng = XorShiftRng::seed_from_u64(1231275789u64);
    let input_bits: Vec<_> = (0..512)
        .map(|_| Boolean::constant(rng.gen()))
        .chain((0..512).map(|i| {
            AllocatedBit::alloc(cs.ns(|| format!("input bit_gadget {}", i)), || Ok(true))
                .unwrap()
                .into()
        }))
        .collect();
    blake2s_gadget(&mut cs, &input_bits).unwrap();
    assert!(cs.is_satisfied());
    assert_eq!(cs.num_constraints(), 21792);
}

#[test]
fn test_blake2s_constant_constraints() {
    let mut cs = TestConstraintSystem::<Fr>::new();
    let mut rng = XorShiftRng::seed_from_u64(1231275789u64);
    let input_bits: Vec<_> = (0..512).map(|_| Boolean::constant(rng.gen())).collect();
    blake2s_gadget(&mut cs, &input_bits).unwrap();
    assert_eq!(cs.num_constraints(), 0);
}

#[test]
fn test_blake2s() {
    let mut rng = XorShiftRng::seed_from_u64(1231275789u64);

    for input_len in (0..32).chain((32..256).filter(|a| a % 8 == 0)) {
        let mut h = Blake2s::new_keyed(&[], 32);

        let data: Vec<u8> = (0..input_len).map(|_| rng.gen()).collect();

        h.process(&data);

        let hash_result = h.fixed_result();

        let mut cs = TestConstraintSystem::<Fr>::new();

        let mut input_bits = vec![];

        for (byte_i, input_byte) in data.into_iter().enumerate() {
            for bit_i in 0..8 {
                let cs = cs.ns(|| format!("input bit_gadget {} {}", byte_i, bit_i));

                input_bits.push(
                    AllocatedBit::alloc(cs, || Ok((input_byte >> bit_i) & 1u8 == 1u8))
                        .unwrap()
                        .into(),
                );
            }
        }

        let r = blake2s_gadget(&mut cs, &input_bits).unwrap();

        assert!(cs.is_satisfied());

        let mut s = hash_result
            .as_ref()
            .iter()
            .flat_map(|&byte| (0..8).map(move |i| (byte >> i) & 1u8 == 1u8));

        for chunk in r {
            for b in chunk.to_bits_le() {
                match b {
                    Boolean::Is(b) => {
                        assert!(s.next().unwrap() == b.get_value().unwrap());
                    }
                    Boolean::Not(b) => {
                        assert!(s.next().unwrap() != b.get_value().unwrap());
                    }
                    Boolean::Constant(b) => {
                        assert!(input_len == 0);
                        assert!(s.next().unwrap() == b);
                    }
                }
            }
        }
    }
}
