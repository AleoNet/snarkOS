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

use crate::{biginteger::*, rand::UniformRand};

use rand::SeedableRng;
use rand_xorshift::XorShiftRng;

fn biginteger_arithmetic_test<B: BigInteger>(a: B, b: B, zero: B) {
    // zero == zero
    assert_eq!(zero, zero);

    // zero.is_zero() == true
    assert_eq!(zero.is_zero(), true);

    // a == a
    assert_eq!(a, a);

    // a + 0 = a
    let mut a0_add = a.clone();
    a0_add.add_nocarry(&zero);
    assert_eq!(a0_add, a);

    // a - 0 = a
    let mut a0_sub = a.clone();
    a0_sub.sub_noborrow(&zero);
    assert_eq!(a0_sub, a);

    // a - a = 0
    let mut aa_sub = a.clone();
    aa_sub.sub_noborrow(&a);
    assert_eq!(aa_sub, zero);

    // a + b = b + a
    let mut ab_add = a.clone();
    ab_add.add_nocarry(&b);
    let mut ba_add = b.clone();
    ba_add.add_nocarry(&a);
    assert_eq!(ab_add, ba_add);
}

fn biginteger_bits_test<B: BigInteger>() {
    let mut one = B::from(1u64);
    assert!(one.get_bit(0));
    assert!(!one.get_bit(1));
    one.muln(5);

    let thirty_two = one;
    assert!(!thirty_two.get_bit(0));
    assert!(!thirty_two.get_bit(1));
    assert!(!thirty_two.get_bit(2));
    assert!(!thirty_two.get_bit(3));
    assert!(!thirty_two.get_bit(4));
    assert!(thirty_two.get_bit(5), "{:?}", thirty_two);
}

fn biginteger_bytes_test<B: BigInteger>() {
    let mut bytes = [0u8; 256];
    let mut rng = XorShiftRng::seed_from_u64(1231275789u64);
    let x: B = UniformRand::rand(&mut rng);
    x.write(bytes.as_mut()).unwrap();
    let y = B::read(bytes.as_ref()).unwrap();
    assert_eq!(x, y);
}

fn test_biginteger<B: BigInteger>(zero: B) {
    let mut rng = XorShiftRng::seed_from_u64(1231275789u64);
    let a: B = UniformRand::rand(&mut rng);
    let b: B = UniformRand::rand(&mut rng);
    biginteger_arithmetic_test(a, b, zero);
    biginteger_bytes_test::<B>();
    biginteger_bits_test::<B>();
}

#[test]
fn test_biginteger64() {
    test_biginteger(BigInteger64::new([0u64; 1]));
}

#[test]
fn test_biginteger128() {
    test_biginteger(BigInteger128::new([0u64; 2]));
}

#[test]
fn test_biginteger256() {
    test_biginteger(BigInteger256::new([0u64; 4]));
}

#[test]
fn test_biginteger384() {
    test_biginteger(BigInteger384::new([0u64; 6]));
}

#[test]
fn test_biginteger768() {
    test_biginteger(BigInteger768::new([0u64; 12]));
}

#[test]
fn test_biginteger832() {
    test_biginteger(BigInteger832::new([0u64; 13]));
}
