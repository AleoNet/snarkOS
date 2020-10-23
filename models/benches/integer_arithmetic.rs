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

use snarkos_models::gadgets::{
    r1cs::{ConstraintSystem, Fr, TestConstraintSystem},
    utilities::{
        alloc::AllocGadget,
        uint::{UInt, UInt128, UInt16, UInt32, UInt64, UInt8},
    },
};

use criterion::{criterion_group, criterion_main, Criterion};
use rand::{Rng, SeedableRng};
use rand_xorshift::XorShiftRng;

macro_rules! create_addmany_bench {
    ($bench_name:ident, $bench_id:expr, $foo_name:ident, $std_type:ty, $bit_type:ty) => {
        fn $bench_name(c: &mut Criterion) {
            fn $foo_name(cs: &mut TestConstraintSystem<Fr>, rng: &mut XorShiftRng) {
                let a: $std_type = rng.gen();
                let b: $std_type = rng.gen();
                let c: $std_type = rng.gen();

                let bench_run_id: u64 = rng.gen();

                let a_bit = <$bit_type>::alloc(cs.ns(|| format!("{}: a (addmany)", bench_run_id)), || Ok(a)).unwrap();
                let b_bit = <$bit_type>::alloc(cs.ns(|| format!("{}: b (addmany)", bench_run_id)), || Ok(b)).unwrap();
                let c_bit = <$bit_type>::alloc(cs.ns(|| format!("{}: c (addmany)", bench_run_id)), || Ok(c)).unwrap();

                <$bit_type>::addmany(cs.ns(|| format!("{}: addmany &[a, b, c]", bench_run_id)), &[
                    a_bit, b_bit, c_bit,
                ])
                .unwrap();
            }

            let mut cs = TestConstraintSystem::<Fr>::new();

            let mut rng = XorShiftRng::seed_from_u64(1231275789u64);

            c.bench_function(&format!("integer_arithmetic::{}", $bench_id), |b| {
                b.iter(|| $foo_name(&mut cs, &mut rng))
            });
        }
    };
}

macro_rules! create_addmany_const_bench {
    ($bench_name:ident, $bench_id:expr, $foo_name:ident, $std_type:ty, $bit_type:ty) => {
        fn $bench_name(c: &mut Criterion) {
            fn $foo_name(cs: &mut TestConstraintSystem<Fr>, rng: &mut XorShiftRng) {
                let a: $std_type = rng.gen();
                let b: $std_type = rng.gen();
                let c: $std_type = rng.gen();

                let bench_run_id: u64 = rng.gen();

                let a_bit = <$bit_type>::constant(a);
                let b_bit = <$bit_type>::constant(b);
                let c_bit = <$bit_type>::constant(c);

                <$bit_type>::addmany(cs.ns(|| format!("{}: addmany &[a, b, c]", bench_run_id)), &[
                    a_bit, b_bit, c_bit,
                ])
                .unwrap();
            }

            let mut cs = TestConstraintSystem::<Fr>::new();

            let mut rng = XorShiftRng::seed_from_u64(1231275789u64);

            c.bench_function(&format!("integer_arithmetic::{}", $bench_id), |b| {
                b.iter(|| $foo_name(&mut cs, &mut rng))
            });
        }
    };
}

macro_rules! create_sub_bench {
    ($bench_name:ident, $bench_id:expr, $foo_name:ident, $std_type:ty, $bit_type:ty) => {
        fn $bench_name(c: &mut Criterion) {
            fn $foo_name(cs: &mut TestConstraintSystem<Fr>, rng: &mut XorShiftRng) {
                let a: $std_type = rng.gen_range(<$std_type>::max_value() / 2, <$std_type>::max_value());
                let b: $std_type = rng.gen_range(0, <$std_type>::max_value() / 2);

                let bench_run_id: u64 = rng.gen();

                let a_bit = <$bit_type>::alloc(cs.ns(|| format!("{}: a (sub)", bench_run_id)), || Ok(a)).unwrap();
                let b_bit = <$bit_type>::alloc(cs.ns(|| format!("{}: b (sub)", bench_run_id)), || Ok(b)).unwrap();

                a_bit
                    .sub(cs.ns(|| format!("{}: a sub b", bench_run_id)), &b_bit)
                    .unwrap();
            }

            let mut cs = TestConstraintSystem::<Fr>::new();

            let mut rng = XorShiftRng::seed_from_u64(1231275789u64);

            c.bench_function(&format!("integer_arithmetic::{}", $bench_id), |b| {
                b.iter(|| $foo_name(&mut cs, &mut rng))
            });
        }
    };
}

