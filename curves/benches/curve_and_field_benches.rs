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

use criterion::{criterion_group, criterion_main};

mod bls12_377;
mod bw6_761;
// mod sw6;

criterion_group!(
    bls12_377_ec,
    bls12_377::ec::g1::bench_g1_rand,
    bls12_377::ec::g1::bench_g1_mul_assign,
    bls12_377::ec::g1::bench_g1_add_assign,
    bls12_377::ec::g1::bench_g1_add_assign_mixed,
    bls12_377::ec::g1::bench_g1_double,
    bls12_377::ec::g2::bench_g2_rand,
    bls12_377::ec::g2::bench_g2_mul_assign,
    bls12_377::ec::g2::bench_g2_add_assign,
    bls12_377::ec::g2::bench_g2_add_assign_mixed,
    bls12_377::ec::g2::bench_g2_double,
);

criterion_group!(
    bls12_377_fq,
    bls12_377::fq::bench_fq_repr_add_nocarry,
    bls12_377::fq::bench_fq_repr_sub_noborrow,
    bls12_377::fq::bench_fq_repr_num_bits,
    bls12_377::fq::bench_fq_repr_mul2,
    bls12_377::fq::bench_fq_repr_div2,
    bls12_377::fq::bench_fq_add_assign,
    bls12_377::fq::bench_fq_sub_assign,
    bls12_377::fq::bench_fq_mul_assign,
    bls12_377::fq::bench_fq_double,
    bls12_377::fq::bench_fq_square,
    bls12_377::fq::bench_fq_inverse,
    bls12_377::fq::bench_fq_negate,
    bls12_377::fq::bench_fq_sqrt,
    bls12_377::fq::bench_fq_into_repr,
    bls12_377::fq::bench_fq_from_repr,
);

criterion_group!(
    bls12_377_fq12,
    bls12_377::fq12::bench_fq12_add_assign,
    bls12_377::fq12::bench_fq12_sub_assign,
    bls12_377::fq12::bench_fq12_mul_assign,
    bls12_377::fq12::bench_fq12_double,
    bls12_377::fq12::bench_fq12_square,
    bls12_377::fq12::bench_fq12_inverse,
);

criterion_group!(
    bls12_377_fq2,
    bls12_377::fq2::bench_fq2_add_assign,
    bls12_377::fq2::bench_fq2_sub_assign,
    bls12_377::fq2::bench_fq2_mul_assign,
    bls12_377::fq2::bench_fq2_double,
    bls12_377::fq2::bench_fq2_square,
    bls12_377::fq2::bench_fq2_inverse,
    bls12_377::fq2::bench_fq2_sqrt,
);

criterion_group!(
    bls12_377_fr,
    bls12_377::fr::bench_fr_repr_add_nocarry,
    bls12_377::fr::bench_fr_repr_sub_noborrow,
    bls12_377::fr::bench_fr_repr_num_bits,
    bls12_377::fr::bench_fr_repr_mul2,
    bls12_377::fr::bench_fr_repr_div2,
    bls12_377::fr::bench_fr_add_assign,
    bls12_377::fr::bench_fr_sub_assign,
    bls12_377::fr::bench_fr_mul_assign,
    bls12_377::fr::bench_fr_double,
    bls12_377::fr::bench_fr_square,
    bls12_377::fr::bench_fr_inverse,
    bls12_377::fr::bench_fr_negate,
    bls12_377::fr::bench_fr_sqrt,
    bls12_377::fr::bench_fr_into_repr,
    bls12_377::fr::bench_fr_from_repr,
);

criterion_group!(
    bls12_377_pairing,
    bls12_377::pairing::pairing::bench_pairing_miller_loop,
    bls12_377::pairing::pairing::bench_pairing_final_exponentiation,
    bls12_377::pairing::pairing::bench_pairing_full,
);

