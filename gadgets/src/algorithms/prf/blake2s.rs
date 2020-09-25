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

use snarkos_algorithms::prf::Blake2s;
use snarkos_errors::gadgets::SynthesisError;
use snarkos_models::{
    curves::PrimeField,
    gadgets::{
        algorithms::PRFGadget,
        r1cs::ConstraintSystem,
        utilities::{
            alloc::AllocGadget,
            boolean::Boolean,
            eq::{ConditionalEqGadget, EqGadget},
            select::CondSelectGadget,
            uint::unsigned_integer::{UInt, UInt32, UInt8},
            ToBytesGadget,
        },
    },
};

use std::borrow::Borrow;

// 2.1.  Parameters
// The following table summarizes various parameters and their ranges:
//               | BLAKE2b          | BLAKE2s          |
// --------------+------------------+------------------+
// Bits in word  | w = 64           | w = 32           |
// Rounds in F   | r = 12           | r = 10           |
// Block bytes   | bb = 128         | bb = 64          |
// Hash bytes    | 1 <= nn <= 64    | 1 <= nn <= 32    |
// Key bytes     | 0 <= kk <= 64    | 0 <= kk <= 32    |
// Input bytes   | 0 <= ll < 2**128 | 0 <= ll < 2**64  |
// --------------+------------------+------------------+
// G Rotation    | (R1, R2, R3, R4) | (R1, R2, R3, R4) |
// constants =   | (32, 24, 16, 63) | (16, 12,  8,  7) |
// --------------+------------------+------------------+
//

const R1: usize = 16;
const R2: usize = 12;
const R3: usize = 8;
const R4: usize = 7;

// Round     |  0  1  2  3  4  5  6  7  8  9 10 11 12 13 14 15 |
// ----------+-------------------------------------------------+
// SIGMA[0]  |  0  1  2  3  4  5  6  7  8  9 10 11 12 13 14 15 |
// SIGMA[1]  | 14 10  4  8  9 15 13  6  1 12  0  2 11  7  5  3 |
// SIGMA[2]  | 11  8 12  0  5  2 15 13 10 14  3  6  7  1  9  4 |
// SIGMA[3]  |  7  9  3  1 13 12 11 14  2  6  5 10  4  0 15  8 |
// SIGMA[4]  |  9  0  5  7  2  4 10 15 14  1 11 12  6  8  3 13 |
// SIGMA[5]  |  2 12  6 10  0 11  8  3  4 13  7  5 15 14  1  9 |
// SIGMA[6]  | 12  5  1 15 14 13  4 10  0  7  6  3  9  2  8 11 |
// SIGMA[7]  | 13 11  7 14 12  1  3  9  5  0 15  4  8  6  2 10 |
// SIGMA[8]  |  6 15 14  9 11  3  0  8 12  2 13  7  1  4 10  5 |
// SIGMA[9]  | 10  2  8  4  7  6  1  5 15 11  9 14  3 12 13  0 |
// ----------+-------------------------------------------------+
//

const SIGMA: [[usize; 16]; 10] = [
    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
    [14, 10, 4, 8, 9, 15, 13, 6, 1, 12, 0, 2, 11, 7, 5, 3],
    [11, 8, 12, 0, 5, 2, 15, 13, 10, 14, 3, 6, 7, 1, 9, 4],
    [7, 9, 3, 1, 13, 12, 11, 14, 2, 6, 5, 10, 4, 0, 15, 8],
    [9, 0, 5, 7, 2, 4, 10, 15, 14, 1, 11, 12, 6, 8, 3, 13],
    [2, 12, 6, 10, 0, 11, 8, 3, 4, 13, 7, 5, 15, 14, 1, 9],
    [12, 5, 1, 15, 14, 13, 4, 10, 0, 7, 6, 3, 9, 2, 8, 11],
    [13, 11, 7, 14, 12, 1, 3, 9, 5, 0, 15, 4, 8, 6, 2, 10],
    [6, 15, 14, 9, 11, 3, 0, 8, 12, 2, 13, 7, 1, 4, 10, 5],
    [10, 2, 8, 4, 7, 6, 1, 5, 15, 11, 9, 14, 3, 12, 13, 0],
];