macro_rules! create_sub_const_bench {
    ($bench_name:ident, $bench_id:expr, $foo_name:ident, $std_type:ty, $bit_type:ty) => {
        fn $bench_name(c: &mut Criterion) {
            fn $foo_name(cs: &mut TestConstraintSystem<Fr>, rng: &mut XorShiftRng) {
                let a: $std_type = rng.gen_range(<$std_type>::max_value() / 2, <$std_type>::max_value());
                let b: $std_type = rng.gen_range(0, <$std_type>::max_value() / 2);

                let bench_run_id: u64 = rng.gen();

                let a_bit = <$bit_type>::constant(a);
                let b_bit = <$bit_type>::constant(b);

                a_bit
                    .sub(cs.ns(|| format!("{}: a sub b", bench_run_id)), &b_bit)
                    .unwrap();
            }

            let mut cs = TestConstraintSystem::<Fr>::new();

            let mut rng = XorShiftRng::seed_from_u64(1231275789u64);

            c.bench_function(&format!("integer_arithmetic::{}", $bench_id), |b| {
                b.iter(|| $foo_name(&mut cs, &mut rng))
            });
        }
    };
}

macro_rules! create_mul_bench {
    ($bench_name:ident, $bench_id:expr, $foo_name:ident, $std_type:ty, $bit_type:ty) => {
        fn $bench_name(c: &mut Criterion) {
            fn $foo_name(cs: &mut TestConstraintSystem<Fr>, rng: &mut XorShiftRng) {
                let a: $std_type = rng.gen();
                let b: $std_type = rng.gen();

                let bench_run_id: u64 = rng.gen();

                let a_bit = <$bit_type>::alloc(cs.ns(|| format!("{}: a (mul)", bench_run_id)), || Ok(a)).unwrap();
                let b_bit = <$bit_type>::alloc(cs.ns(|| format!("{}: b (mul)", bench_run_id)), || Ok(b)).unwrap();

                a_bit
                    .mul(cs.ns(|| format!("{}: a mul b", bench_run_id)), &b_bit)
                    .unwrap();
            }

            let mut cs = TestConstraintSystem::<Fr>::new();

            let mut rng = XorShiftRng::seed_from_u64(1231275789u64);

            c.bench_function(&format!("integer_arithmetic::{}", $bench_id), |b| {
                b.iter(|| $foo_name(&mut cs, &mut rng))
            });
        }
    };
}

macro_rules! create_mul_const_bench {
    ($bench_name:ident, $bench_id:expr, $foo_name:ident, $std_type:ty, $bit_type:ty) => {
        fn $bench_name(c: &mut Criterion) {
            fn $foo_name(cs: &mut TestConstraintSystem<Fr>, rng: &mut XorShiftRng) {
                let a: $std_type = rng.gen();
                let b: $std_type = rng.gen();

                let bench_run_id: u64 = rng.gen();

                let a_bit = <$bit_type>::constant(a);
                let b_bit = <$bit_type>::constant(b);

                a_bit
                    .mul(cs.ns(|| format!("{}: a mul b", bench_run_id)), &b_bit)
                    .unwrap();
            }

            let mut cs = TestConstraintSystem::<Fr>::new();

            let mut rng = XorShiftRng::seed_from_u64(1231275789u64);

            c.bench_function(&format!("integer_arithmetic::{}", $bench_id), |b| {
                b.iter(|| $foo_name(&mut cs, &mut rng))
            });
        }
    };
}