criterion_group!(
    bw6_761_ec,
    bw6_761::ec::g1::bench_g1_rand,
    bw6_761::ec::g1::bench_g1_mul_assign,
    bw6_761::ec::g1::bench_g1_add_assign,
    bw6_761::ec::g1::bench_g1_add_assign_mixed,
    bw6_761::ec::g1::bench_g1_double,
    bw6_761::ec::g2::bench_g2_rand,
    bw6_761::ec::g2::bench_g2_mul_assign,
    bw6_761::ec::g2::bench_g2_add_assign,
    bw6_761::ec::g2::bench_g2_add_assign_mixed,
    bw6_761::ec::g2::bench_g2_double,
);

criterion_group!(
    bw6_761_fq,
    bw6_761::fq::bench_fq_repr_add_nocarry,
    bw6_761::fq::bench_fq_repr_sub_noborrow,
    bw6_761::fq::bench_fq_repr_num_bits,
    bw6_761::fq::bench_fq_repr_mul2,
    bw6_761::fq::bench_fq_repr_div2,
    bw6_761::fq::bench_fq_add_assign,
    bw6_761::fq::bench_fq_sub_assign,
    bw6_761::fq::bench_fq_mul_assign,
    bw6_761::fq::bench_fq_double,
    bw6_761::fq::bench_fq_square,
    bw6_761::fq::bench_fq_inverse,
    bw6_761::fq::bench_fq_negate,
    bw6_761::fq::bench_fq_sqrt,
    bw6_761::fq::bench_fq_into_repr,
    bw6_761::fq::bench_fq_from_repr,
);

criterion_group!(
    bw6_761_fq3,
    bw6_761::fq3::bench_fq3_add_assign,
    bw6_761::fq3::bench_fq3_sub_assign,
    bw6_761::fq3::bench_fq3_mul_assign,
    bw6_761::fq3::bench_fq3_double,
    bw6_761::fq3::bench_fq3_square,
    bw6_761::fq3::bench_fq3_inverse,
    bw6_761::fq3::bench_fq3_sqrt,
);

criterion_group!(
    bw6_761_fq6,
    bw6_761::fq6::bench_fq6_add_assign,
    bw6_761::fq6::bench_fq6_sub_assign,
    bw6_761::fq6::bench_fq6_mul_assign,
    bw6_761::fq6::bench_fq6_double,
    bw6_761::fq6::bench_fq6_square,
    bw6_761::fq6::bench_fq6_inverse,
);

criterion_group!(
    bw6_761_fr,
    bw6_761::fr::bench_fr_repr_add_nocarry,
    bw6_761::fr::bench_fr_repr_sub_noborrow,
    bw6_761::fr::bench_fr_repr_num_bits,
    bw6_761::fr::bench_fr_repr_mul2,
    bw6_761::fr::bench_fr_repr_div2,
    bw6_761::fr::bench_fr_add_assign,
    bw6_761::fr::bench_fr_sub_assign,
    bw6_761::fr::bench_fr_mul_assign,
    bw6_761::fr::bench_fr_double,
    bw6_761::fr::bench_fr_square,
    bw6_761::fr::bench_fr_inverse,
    bw6_761::fr::bench_fr_negate,
    bw6_761::fr::bench_fr_sqrt,
    bw6_761::fr::bench_fr_into_repr,
    bw6_761::fr::bench_fr_from_repr,
);

criterion_group!(
    bw6_761_pairing,
    bw6_761::pairing::pairing::bench_pairing_miller_loop,
    bw6_761::pairing::pairing::bench_pairing_final_exponentiation,
    bw6_761::pairing::pairing::bench_pairing_full,
);

criterion_main!(
    bls12_377_ec,
    bls12_377_fq,
    bls12_377_fq12,
    bls12_377_fq2,
    bls12_377_fr,
    bls12_377_pairing,
    bw6_761_ec,
    bw6_761_fq,
    bw6_761_fq3,
    bw6_761_fq6,
    bw6_761_fr,
    bw6_761_pairing,
);