// 3.1.  Mixing Function G
// The G primitive function mixes two input words, "x" and "y", into
// four words indexed by "a", "b", "c", and "d" in the working vector
// v[0..15].  The full modified vector is returned.  The rotation
// constants (R1, R2, R3, R4) are given in Section 2.1.
// FUNCTION G( v[0..15], a, b, c, d, x, y )
// |
// |   v[a] := (v[a] + v[b] + x) mod 2**w
// |   v[d] := (v[d] ^ v[a]) >>> R1
// |   v[c] := (v[c] + v[d])     mod 2**w
// |   v[b] := (v[b] ^ v[c]) >>> R2
// |   v[a] := (v[a] + v[b] + y) mod 2**w
// |   v[d] := (v[d] ^ v[a]) >>> R3
// |   v[c] := (v[c] + v[d])     mod 2**w
// |   v[b] := (v[b] ^ v[c]) >>> R4
// |
// |   RETURN v[0..15]
// |
// END FUNCTION.
//

#[allow(clippy::many_single_char_names)]
#[allow(clippy::too_many_arguments)]
fn mixing_g<F: PrimeField, CS: ConstraintSystem<F>>(
    mut cs: CS,
    v: &mut [UInt32],
    a: usize,
    b: usize,
    c: usize,
    d: usize,
    x: &UInt32,
    y: &UInt32,
) -> Result<(), SynthesisError> {
    v[a] = UInt32::addmany(cs.ns(|| "mixing step 1"), &[v[a].clone(), v[b].clone(), x.clone()])?;
    v[d] = v[d].xor(cs.ns(|| "mixing step 2"), &v[a])?.rotr(R1);
    v[c] = UInt32::addmany(cs.ns(|| "mixing step 3"), &[v[c].clone(), v[d].clone()])?;
    v[b] = v[b].xor(cs.ns(|| "mixing step 4"), &v[c])?.rotr(R2);
    v[a] = UInt32::addmany(cs.ns(|| "mixing step 5"), &[v[a].clone(), v[b].clone(), y.clone()])?;
    v[d] = v[d].xor(cs.ns(|| "mixing step 6"), &v[a])?.rotr(R3);
    v[c] = UInt32::addmany(cs.ns(|| "mixing step 7"), &[v[c].clone(), v[d].clone()])?;
    v[b] = v[b].xor(cs.ns(|| "mixing step 8"), &v[c])?.rotr(R4);

    Ok(())
}

// 3.2.  Compression Function F
// Compression function F takes as an argument the state vector "h",
// message block vector "m" (last block is padded with zeros to full
// block size, if required), 2w-bit_gadget offset counter "t", and final block
// indicator flag "f".  Local vector v[0..15] is used in processing.  F
// returns a new state vector.  The number of rounds, "r", is 12 for
// BLAKE2b and 10 for BLAKE2s.  Rounds are numbered from 0 to r - 1.
// FUNCTION F( h[0..7], m[0..15], t, f )
// |
// |      // Initialize local work vector v[0..15]
// |      v[0..7] := h[0..7]              // First half from state.
// |      v[8..15] := IV[0..7]            // Second half from IV.
// |
// |      v[12] := v[12] ^ (t mod 2**w)   // Low word of the offset.
// |      v[13] := v[13] ^ (t >> w)       // High word.
// |
// |      IF f = TRUE THEN                // last block flag?
// |      |   v[14] := v[14] ^ 0xFF..FF   // Invert all bits.
// |      END IF.
// |
// |      // Cryptographic mixing
// |      FOR i = 0 TO r - 1 DO           // Ten or twelve rounds.
// |      |
// |      |   // Message word selection permutation for this round.
// |      |   s[0..15] := SIGMA[i mod 10][0..15]
// |      |
// |      |   v := G( v, 0, 4,  8, 12, m[s[ 0]], m[s[ 1]] )
// |      |   v := G( v, 1, 5,  9, 13, m[s[ 2]], m[s[ 3]] )
// |      |   v := G( v, 2, 6, 10, 14, m[s[ 4]], m[s[ 5]] )
// |      |   v := G( v, 3, 7, 11, 15, m[s[ 6]], m[s[ 7]] )
// |      |
// |      |   v := G( v, 0, 5, 10, 15, m[s[ 8]], m[s[ 9]] )
// |      |   v := G( v, 1, 6, 11, 12, m[s[10]], m[s[11]] )
// |      |   v := G( v, 2, 7,  8, 13, m[s[12]], m[s[13]] )
// |      |   v := G( v, 3, 4,  9, 14, m[s[14]], m[s[15]] )
// |      |
// |      END FOR
// |
// |      FOR i = 0 TO 7 DO               // XOR the two halves.
// |      |   h[i] := h[i] ^ v[i] ^ v[i + 8]
// |      END FOR.
// |
// |      RETURN h[0..7]                  // New state.
// |
// END FUNCTION.
//