macro_rules! create_div_bench {
    ($bench_name:ident, $bench_id:expr, $foo_name:ident, $std_type:ty, $bit_type:ty) => {
        fn $bench_name(c: &mut Criterion) {
            fn $foo_name(cs: &mut TestConstraintSystem<Fr>, rng: &mut XorShiftRng) {
                let a: $std_type = rng.gen();
                let b: $std_type = rng.gen_range(1, <$std_type>::max_value());

                let bench_run_id: u64 = rng.gen();

                let a_bit = <$bit_type>::alloc(cs.ns(|| format!("{}: a (div)", bench_run_id)), || Ok(a)).unwrap();
                let b_bit = <$bit_type>::alloc(cs.ns(|| format!("{}: b (div)", bench_run_id)), || Ok(b)).unwrap();

                a_bit
                    .div(cs.ns(|| format!("{}: a div b", bench_run_id)), &b_bit)
                    .unwrap();
            }

            let mut cs = TestConstraintSystem::<Fr>::new();

            let mut rng = XorShiftRng::seed_from_u64(1231275789u64);

            c.bench_function(&format!("integer_arithmetic::{}", $bench_id), |b| {
                b.iter(|| $foo_name(&mut cs, &mut rng))
            });
        }
    };
}

macro_rules! create_div_const_bench {
    ($bench_name:ident, $bench_id:expr, $foo_name:ident, $std_type:ty, $bit_type:ty) => {
        fn $bench_name(c: &mut Criterion) {
            fn $foo_name(cs: &mut TestConstraintSystem<Fr>, rng: &mut XorShiftRng) {
                let a: $std_type = rng.gen();
                let b: $std_type = rng.gen_range(1, <$std_type>::max_value());

                let bench_run_id: u64 = rng.gen();

                let a_bit = <$bit_type>::constant(a);
                let b_bit = <$bit_type>::constant(b);

                a_bit
                    .div(cs.ns(|| format!("{}: a div b", bench_run_id)), &b_bit)
                    .unwrap();
            }

            let mut cs = TestConstraintSystem::<Fr>::new();

            let mut rng = XorShiftRng::seed_from_u64(1231275789u64);

            c.bench_function(&format!("integer_arithmetic::{}", $bench_id), |b| {
                b.iter(|| $foo_name(&mut cs, &mut rng))
            });
        }
    };
}

macro_rules! create_pow_bench {
    ($bench_name:ident, $bench_id:expr, $foo_name:ident, $std_type:ty, $bit_type:ty) => {
        fn $bench_name(c: &mut Criterion) {
            fn $foo_name(cs: &mut TestConstraintSystem<Fr>, rng: &mut XorShiftRng) {
                let a: $std_type = rng.gen_range(0, <$std_type>::from(u8::max_value()));
                let b: $std_type = rng.gen_range(0, 4);

                let bench_run_id: u64 = rng.gen();

                let a_bit = <$bit_type>::alloc(cs.ns(|| format!("{}: a (pow)", bench_run_id)), || Ok(a)).unwrap();
                let b_bit = <$bit_type>::alloc(cs.ns(|| format!("{}: b (pow)", bench_run_id)), || Ok(b)).unwrap();

                a_bit
                    .pow(cs.ns(|| format!("{}: a pow b", bench_run_id)), &b_bit)
                    .unwrap();
            }

            let mut cs = TestConstraintSystem::<Fr>::new();

            let mut rng = XorShiftRng::seed_from_u64(1231275789u64);

            c.bench_function(&format!("integer_arithmetic::{}", $bench_id), |b| {
                b.iter(|| $foo_name(&mut cs, &mut rng))
            });
        }
    };
}

macro_rules! create_pow_const_bench {
    ($bench_name:ident, $bench_id:expr, $foo_name:ident, $std_type:ty, $bit_type:ty) => {
        fn $bench_name(c: &mut Criterion) {
            fn $foo_name(cs: &mut TestConstraintSystem<Fr>, rng: &mut XorShiftRng) {
                let a: $std_type = rng.gen_range(0, <$std_type>::from(u8::max_value()));
                let b: $std_type = rng.gen_range(0, 4);

                let bench_run_id: u64 = rng.gen();

                let a_bit = <$bit_type>::constant(a);
                let b_bit = <$bit_type>::constant(b);

                a_bit
                    .pow(cs.ns(|| format!("{}: a pow b", bench_run_id)), &b_bit)
                    .unwrap();
            }

            let mut cs = TestConstraintSystem::<Fr>::new();

            let mut rng = XorShiftRng::seed_from_u64(1231275789u64);

            c.bench_function(&format!("integer_arithmetic::{}", $bench_id), |b| {
                b.iter(|| $foo_name(&mut cs, &mut rng))
            });
        }
    };
}

