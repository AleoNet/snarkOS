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

use snarkos_models::curves::{Fp256, Fp256Parameters, FpParameters};
use snarkos_utilities::biginteger::BigInteger256 as BigInteger;

pub type Fr = Fp256<FrParameters>;

pub struct FrParameters;

impl Fp256Parameters for FrParameters {}

impl FpParameters for FrParameters {
    type BigInteger = BigInteger;

    const CAPACITY: u32 = Self::MODULUS_BITS - 1;
    // 5
    const GENERATOR: BigInteger = BigInteger([
        11289572479685143826u64,
        11383637369941080925u64,
        2288212753973340071u64,
        82014976407880291u64,
    ]);
    const INV: u64 = 9659935179256617473u64;
    // MODULUS = 2111115437357092606062206234695386632838870926408408195193685246394721360383
    const MODULUS: BigInteger = BigInteger([
        13356249993388743167u64,
        5950279507993463550u64,
        10965441865914903552u64,
        336320092672043349u64,
    ]);
    const MODULUS_BITS: u32 = 251;
    const MODULUS_MINUS_ONE_DIV_TWO: BigInteger = BigInteger([
        6678124996694371583u64,
        2975139753996731775u64,
        14706092969812227584u64,
        168160046336021674u64,
    ]);
    const R: BigInteger = BigInteger([
        16632263305389933622u64,
        10726299895124897348u64,
        16608693673010411502u64,
        285459069419210737u64,
    ]);
    const R2: BigInteger = BigInteger([
        3987543627614508126u64,
        17742427666091596403u64,
        14557327917022607905u64,
        322810149704226881u64,
    ]);
    const REPR_SHAVE_BITS: u32 = 5;
    const ROOT_OF_UNITY: BigInteger = BigInteger([
        15170730761708361161u64,
        13670723686578117817u64,
        12803492266614043665u64,
        50861023252832611u64,
    ]);
    const T: BigInteger = BigInteger([0x0, 0x0, 0x0, 0x0]);
    const TWO_ADICITY: u32 = 1;
    const T_MINUS_ONE_DIV_TWO: BigInteger = BigInteger([0x0, 0x0, 0x0, 0x0]);
}
