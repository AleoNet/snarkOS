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

use crate::bw6_761::Fq;
use snarkos_models::{
    curves::{Fp3, Fp3Parameters},
    field,
};
use snarkos_utilities::biginteger::BigInteger768 as BigInteger;

use serde::{Deserialize, Serialize};

pub type Fq3 = Fp3<Fq3Parameters>;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fq3Parameters;

impl Fp3Parameters for Fq3Parameters {
    type Fp = Fq;

    // NQR ^ (MODULUS^i - 1)/3, i=0,1,2 with NQR = u = (0,1,0)
    const FROBENIUS_COEFF_FP3_C1: [Fq; 3] = [
        field!(
            Fq,
            BigInteger([
                0x0202ffffffff85d5,
                0x5a5826358fff8ce7,
                0x9e996e43827faade,
                0xda6aff320ee47df4,
                0xece9cb3e1d94b80b,
                0xc0e667a25248240b,
                0xa74da5bfdcad3905,
                0x2352e7fe462f2103,
                0x7b56588008b1c87c,
                0x45848a63e711022f,
                0xd7a81ebb9f65a9df,
                0x51f77ef127e87d,
            ])
        ),
        field!(
            Fq,
            BigInteger([
                0x7f96b51bd840c549,
                0xd59782096496171f,
                0x49b046fd9ce14bbc,
                0x4b6163bba7527a56,
                0xef6c92fb771d59f1,
                0x0425bedbac1dfdc7,
                0xd3ac39de759c0ffd,
                0x9f43ed0e063a81d0,
                0x5bd7d20b4f9a3ce2,
                0x0411f03c36cf5c3c,
                0x2d658fd49661c472,
                0x1100249ae760b93,
            ])
        ),
        field!(
            Fq,
            BigInteger([
                0x67a04ae427bfb5f8,
                0x9d32d491eb6a5cff,
                0x43d03c1cb68051d4,
                0x0b75ca96f69859a5,
                0x0763497f5325ec60,
                0x48076b5c278dd94d,
                0x8ca3965ff91efd06,
                0x1e6077657ea02f5d,
                0xcdd6c153a8c37724,
                0x28b5b634e5c22ea4,
                0x9e01e3efd42e902c,
                0xe3d6815769a804,
            ])
        ),
    ];
    // NQR ^ (2*MODULUS^i - 2)/3, i=0,1,2 with NQR = u = (0,1,0)
    const FROBENIUS_COEFF_FP3_C2: [Fq; 3] = [
        field!(
            Fq,
            BigInteger([
                0x0202ffffffff85d5,
                0x5a5826358fff8ce7,
                0x9e996e43827faade,
                0xda6aff320ee47df4,
                0xece9cb3e1d94b80b,
                0xc0e667a25248240b,
                0xa74da5bfdcad3905,
                0x2352e7fe462f2103,
                0x7b56588008b1c87c,
                0x45848a63e711022f,
                0xd7a81ebb9f65a9df,
                0x51f77ef127e87d,
            ])
        ),
        field!(
            Fq,
            BigInteger([
                0x67a04ae427bfb5f8,
                0x9d32d491eb6a5cff,
                0x43d03c1cb68051d4,
                0x0b75ca96f69859a5,
                0x0763497f5325ec60,
                0x48076b5c278dd94d,
                0x8ca3965ff91efd06,
                0x1e6077657ea02f5d,
                0xcdd6c153a8c37724,
                0x28b5b634e5c22ea4,
                0x9e01e3efd42e902c,
                0xe3d6815769a804,
            ])
        ),
        field!(
            Fq,
            BigInteger([
                0x7f96b51bd840c549,
                0xd59782096496171f,
                0x49b046fd9ce14bbc,
                0x4b6163bba7527a56,
                0xef6c92fb771d59f1,
                0x0425bedbac1dfdc7,
                0xd3ac39de759c0ffd,
                0x9f43ed0e063a81d0,
                0x5bd7d20b4f9a3ce2,
                0x0411f03c36cf5c3c,
                0x2d658fd49661c472,
                0x1100249ae760b93,
            ])
        ),
    ];
    /// NONRESIDUE = -4
    // Fq3 = Fq[u]/u^3+4
    const NONRESIDUE: Fq = field!(
        Fq,
        BigInteger([
            0xe12e00000001e9c2,
            0x63c1e3faa001cd69,
            0xb1b4384fcbe29cf6,
            0xc79630bc713d5a1d,
            0x30127ac071851e2d,
            0x0979f350dcd36af1,
            0x6a66defed8b361f2,
            0x53abac78b24d4e23,
            0xb7ab89dede485a92,
            0x5c3a0745675e8452,
            0x446f17918c5f5700,
            0xfdf24e3267fa1e,
        ])
    );
    // NONRESIDUE^T % q
    const QUADRATIC_NONRESIDUE_TO_T: (Fq, Fq, Fq) = (
        field!(
            Fq,
            BigInteger([
                0xf29a000000007ab6,
                0x8c391832e000739b,
                0x77738a6b6870f959,
                0xbe36179047832b03,
                0x84f3089e56574722,
                0xc5a3614ac0b1d984,
                0x5c81153f4906e9fe,
                0x4d28be3a9f55c815,
                0xd72c1d6f77d5f5c5,
                0x73a18e069ac04458,
                0xf9dfaa846595555f,
                0xd0f0a60a5be58c,
            ])
        ),
        field!(Fq, BigInteger([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0])),
        field!(Fq, BigInteger([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0])),
    );
    // (MODULUS^3 - 1) % 2^TWO_ADICITY == 0
    const TWO_ADICITY: u32 = 1;
    // (T-1)/2 with T = (MODULUS^3-1) / 2^TWO_ADICITY
    const T_MINUS_ONE_DIV_TWO: &'static [u64] = &[
        0xb5e7c000000a3eac,
        0xf79b99dbf41cf4ab,
        0xe9372b1919e55ee5,
        0xbb7bbc4936c1980b,
        0x7c0cb9d4399b36e1,
        0x73304a5507bb1ae0,
        0x92f639be8963936f,
        0x4f574ac2439ba816,
        0x670d9bd389dd29ef,
        0x606ddf900d2124f1,
        0x928fb14985ec3270,
        0x6b2f2428c5f420f3,
        0xac9ade29d5ab5fbe,
        0xec0d0434c4005822,
        0x973f10d7f3c5c108,
        0x6d5e83fc81095979,
        0xdac3e6e4e1647752,
        0x227febf93994603e,
        0x4ab8755d894167d1,
        0x4fd2d3f67d8b537a,
        0x33e196a4d5f4030a,
        0x88b51fb72092df1a,
        0xa67e5b1e8fc48316,
        0xb0855eb2a00d7dab,
        0xe875dd2da6751442,
        0x777594a243e25676,
        0x294e0f70376a85a8,
        0x83f431c7988e4f18,
        0x8e8fb6af3ca2f5f1,
        0x7297896b4b9e90f1,
        0xff38f54664d66123,
        0xb5ecf80bfff41e13,
        0x1662a3666bb8392a,
        0x07a0968e8742d3e1,
        0xf12927e564bcdfdc,
        0x5de9825a0e,
    ];

    #[inline(always)]
    fn mul_fp_by_nonresidue(fe: &Self::Fp) -> Self::Fp {
        let original = -(*fe);
        let double = original + &original;
        double + &double
    }
}
