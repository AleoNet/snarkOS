pub mod uint128;
pub mod uint16;
pub mod uint32;
pub mod uint64;
pub mod uint8;

mod uint_macro {
    // use crate::gadgets::utilities::{
    //     uint::{UInt, UInt8, UInt16, UInt32, UInt64}
    // };
    // use crate::gadgets::r1cs::{TestConstraintSystem, Fr, ConstraintSystem};
    // use rand::{Rng, SeedableRng};
    // use rand_xorshift::XorShiftRng;
    // use crate::gadgets::utilities::boolean::Boolean;

    // macro_rules! addmany_constants {
    //     ($_type: ty, $uint: ty) => {
    //         for _ in 0..1000 {
    //             let mut cs = TestConstraintSystem::<Fr>::new();
    //
    //             let mut rng = XorShiftRng::seed_from_u64(1231275789u64);
    //
    //             let a = rng.gen::<$_type>();
    //             let b = rng.gen::<$_type>();
    //             let c = rng.gen::<$_type>();
    //
    //             let a_bit = <$uint>::constant(a);
    //             let b_bit = <$uint>::constant(b);
    //             let c_bit = <$uint>::constant(b);
    //
    //             let mut expected = a.wrapping_add(b).wrapping_add(c);
    //
    //             let r = <$uint>::addmany(cs.ns(|| "addition"), &[a_bit, b_bit, c_bit]).unwrap();
    //
    //             // check value
    //             assert!(r.value == Some(expected));
    //
    //             // check bits
    //             for b in r.bits.iter() {
    //                 match b {
    //                     &Boolean::Is(_) => panic!(),
    //                     &Boolean::Not(_) => panic!(),
    //                     &Boolean::Constant(b) => {
    //                         assert!(b == ( expected & 1 == 1));
    //                     }
    //                 }
    //
    //                 expected >>= 1;
    //             }
    //         }
    //     }
    // }

    // fn check_all_constant_bits(mut expected: u8, actual: UInt8) {
    //     for b in actual.bits.iter() {
    //         match b {
    //             &Boolean::Is(_) => panic!(),
    //             &Boolean::Not(_) => panic!(),
    //             &Boolean::Constant(b) => {
    //                 assert!(b == (expected & 1 == 1));
    //             }
    //         }
    //
    //         expected >>= 1;
    //     }
    // }
    //
    // fn addmany_constants<I: UInt>(a: I, b: I) {
    //

    // }

    // fn test_uint_constants<I: UInt>(a: I, b: I) {
    //
    //     // addmany_constants(a, b)
    // }
    // fn check_all_constant_bits(mut expected: u8, actual: UInt8) {
    //     for b in actual.bits.iter() {
    //         match b {
    //             &Boolean::Is(_) => panic!(),
    //             &Boolean::Not(_) => panic!(),
    //             &Boolean::Constant(b) => {
    //                 assert!(b == (expected & 1 == 1));
    //             }
    //         }
    //
    //         expected >>= 1;
    //     }
    // }

    // #[test]
    // fn test_uint8() {
    //
    // }
    //
    // #[test]
    // fn test_uint16() {
    //     let mut rng = XorShiftRng::seed_from_u64(1231275789u64);
    //     let a: u16 = rng.gen();
    //     let b: u16 = rng.gen();
    //
    //     let a_bit = UInt16::constant(a);
    //     let b_bit = UInt16::constant(b);
    //
    //     test_uint_constants(a_bit, b_bit);
    // }
    //
    // #[test]
    // fn test_uint32() {
    //     let mut rng = XorShiftRng::seed_from_u64(1231275789u64);
    //     let a: u32 = rng.gen();
    //     let b: u32 = rng.gen();
    //
    //     let a_bit = UInt32::constant(a);
    //     let b_bit = UInt32::constant(b);
    //
    //     test_uint_constants(a_bit, b_bit);
    // }
    //
    // #[test]
    // fn test_uint64() {
    //     let mut rng = XorShiftRng::seed_from_u64(1231275789u64);
    //     let a: u64 = rng.gen();
    //     let b: u64 = rng.gen();
    //
    //     let a_bit = UInt64::constant(a);
    //     let b_bit = UInt64::constant(b);
    //
    //     test_uint_constants(a_bit, b_bit);
    // }

    // #[test]
    // fn test_uint128() {
    //     test_uint(UInt128::constant(0u128))
    // }
}