#[allow(clippy::many_single_char_names)]
fn blake2s_compression<F: PrimeField, CS: ConstraintSystem<F>>(
    mut cs: CS,
    h: &mut [UInt32],
    m: &[UInt32],
    t: u64,
    f: bool,
) -> Result<(), SynthesisError> {
    assert_eq!(h.len(), 8);
    assert_eq!(m.len(), 16);

    // static const uint32_t blake2s_iv[8] =
    // {
    // 0x6A09E667, 0xBB67AE85, 0x3C6EF372, 0xA54FF53A,
    // 0x510E527F, 0x9B05688C, 0x1F83D9AB, 0x5BE0CD19
    // };
    //

    let mut v = Vec::with_capacity(16);
    v.extend_from_slice(h);
    v.push(UInt32::constant(0x6A09E667));
    v.push(UInt32::constant(0xBB67AE85));
    v.push(UInt32::constant(0x3C6EF372));
    v.push(UInt32::constant(0xA54FF53A));
    v.push(UInt32::constant(0x510E527F));
    v.push(UInt32::constant(0x9B05688C));
    v.push(UInt32::constant(0x1F83D9AB));
    v.push(UInt32::constant(0x5BE0CD19));

    assert_eq!(v.len(), 16);

    v[12] = v[12].xor(cs.ns(|| "first xor"), &UInt32::constant(t as u32))?;
    v[13] = v[13].xor(cs.ns(|| "second xor"), &UInt32::constant((t >> 32) as u32))?;

    if f {
        v[14] = v[14].xor(cs.ns(|| "third xor"), &UInt32::constant(u32::max_value()))?;
    }

    for i in 0..10 {
        let mut cs = cs.ns(|| format!("round {}", i));

        let s = SIGMA[i % 10];

        mixing_g(cs.ns(|| "mixing invocation 1"), &mut v, 0, 4, 8, 12, &m[s[0]], &m[s[1]])?;
        mixing_g(cs.ns(|| "mixing invocation 2"), &mut v, 1, 5, 9, 13, &m[s[2]], &m[s[3]])?;
        mixing_g(
            cs.ns(|| "mixing invocation 3"),
            &mut v,
            2,
            6,
            10,
            14,
            &m[s[4]],
            &m[s[5]],
        )?;
        mixing_g(
            cs.ns(|| "mixing invocation 4"),
            &mut v,
            3,
            7,
            11,
            15,
            &m[s[6]],
            &m[s[7]],
        )?;

        mixing_g(
            cs.ns(|| "mixing invocation 5"),
            &mut v,
            0,
            5,
            10,
            15,
            &m[s[8]],
            &m[s[9]],
        )?;
        mixing_g(
            cs.ns(|| "mixing invocation 6"),
            &mut v,
            1,
            6,
            11,
            12,
            &m[s[10]],
            &m[s[11]],
        )?;
        mixing_g(
            cs.ns(|| "mixing invocation 7"),
            &mut v,
            2,
            7,
            8,
            13,
            &m[s[12]],
            &m[s[13]],
        )?;
        mixing_g(
            cs.ns(|| "mixing invocation 8"),
            &mut v,
            3,
            4,
            9,
            14,
            &m[s[14]],
            &m[s[15]],
        )?;
    }

    for i in 0..8 {
        let mut cs = cs.ns(|| format!("h[{i}] ^ v[{i}] ^ v[{i} + 8]", i = i));

        h[i] = h[i].xor(cs.ns(|| "first xor"), &v[i])?;
        h[i] = h[i].xor(cs.ns(|| "second xor"), &v[i + 8])?;
    }

    Ok(())
}

