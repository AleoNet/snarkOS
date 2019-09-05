use crate::curves::{Field, Group};
use snarkos_utilities::rand::UniformRand;

use rand::SeedableRng;
use rand_xorshift::XorShiftRng;

pub fn group_test<G: Group>(a: G, mut b: G) {
    let mut rng = XorShiftRng::seed_from_u64(1231275789u64);
    let zero = G::zero();
    let fr_zero = G::ScalarField::zero();
    let fr_one = G::ScalarField::one();
    let fr_two = fr_one + &fr_one;
    assert_eq!(zero, zero);
    assert_eq!(zero.is_zero(), true);
    assert_eq!(a.mul(&fr_one), a);
    assert_eq!(a.mul(&fr_two), a + &a);
    assert_eq!(a.mul(&fr_zero), zero);
    assert_eq!(a.mul(&fr_zero) - &a, -a);
    assert_eq!(a.mul(&fr_one) - &a, zero);
    assert_eq!(a.mul(&fr_two) - &a, a);

    // a == a
    assert_eq!(a, a);
    // a + 0 = a
    assert_eq!(a + &zero, a);
    // a - 0 = a
    assert_eq!(a - &zero, a);
    // a - a = 0
    assert_eq!(a - &a, zero);
    // 0 - a = -a
    assert_eq!(zero - &a, -a);
    // a.double() = a + a
    assert_eq!(a.double(), a + &a);
    // b.double() = b + b
    assert_eq!(b.double(), b + &b);
    // a + b = b + a
    assert_eq!(a + &b, b + &a);
    // a - b = -(b - a)
    assert_eq!(a - &b, -(b - &a));
    // (a + b) + a = a + (b + a)
    assert_eq!((a + &b) + &a, a + &(b + &a));
    // (a + b).double() = (a + b) + (b + a)
    assert_eq!((a + &b).double(), (a + &b) + &(b + &a));

    // Check that double_in_place and double give the same result
    let original_b = b;
    b.double_in_place();
    assert_eq!(original_b.double(), b);

    let fr_rand1 = G::ScalarField::rand(&mut rng);
    let fr_rand2 = G::ScalarField::rand(&mut rng);
    let a_rand1 = a.mul(&fr_rand1);
    let a_rand2 = a.mul(&fr_rand2);
    let fr_three = fr_two + &fr_rand1;
    let a_two = a.mul(&fr_two);
    assert_eq!(a_two, a.double(), "(a * 2)  != a.double()");
    let a_six = a.mul(&(fr_three * &fr_two));
    assert_eq!(a_two.mul(&fr_three), a_six, "(a * 2) * 3 != a * (2 * 3)");

    assert_eq!(
        a_rand1.mul(&fr_rand2),
        a_rand2.mul(&fr_rand1),
        "(a * r1) * r2 != (a * r2) * r1"
    );
    assert_eq!(
        a_rand2.mul(&fr_rand1),
        a.mul(&(fr_rand1 * &fr_rand2)),
        "(a * r2) * r1 != a * (r1 * r2)"
    );
    assert_eq!(
        a_rand1.mul(&fr_rand2),
        a.mul(&(fr_rand1 * &fr_rand2)),
        "(a * r1) * r2 != a * (r1 * r2)"
    );
}
