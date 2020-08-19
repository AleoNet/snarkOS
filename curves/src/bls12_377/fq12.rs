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

use crate::bls12_377::{Fq, Fq2, Fq6Parameters};
use snarkos_models::{
    curves::{Fp12, Fp12Parameters},
    field,
};
use snarkos_utilities::biginteger::BigInteger384 as BigInteger;

pub type Fq12 = Fp12<Fq12Parameters>;

#[derive(Clone, Copy)]
pub struct Fq12Parameters;

impl Fp12Parameters for Fq12Parameters {
    type Fp6Params = Fq6Parameters;

    const FROBENIUS_COEFF_FP12_C1: [Fq2; 12] = [
        // Fp2::NONRESIDUE^(((q^0) - 1) / 6)
        field!(
            Fq2,
            field!(
                Fq,
                BigInteger([
                    0x2cdffffffffff68,
                    0x51409f837fffffb1,
                    0x9f7db3a98a7d3ff2,
                    0x7b4e97b76e7c6305,
                    0x4cf495bf803c84e8,
                    0x8d6661e2fdf49a,
                ])
            ),
            field!(Fq, BigInteger([0x0, 0x0, 0x0, 0x0, 0x0, 0x0])),
        ),
        // Fp2::NONRESIDUE^(((q^1) - 1) / 6)
        field!(
            Fq2,
            field!(
                Fq,
                BigInteger([
                    0x6ec47a04a3f7ca9e,
                    0xa42e0cb968c1fa44,
                    0x578d5187fbd2bd23,
                    0x930eeb0ac79dd4bd,
                    0xa24883de1e09a9ee,
                    0xdaa7058067d46f,
                ])
            ),
            field!(Fq, BigInteger([0x0, 0x0, 0x0, 0x0, 0x0, 0x0])),
        ),
        // Fp2::NONRESIDUE^(((q^2) - 1) / 6)
        field!(
            Fq2,
            field!(
                Fq,
                BigInteger([
                    0x5892506da58478da,
                    0x133366940ac2a74b,
                    0x9b64a150cdf726cf,
                    0x5cc426090a9c587e,
                    0x5cf848adfdcd640c,
                    0x4702bf3ac02380,
                ])
            ),
            field!(Fq, BigInteger([0x0, 0x0, 0x0, 0x0, 0x0, 0x0])),
        ),
        // Fp2::NONRESIDUE^(((q^3) - 1) / 6)
        field!(
            Fq2,
            field!(
                Fq,
                BigInteger([
                    0x982c13d9d084771f,
                    0xfd49de0c6da34a32,
                    0x61a530d183ab0e53,
                    0xdf8fe44106dd9879,
                    0x40f29b58d88472bc,
                    0x158723199046d5d,
                ])
            ),
            field!(Fq, BigInteger([0x0, 0x0, 0x0, 0x0, 0x0, 0x0])),
        ),
        // Fp2::NONRESIDUE^(((q^4) - 1) / 6)
        field!(
            Fq2,
            field!(
                Fq,
                BigInteger([
                    0xdacd106da5847973,
                    0xd8fe2454bac2a79a,
                    0x1ada4fd6fd832edc,
                    0xfb9868449d150908,
                    0xd63eb8aeea32285e,
                    0x167d6a36f873fd0,
                ])
            ),
            field!(Fq, BigInteger([0x0, 0x0, 0x0, 0x0, 0x0, 0x0])),
        ),
        // Fp2::NONRESIDUE^(((q^5) - 1) / 6)
        field!(
            Fq2,
            field!(
                Fq,
                BigInteger([
                    0x296799d52c8cac81,
                    0x591bd15304e14fee,
                    0xa17df4987d85130,
                    0x4c80f9363f3fc3bc,
                    0x9eaa177aba7ac8ce,
                    0x7dcb2c189c98ed,
                ])
            ),
            field!(Fq, BigInteger([0x0, 0x0, 0x0, 0x0, 0x0, 0x0])),
        ),
        // Fp2::NONRESIDUE^(((q^6) - 1) / 6)
        field!(
            Fq2,
            field!(
                Fq,
                BigInteger([
                    0x823ac00000000099,
                    0xc5cabdc0b000004f,
                    0x7f75ae862f8c080d,
                    0x9ed4423b9278b089,
                    0x79467000ec64c452,
                    0x120d3e434c71c50,
                ])
            ),
            field!(Fq, BigInteger([0x0, 0x0, 0x0, 0x0, 0x0, 0x0])),
        ),
        // Fp2::NONRESIDUE^(((q^7) - 1) / 6)
        field!(
            Fq2,
            field!(
                Fq,
                BigInteger([
                    0x164445fb5c083563,
                    0x72dd508ac73e05bc,
                    0xc76610a7be368adc,
                    0x8713eee839573ed1,
                    0x23f281e24e979f4c,
                    0xd39340975d3c7b,
                ])
            ),
            field!(Fq, BigInteger([0x0, 0x0, 0x0, 0x0, 0x0, 0x0])),
        ),
        // Fp2::NONRESIDUE^(((q^8) - 1) / 6)
        field!(
            Fq2,
            field!(
                Fq,
                BigInteger([
                    0x2c766f925a7b8727,
                    0x3d7f6b0253d58b5,
                    0x838ec0deec122131,
                    0xbd5eb3e9f658bb10,
                    0x6942bd126ed3e52e,
                    0x1673786dd04ed6a,
                ])
            ),
            field!(Fq, BigInteger([0x0, 0x0, 0x0, 0x0, 0x0, 0x0])),
        ),
        // Fp2::NONRESIDUE^(((q^9) - 1) / 6)
        field!(
            Fq2,
            field!(
                Fq,
                BigInteger([
                    0xecdcac262f7b88e2,
                    0x19c17f37c25cb5cd,
                    0xbd4e315e365e39ac,
                    0x3a92f5b1fa177b15,
                    0x85486a67941cd67e,
                    0x55c8147ec0a38d,
                ])
            ),
            field!(Fq, BigInteger([0x0, 0x0, 0x0, 0x0, 0x0, 0x0])),
        ),
        // Fp2::NONRESIDUE^(((q^10) - 1) / 6)
        field!(
            Fq2,
            field!(
                Fq,
                BigInteger([
                    0xaa3baf925a7b868e,
                    0x3e0d38ef753d5865,
                    0x4191258bc861923,
                    0x1e8a71ae63e00a87,
                    0xeffc4d11826f20dc,
                    0x4663a2a83dd119,
                ])
            ),
            field!(Fq, BigInteger([0x0, 0x0, 0x0, 0x0, 0x0, 0x0])),
        ),
        // Fp2::NONRESIDUE^(((q^11) - 1) / 6)
        field!(
            Fq2,
            field!(
                Fq,
                BigInteger([
                    0x5ba1262ad3735380,
                    0xbdef8bf12b1eb012,
                    0x14db82e63230f6cf,
                    0xcda1e0bcc1b54fd3,
                    0x2790ee45b226806c,
                    0x1306f19ff2877fd,
                ])
            ),
            field!(Fq, BigInteger([0x0, 0x0, 0x0, 0x0, 0x0, 0x0])),
        ),
    ];
}