// FUNCTION BLAKE2( d[0..dd-1], ll, kk, nn )
// |
// |     h[0..7] := IV[0..7]          // Initialization Vector.
// |
// |     // Parameter block p[0]
// |     h[0] := h[0] ^ 0x01010000 ^ (kk << 8) ^ nn
// |
// |     // Process padded key and data blocks
// |     IF dd > 1 THEN
// |     |       FOR i = 0 TO dd - 2 DO
// |     |       |       h := F( h, d[i], (i + 1) * bb, FALSE )
// |     |       END FOR.
// |     END IF.
// |
// |     // Final block.
// |     IF kk = 0 THEN
// |     |       h := F( h, d[dd - 1], ll, TRUE )
// |     ELSE
// |     |       h := F( h, d[dd - 1], ll + bb, TRUE )
// |     END IF.
// |
// |     RETURN first "nn" bytes from little-endian word array h[].
// |
// END FUNCTION.
//

pub fn blake2s_gadget<F: PrimeField, CS: ConstraintSystem<F>>(
    mut cs: CS,
    input: &[Boolean],
) -> Result<Vec<UInt32>, SynthesisError> {
    assert!(input.len() % 8 == 0);

    let mut h = Vec::with_capacity(8);
    h.push(UInt32::constant(0x6A09E667 ^ 0x01010000 ^ 32));
    h.push(UInt32::constant(0xBB67AE85));
    h.push(UInt32::constant(0x3C6EF372));
    h.push(UInt32::constant(0xA54FF53A));
    h.push(UInt32::constant(0x510E527F));
    h.push(UInt32::constant(0x9B05688C));
    h.push(UInt32::constant(0x1F83D9AB));
    h.push(UInt32::constant(0x5BE0CD19));

    let mut blocks: Vec<Vec<UInt32>> = vec![];

    for block in input.chunks(512) {
        let mut this_block = Vec::with_capacity(16);
        for word in block.chunks(32) {
            let mut tmp = word.to_vec();
            while tmp.len() < 32 {
                tmp.push(Boolean::constant(false));
            }
            this_block.push(UInt32::from_bits_le(&tmp));
        }
        while this_block.len() < 16 {
            this_block.push(UInt32::constant(0));
        }
        blocks.push(this_block);
    }

    if blocks.is_empty() {
        blocks.push((0..16).map(|_| UInt32::constant(0)).collect());
    }

    for (i, block) in blocks[0..blocks.len() - 1].iter().enumerate() {
        let cs = cs.ns(|| format!("block {}", i));

        blake2s_compression(cs, &mut h, block, ((i as u64) + 1) * 64, false)?;
    }

    {
        let cs = cs.ns(|| "final block");

        blake2s_compression(cs, &mut h, &blocks[blocks.len() - 1], (input.len() / 8) as u64, true)?;
    }

    Ok(h)
}

pub struct Blake2sGadget;

#[derive(Clone, Debug)]
pub struct Blake2sOutputGadget(pub Vec<UInt8>);