create_addmany_bench!(bench_u8_addmany, "u8_addmany", u8_addmany, u8, UInt8);
create_addmany_bench!(bench_u16_addmany, "u16_addmany", u16_addmany, u16, UInt16);
create_addmany_bench!(bench_u32_addmany, "u32_addmany", u32_addmany, u32, UInt32);
create_addmany_bench!(bench_u64_addmany, "u64_addmany", u64_addmany, u64, UInt64);
create_addmany_bench!(bench_u128_addmany, "u128_addmany", u128_addmany, u128, UInt128);

create_sub_bench!(bench_u8_sub, "u8_sub", u8_sub, u8, UInt8);
create_sub_bench!(bench_u16_sub, "u16_sub", u16_sub, u16, UInt16);
create_sub_bench!(bench_u32_sub, "u32_sub", u32_sub, u32, UInt32);
create_sub_bench!(bench_u64_sub, "u64_sub", u64_sub, u64, UInt64);
create_sub_bench!(bench_u128_sub, "u128_sub", u128_sub, u128, UInt128);

create_mul_bench!(bench_u8_mul, "u8_mul", u8_mul, u8, UInt8);
create_mul_bench!(bench_u16_mul, "u16_mul", u16_mul, u16, UInt16);
create_mul_bench!(bench_u32_mul, "u32_mul", u32_mul, u32, UInt32);
create_mul_bench!(bench_u64_mul, "u64_mul", u64_mul, u64, UInt64);
// create_mul_bench!(bench_u128_mul, "u128_mul", u128_mul, u128, UInt128);

create_div_bench!(bench_u8_div, "u8_div", u8_div, u8, UInt8);
create_div_bench!(bench_u16_div, "u16_div", u16_div, u16, UInt16);
create_div_bench!(bench_u32_div, "u32_div", u32_div, u32, UInt32);
create_div_bench!(bench_u64_div, "u64_div", u64_div, u64, UInt64);
create_div_bench!(bench_u128_div, "u128_div", u128_div, u128, UInt128);

create_pow_bench!(bench_u8_pow, "u8_pow", u8_pow, u8, UInt8);
create_pow_bench!(bench_u16_pow, "u16_pow", u16_pow, u16, UInt16);
// create_pow_bench!(bench_u32_pow, "u32_pow", u32_pow, u32, UInt32);
// create_pow_bench!(bench_u64_pow, "u64_pow", u64_pow, u64, UInt64);
// create_pow_bench!(bench_u128_pow, "u128_pow", u128_pow, u128, UInt128);

create_addmany_const_bench!(bench_u8_addmany_const, "u8_addmany_const", u8_addmany_const, u8, UInt8);
create_addmany_const_bench!(
    bench_u16_addmany_const,
    "u16_addmany_const",
    u16_addmany_const,
    u16,
    UInt16
);
create_addmany_const_bench!(
    bench_u32_addmany_const,
    "u32_addmany_const",
    u32_addmany_const,
    u32,
    UInt32
);
create_addmany_const_bench!(
    bench_u64_addmany_const,
    "u64_addmany_const",
    u64_addmany_const,
    u64,
    UInt64
);
create_addmany_const_bench!(
    bench_u128_addmany_const,
    "u128_addmany_const",
    u128_addmany_const,
    u128,
    UInt128
);

create_sub_const_bench!(bench_u8_sub_const, "u8_sub_const", u8_sub_const, u8, UInt8);
create_sub_const_bench!(bench_u16_sub_const, "u16_sub_const", u16_sub_const, u16, UInt16);
create_sub_const_bench!(bench_u32_sub_const, "u32_sub_const", u32_sub_const, u32, UInt32);
create_sub_const_bench!(bench_u64_sub_const, "u64_sub_const", u64_sub_const, u64, UInt64);
create_sub_const_bench!(bench_u128_sub_const, "u128_sub_const", u128_sub_const, u128, UInt128);