impl PartialEq for Blake2sOutputGadget {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for Blake2sOutputGadget {}

impl<F: PrimeField> EqGadget<F> for Blake2sOutputGadget {}

impl<F: PrimeField> ConditionalEqGadget<F> for Blake2sOutputGadget {
    #[inline]
    fn conditional_enforce_equal<CS: ConstraintSystem<F>>(
        &self,
        mut cs: CS,
        other: &Self,
        condition: &Boolean,
    ) -> Result<(), SynthesisError> {
        for (i, (a, b)) in self.0.iter().zip(other.0.iter()).enumerate() {
            a.conditional_enforce_equal(&mut cs.ns(|| format!("blake2s_equal_{}", i)), b, condition)?;
        }
        Ok(())
    }

    fn cost() -> usize {
        32 * <UInt8 as ConditionalEqGadget<F>>::cost()
    }
}

impl<F: PrimeField> CondSelectGadget<F> for Blake2sOutputGadget {
    fn conditionally_select<CS: ConstraintSystem<F>>(
        _cs: CS,
        _cond: &Boolean,
        _first: &Self,
        _second: &Self,
    ) -> Result<Self, SynthesisError> {
        unimplemented!()
    }

    fn cost() -> usize {
        unimplemented!()
    }
}

impl<F: PrimeField> ToBytesGadget<F> for Blake2sOutputGadget {
    #[inline]
    fn to_bytes<CS: ConstraintSystem<F>>(&self, _cs: CS) -> Result<Vec<UInt8>, SynthesisError> {
        Ok(self.0.clone())
    }

    #[inline]
    fn to_bytes_strict<CS: ConstraintSystem<F>>(&self, cs: CS) -> Result<Vec<UInt8>, SynthesisError> {
        self.to_bytes(cs)
    }
}

impl<F: PrimeField> AllocGadget<[u8; 32], F> for Blake2sOutputGadget {
    #[inline]
    fn alloc<Fn: FnOnce() -> Result<T, SynthesisError>, T: Borrow<[u8; 32]>, CS: ConstraintSystem<F>>(
        cs: CS,
        value_gen: Fn,
    ) -> Result<Self, SynthesisError> {
        Ok(Blake2sOutputGadget(<UInt8>::alloc_vec(cs, &match value_gen() {
            Ok(val) => *(val.borrow()),
            Err(_) => [0u8; 32],
        })?))
    }

    #[inline]
    fn alloc_input<Fn: FnOnce() -> Result<T, SynthesisError>, T: Borrow<[u8; 32]>, CS: ConstraintSystem<F>>(
        cs: CS,
        value_gen: Fn,
    ) -> Result<Self, SynthesisError> {
        Ok(Blake2sOutputGadget(<UInt8>::alloc_input_vec(cs, &match value_gen() {
            Ok(val) => *(val.borrow()),
            Err(_) => [0u8; 32],
        })?))
    }
}

impl<F: PrimeField> PRFGadget<Blake2s, F> for Blake2sGadget {
    type OutputGadget = Blake2sOutputGadget;

    fn new_seed<CS: ConstraintSystem<F>>(mut cs: CS, seed: &[u8; 32]) -> Vec<UInt8> {
        UInt8::alloc_vec(&mut cs.ns(|| "alloc_seed"), seed).unwrap()
    }

    fn check_evaluation_gadget<CS: ConstraintSystem<F>>(
        mut cs: CS,
        seed: &[UInt8],
        input: &[UInt8],
    ) -> Result<Self::OutputGadget, SynthesisError> {
        assert_eq!(seed.len(), 32);
        // assert_eq!(input.len(), 32);
        let mut gadget_input = vec![];
        for byte in seed.iter().chain(input) {
            gadget_input.extend_from_slice(&byte.to_bits_le());
        }
        let mut result = vec![];
        for (i, int) in blake2s_gadget(cs.ns(|| "blake2s_prf"), &gadget_input)?
            .into_iter()
            .enumerate()
        {
            let chunk = int.to_bytes(&mut cs.ns(|| format!("to_bytes_{}", i)))?;
            result.extend_from_slice(&chunk);
        }
        Ok(Blake2sOutputGadget(result))
    }
}