create_mul_const_bench!(bench_u8_mul_const, "u8_mul_const", u8_mul_const, u8, UInt8);
create_mul_const_bench!(bench_u16_mul_const, "u16_mul_const", u16_mul_const, u16, UInt16);
create_mul_const_bench!(bench_u32_mul_const, "u32_mul_const", u32_mul_const, u32, UInt32);
create_mul_const_bench!(bench_u64_mul_const, "u64_mul_const", u64_mul_const, u64, UInt64);
create_mul_const_bench!(bench_u128_mul_const, "u128_mul_const", u128_mul_const, u128, UInt128);

create_div_const_bench!(bench_u8_div_const, "u8_div_const", u8_div_const, u8, UInt8);
create_div_const_bench!(bench_u16_div_const, "u16_div_const", u16_div_const, u16, UInt16);
create_div_const_bench!(bench_u32_div_const, "u32_div_const", u32_div_const, u32, UInt32);
create_div_const_bench!(bench_u64_div_const, "u64_div_const", u64_div_const, u64, UInt64);
create_div_const_bench!(bench_u128_div_const, "u128_div_const", u128_div_const, u128, UInt128);

create_pow_const_bench!(bench_u8_pow_const, "u8_pow_const", u8_pow_const, u8, UInt8);
create_pow_const_bench!(bench_u16_pow_const, "u16_pow_const", u16_pow_const, u16, UInt16);
// create_pow_const_bench!(bench_u32_pow_const, "u32_pow_const", u32_pow_const, u32, UInt32);
// create_pow_const_bench!(bench_u64_pow_const, "u64_pow_const", u64_pow_const, u64, UInt64);
// create_pow_const_bench!(bench_u128_pow_const, "u128_pow_const", u128_pow_const, u128, UInt128);

criterion_group!(
    name = benches_addmany;
    config = Criterion::default();
    targets = bench_u8_addmany,
    bench_u16_addmany,
    bench_u32_addmany,
    bench_u64_addmany,
    bench_u128_addmany,
);

criterion_group!(
    name = benches_sub;
    config = Criterion::default();
    targets = bench_u8_sub,
    bench_u16_sub,
    bench_u32_sub,
    bench_u64_sub,
    bench_u128_sub,
);

criterion_group!(
    name = benches_mul;
    config = Criterion::default();
    targets = bench_u8_mul,
    bench_u16_mul,
    bench_u32_mul,
    bench_u64_mul,
    // bench_u128_mul,
);

criterion_group!(
    name = benches_div;
    config = Criterion::default();
    targets = bench_u8_div,
    bench_u16_div,
    bench_u32_div,
    bench_u64_div,
    bench_u128_div,
);

criterion_group!(
    name = benches_pow;
    config = Criterion::default();
    targets = bench_u8_pow,
    bench_u16_pow,
    // bench_u32_pow,
    // bench_u64_pow,
    // bench_u128_pow,
);

criterion_group!(
    name = benches_addmany_const;
    config = Criterion::default();
    targets = bench_u8_addmany_const,
    bench_u16_addmany_const,
    bench_u32_addmany_const,
    bench_u64_addmany_const,
    bench_u128_addmany_const,
);

criterion_group!(
    name = benches_sub_const;
    config = Criterion::default();
    targets = bench_u8_sub_const,
    bench_u16_sub_const,
    bench_u32_sub_const,
    bench_u64_sub_const,
    bench_u128_sub_const,
);

criterion_group!(
    name = benches_mul_const;
    config = Criterion::default();
    targets = bench_u8_mul_const,
    bench_u16_mul_const,
    bench_u32_mul_const,
    bench_u64_mul_const,
    bench_u128_mul_const,
);

criterion_group!(
    name = benches_div_const;
    config = Criterion::default();
    targets = bench_u8_div_const,
    bench_u16_div_const,
    bench_u32_div_const,
    bench_u64_div_const,
    bench_u128_div_const,
);

criterion_group!(
    name = benches_pow_const;
    config = Criterion::default();
    targets = bench_u8_pow_const,
    bench_u16_pow_const,
    // bench_u32_pow_const,
    // bench_u64_pow_const,
    // bench_u128_pow_const,
);

criterion_main!(
    benches_addmany,
    benches_sub,
    benches_mul,
    benches_div,
    benches_pow,
    benches_addmany_const,
    benches_sub_const,
    benches_mul_const,
    benches_div_const,
    benches_pow
);
